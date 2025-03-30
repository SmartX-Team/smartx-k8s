use std::{collections::BTreeMap, iter, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use clap::Parser;
use convert_case::{Case, Casing};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::{Node, ObjectReference},
    apimachinery::pkg::apis::meta::v1::{OwnerReference, Time},
};
use kube::{
    Api, Client, Error, Resource, ResourceExt, Result,
    api::{
        DeleteParams, ListParams, ObjectMeta, Patch, PatchParams, PostParams, PropagationPolicy,
        ValidationDirective,
    },
    runtime::{
        Controller,
        controller::Action,
        events::{Event, EventType, Recorder, Reporter},
        reflector::ObjectRef,
        watcher::Config,
    },
};
use kube_custom_resources_rs::argoproj_io::v1alpha1::applications::{
    Application, ApplicationDestination, ApplicationIgnoreDifferences, ApplicationSources,
    ApplicationSourcesHelm, ApplicationSpec, ApplicationSyncPolicy, ApplicationSyncPolicyAutomated,
    ApplicationSyncPolicyManagedNamespaceMetadata,
};
use openark_core::operator::{OperatorArgs, install_crd};
use openark_vine_session_api::{
    NodeSession, ProfileState,
    binding::{SessionBindingCrd, SessionBindingSpec},
    owned_profile::{
        OwnedFeaturesSpec, OwnedOpenArkSpec, OwnedSessionProfileSpec, OwnedUserSpec,
        OwnedVMHostDeviceSpec, OwnedVMSpec,
    },
    profile::{SessionProfileCrd, SessionProfileSpec, VolumeSharingSpec},
};
#[cfg(feature = "tracing")]
use tracing::{Level, debug, error, info, instrument, warn};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    api: ::openark_vine_session_api::VineSessionArgs,

    #[arg(long, env = "CONTROLLER_POD_NAME")]
    controller_pod_name: Option<String>,

    #[arg(long, env = "INSTALL_CRDS")]
    install_crds: bool,

    #[arg(long, env = "OPENARK_LABEL_SELECTOR")]
    label_selector: String,

    #[arg(long, env = "NAMESPACE")]
    namespace: String,

    #[command(flatten)]
    operator: OperatorArgs,

    #[arg(long, env = "SESSION_NAMESPACE")]
    session_namespace: String,

    #[arg(long, env = "VOLUME_NAME_PUBLIC")]
    volume_name_public: Option<String>,

    #[arg(long, env = "VOLUME_NAME_STATIC")]
    volume_name_static: Option<String>,
}

#[derive(Clone)]
struct Context {
    api_app: Api<Application>,
    api_binding: Api<SessionBindingCrd>,
    api_node: Api<Node>,
    api_profile: Api<SessionProfileCrd>,
    args: Args,
    delete_params: DeleteParams,
    patch_params: PatchParams,
    post_params: PostParams,
    recorder: Recorder,
}

#[must_use]
fn is_selected(
    node: Option<&BTreeMap<String, String>>,
    selector: Option<&BTreeMap<String, String>>,
) -> bool {
    match (node, selector) {
        (Some(node), Some(selector)) => selector
            .iter()
            .all(|(key, value)| node.get(key) == Some(value)),
        (None, Some(selector)) => selector.is_empty(),
        (Some(_) | None, None) => true,
    }
}

#[must_use]
fn collect_node_host_devices(node: &Node) -> Vec<OwnedVMHostDeviceSpec> {
    node.status
        .as_ref()
        .and_then(|status| status.capacity.as_ref())
        .map(|capacity| {
            capacity
                .iter()
                .filter_map(|(key, value)| {
                    let items: Vec<_> = key.split('-').collect();
                    if items.len() != 4 || items[0].len() <= 4 {
                        return None;
                    }
                    let bus_type = &items[0][items[0].len() - 4..];
                    if !["/pci", "/usb"].contains(&bus_type) {
                        return None;
                    }
                    let device = OwnedVMHostDeviceSpec {
                        api_group: items[0].into(),
                        kind: items[1].to_case(Case::Pascal),
                        vendor: items[2].into(),
                        product: items[3].into(),
                    };
                    let count = value.0.parse().ok()?;
                    Some(iter::repeat_n(device, count))
                })
                .flatten()
                .collect()
        })
        .unwrap_or_default()
}

enum AppState {
    Created,
    Deleted,
    Deleting,
    NodeNotReady,
}

#[must_use]
fn build_owned_session_profile(
    ctx: &Context,
    node: &Node,
    session: &NodeSession,
    binding: &SessionBindingSpec,
    profile: &SessionProfileSpec,
) -> Result<OwnedSessionProfileSpec, AppState> {
    let SessionProfileSpec {
        features,
        greeter,
        persistence,
        services,
        session: session_spec,
        user,
        vm,
        volumes,
    } = profile.clone();

    let features = features.unwrap_or_default();
    let vm = vm.unwrap_or_default();

    let mut session_spec = session_spec.unwrap_or_default();
    {
        let limits = session_spec
            .resources
            .get_or_insert_default()
            .limits
            .get_or_insert_default();
        for (key, value) in session.to_resources_compute(node) {
            limits.entry(key).or_insert(value);
        }
    }

    let host_devices = if features.host_display.unwrap_or(false) && vm.enabled.unwrap_or(false) {
        let devices = collect_node_host_devices(node);
        if !devices
            .iter()
            .any(|device| device.api_group.ends_with("/pci") && device.kind == "UsbController")
        {
            {
                #[cfg(feature = "tracing")]
                warn!("No USB Controllers are ready: {}", node.name_any());
            }
            return Err(AppState::NodeNotReady);
        }
        Some(devices)
    } else {
        None
    };

    let mut volumes = volumes.unwrap_or_default();
    {
        // Provision local storage
        let capacity = volumes
            .local
            .get_or_insert_default()
            .capacity
            .get_or_insert_default();
        for (key, value) in session.to_resources_local_storage() {
            capacity.entry(key).or_insert(value);
        }

        fn attach_shared_volume(volume: &mut VolumeSharingSpec, default_pvc_name: Option<&str>) {
            if let Some(name) = default_pvc_name {
                if volume.enabled.unwrap_or(false)
                    && volume
                        .persistent_volume_claim
                        .as_ref()
                        .is_none_or(|claim| claim.claim_name.is_empty())
                {
                    let claim = volume.persistent_volume_claim.get_or_insert_default();
                    claim.claim_name = name.into();
                }
            }
        }

        // Provision shared storages
        attach_shared_volume(
            volumes.public.get_or_insert_default(),
            ctx.args.volume_name_public.as_deref(),
        );
        attach_shared_volume(
            volumes.r#static.get_or_insert_default(),
            ctx.args.volume_name_static.as_deref(),
        );
    }

    Ok(OwnedSessionProfileSpec {
        auth: ctx.args.api.to_openark_auth_spec(),
        features: OwnedFeaturesSpec {
            data: features,
            vm: ctx.args.api.feature_vm(),
        },
        greeter: greeter.unwrap_or_default(),
        ingress: ctx.args.api.to_openark_ingress_spec(),
        node: ctx.args.api.to_node_spec(node),
        openark: OwnedOpenArkSpec {
            labels: ctx.args.api.to_openark_labels(),
        },
        persistence: persistence.unwrap_or_default(),
        services: services.unwrap_or_default(),
        session: session_spec,
        user: OwnedUserSpec {
            binding: binding.user.clone(),
            data: user.unwrap_or_default(),
        },
        vm: OwnedVMSpec {
            data: vm,
            host_devices,
        },
        volumes,
    })
}

#[must_use]
fn build_owner_reference<K>(object: &K) -> OwnerReference
where
    K: Resource<DynamicType = ()> + ResourceExt,
{
    OwnerReference {
        api_version: K::api_version(&()).into(),
        block_owner_deletion: Some(true),
        controller: Some(false),
        kind: K::kind(&()).into(),
        name: object.name_any(),
        uid: object.uid().unwrap_or_default(),
    }
}

#[must_use]
fn build_app_name(node: &Node) -> String {
    format!("session-{}", node.name_any())
}

fn build_app(
    ctx: &Context,
    node: &Node,
    session: &NodeSession,
    binding: &SessionBindingCrd,
    profile: &SessionProfileCrd,
) -> Result<Result<Application, AppState>> {
    let name = build_app_name(node);
    let namespace = ctx.args.session_namespace.clone();
    let owned_spec =
        match build_owned_session_profile(ctx, node, session, &binding.spec, &profile.spec) {
            Ok(spec) => spec,
            Err(state) => return Ok(Err(state)),
        };

    Ok(Ok(Application {
        metadata: ObjectMeta {
            annotations: Some(profile.annotations().clone()),
            labels: Some({
                let labels = profile.labels().clone();
                session.append_labels(&ctx.args.api, labels)
            }),
            name: Some(name),
            namespace: Some(ctx.args.namespace.clone()),
            // The default behaviour is foreground cascading deletion
            // TODO: Can be changed: https://github.com/argoproj/argo-cd/issues/21035
            finalizers: Some(vec!["resources-finalizer.argocd.argoproj.io".into()]),
            owner_references: Some(vec![
                build_owner_reference(node),
                build_owner_reference(profile),
                build_owner_reference(binding),
            ]),
            ..Default::default()
        },
        // TODO: to be implemented
        spec: ApplicationSpec {
            destination: ApplicationDestination {
                name: Some("mobilex".into()),
                namespace: Some(namespace),
                server: None,
            },
            ignore_differences: Some(vec![ApplicationIgnoreDifferences {
                group: Some("kubevirt.io".into()),
                kind: "VirtualMachine".into(),
                name: None,
                json_pointers: Some(vec![
                    "/spec/template/spec/domain/resources/limits/memory".into(),
                ]),
                ..Default::default()
            }]),
            info: None,
            project: "mobilex-openark-vine-session".into(),
            revision_history_limit: None,
            source: None,
            sources: Some(vec![ApplicationSources {
                helm: Some(ApplicationSourcesHelm {
                    release_name: Some("session".into()), // FIXED
                    value_files: None,
                    values_object: Some(
                        ::serde_json::to_value(&owned_spec)
                            .and_then(::serde_json::from_value)
                            .map_err(Error::SerdeError)?,
                    ),
                    ..Default::default()
                }),
                path: Some(ctx.args.api.source_path().into()),
                repo_url: ctx.args.api.source_repo_url().to_string(),
                target_revision: Some(ctx.args.api.source_repo_revision().into()),
                ..Default::default()
            }]),
            sync_policy: Some(ApplicationSyncPolicy {
                automated: Some(ApplicationSyncPolicyAutomated {
                    allow_empty: None,
                    prune: Some(true),
                    self_heal: Some(true),
                }),
                managed_namespace_metadata: Some(ApplicationSyncPolicyManagedNamespaceMetadata {
                    annotations: None,
                    labels: Some({
                        let mut map = BTreeMap::default();
                        map.insert(
                            "pod-security.kubernetes.io/enforce".into(),
                            "privileged".into(),
                        );
                        map
                    }),
                }),
                retry: None,
                sync_options: Some(vec![
                    "CreateNamespace=true".into(),
                    "RespectIgnoreDifferences=true".into(),
                    "ServerSideApply=true".into(),
                ]),
            }),
        },
        status: None,
    }))
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, err(level = Level::ERROR), skip_all))]
async fn create_app(
    ctx: &Context,
    node: &Node,
    session: &NodeSession,
    binding: &SessionBindingCrd,
    profile: &SessionProfileCrd,
) -> Result<AppState> {
    let app = match build_app(ctx, node, session, binding, profile)? {
        Ok(app) => app,
        Err(state) => return Ok(state),
    };

    // TODO: use watcher+managedresources instead
    let name = app.name_any();
    if ctx.api_app.get_metadata_opt(&name).await?.is_none() {
        ctx.api_app.create(&ctx.post_params, &app).await?;
        #[cfg(feature = "tracing")]
        info!("created application/{name}");
    } else {
        #[cfg(feature = "tracing")]
        debug!("already created application/{name}");
    }
    Ok(AppState::Created)
}

/// Return `true` if the app has been deleted.
///
#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, err(level = Level::ERROR), skip_all))]
async fn delete_app(ctx: &Context, node: &Node) -> Result<AppState> {
    // TODO: use watcher+managedresources instead
    let name = build_app_name(node);
    match ctx.api_app.get_metadata_opt(&name).await? {
        Some(_) => {
            ctx.api_app.delete(&name, &ctx.delete_params).await?;
            {
                #[cfg(feature = "tracing")]
                info!("start deleting application/{name}");
            }
            Ok(AppState::Deleting)
        }
        None => {
            {
                #[cfg(feature = "tracing")]
                info!("deleted application/{name}");
            }
            Ok(AppState::Deleted)
        }
    }
}

/// Return `true` if the app has been synced.
///
#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, err(level = Level::ERROR), skip_all))]
async fn sync_app(ctx: &Context, node: &Node, timestamp: DateTime<Utc>) -> Result<bool> {
    // TODO: use watcher+managedresources instead
    let name = build_app_name(node);
    match ctx.api_app.get_opt(&name).await? {
        Some(app) => {
            match app.metadata.creation_timestamp {
                Some(Time(since)) => match app.metadata.deletion_timestamp {
                    // Wait until the app is deleted
                    Some(Time(since)) => {
                        let duration = timestamp - since;
                        if duration >= NodeSession::DURATION_SIGN_OUT {
                            #[cfg(feature = "tracing")]
                            warn!("still deleting application/{name} since {since:?} ({duration})");
                        } else {
                            #[cfg(feature = "tracing")]
                            info!("still deleting application/{name} since {since:?} ({duration})");
                        }
                        Ok(false)
                    }
                    // The app should be synced
                    None => match app
                        .status
                        .as_ref()
                        .and_then(|status| status.sync.as_ref())
                        .map(|sync| sync.status.as_str())
                    {
                        // The app is synced
                        Some("Synced") => {
                            {
                                #[cfg(feature = "tracing")]
                                debug!("synced application/{name}");
                            }
                            Ok(true)
                        }
                        // Wait until the app is synced (pre-install jobs are completed)
                        _ => {
                            let duration = timestamp - since;
                            if duration >= NodeSession::DURATION_SIGN_OUT {
                                #[cfg(feature = "tracing")]
                                warn!(
                                    "still syncing application/{name} since {since:?} ({duration})"
                                );
                            } else {
                                #[cfg(feature = "tracing")]
                                info!(
                                    "still syncing application/{name} since {since:?} ({duration})"
                                );
                            }
                            Ok(false)
                        }
                    },
                },
                // The app is not controlled yet
                None => Ok(false),
            }
        }
        // Nothing to sync
        None => Ok(true),
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(node: Arc<Node>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = node.name_any();

    // Check the node's current session
    let current = NodeSession::load(&ctx.args.api, &node);
    let timestamp = Utc::now();

    // Do nothing while signing out
    if let Some(remaining) = current
        .signing_out(&ctx.args.api, timestamp)
        .and_then(|delta| delta.to_std().ok())
    {
        {
            #[cfg(feature = "tracing")]
            info!("waiting for signing out: {name} for {remaining:?}");
        }
        return Ok(Action::requeue(remaining));
    }

    // Do nothing while syncing the session application
    let is_synced = sync_app(&ctx, &node, timestamp).await?;
    if !is_synced {
        return Ok(Action::requeue(NodeSession::DURATION_SIGN_OUT_STD));
    }

    // Clone the session and apply the static information
    let mut next = current.clone();
    next.apply_node(&node);

    // Try signing in with a new profile
    let next_profile = {
        // TODO: use pools (reflector?)
        let lp = ListParams::default();
        let bindings = ctx.api_binding.list(&lp).await?;
        let binding = bindings
            .items
            .into_iter()
            .filter(|binding| {
                is_selected(
                    node.metadata.labels.as_ref(),
                    binding.spec.node_selector.as_ref(),
                )
            })
            .min_by_key(|binding| {
                (
                    binding.spec.priority,
                    binding.metadata.creation_timestamp.clone(),
                    binding.metadata.uid.clone(),
                )
            });
        let profile = match &binding {
            Some(binding) => ctx.api_profile.get_opt(&binding.spec.profile).await?,
            None => None,
        };
        binding.zip(profile)
    };
    let profile_state = next.apply_profile(&ctx.args.api, next_profile.as_ref(), timestamp);

    // Sign out if the node is not ready
    let must_sign_out = profile_state.has_changed() || next.unreachable();
    let mut sign_out_remaining = next.set_sign_out(&ctx.args.api, timestamp, must_sign_out);

    // Update session application
    let app_state = match profile_state {
        ProfileState::Changed(_) => delete_app(&ctx, &node).await?,
        ProfileState::Created { binding, profile } => {
            create_app(&ctx, &node, &next, binding, profile).await?
        }
        ProfileState::Deleted(Some(_)) => delete_app(&ctx, &node).await?,
        ProfileState::Deleted(None) => AppState::Deleted,
        ProfileState::Unchanged { binding, profile } => {
            create_app(&ctx, &node, &next, binding, profile).await?
        }
    };

    // Remove the session revision to notify that the application is deleting
    let is_app_deleting = matches!(app_state, AppState::Deleting);
    if is_app_deleting {
        // Revert changes
        next = current.clone();
        sign_out_remaining = None;
        // Remove only the session revision
        next.remove_session_revision();
    }

    // Update if changed
    if current != next {
        // Apply patch
        let patch = Patch::Strategic(next.to_patch(&ctx.args.api));
        ctx.api_node.patch(&name, &ctx.patch_params, &patch).await?;
        {
            #[cfg(feature = "tracing")]
            info!("updated node/{name}: {patch:?}");
        }

        // Report message
        let message = if is_app_deleting {
            "Signing out".into()
        } else if must_sign_out {
            "Signed out".into()
        } else if let Some(user) = next.get_user() {
            format!("Signed in with {user}")
        } else {
            "Signed in".into()
        };
        let reference = ObjectRef::from_obj(&*node).into();
        report_update(&ctx.recorder, &reference, message).await?;
    }

    // Wait some seconds to apply signing out
    match sign_out_remaining.and_then(|delta| delta.to_std().ok()) {
        Some(remaining) => {
            {
                #[cfg(feature = "tracing")]
                info!("start waiting for signing out: {name} for {remaining:?}");
            }
            Ok(Action::requeue(remaining))
        }
        None => Ok(Action::requeue(NodeSession::DURATION_SIGN_OUT_STD)),
    }
}

async fn report_update(
    recorder: &Recorder,
    reference: &ObjectReference,
    message: String,
) -> Result<()> {
    let event = Event {
        type_: EventType::Normal,
        reason: "SessionUpdated".into(),
        note: Some(message),
        action: "Scheduling".into(),
        secondary: None,
    };
    recorder.publish(&event, reference).await
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    if let ::kube::runtime::controller::Error::ReconcilerFailed(error, object) = &error {
        // Shorten error notes
        let error = match error {
            Error::Api(error) => error.to_string(),
            error => error.to_string(),
        };
        let event = Event {
            type_: EventType::Warning,
            reason: "SessionError".into(),
            note: Some(error),
            action: "Scheduling".into(),
            secondary: None,
        };
        let reference = object.clone().into();
        recorder.publish(&event, &reference).await.ok();
    }
    #[cfg(feature = "tracing")]
    error!("reconcile failed: {error:?}")
}

fn error_policy(_node: Arc<Node>, _error: &Error, _ctx: Arc<Context>) -> Action {
    Action::requeue(Duration::from_secs(30))
}

async fn install_crds(args: &OperatorArgs, client: &Client) -> Result<()> {
    install_crd::<SessionBindingCrd>(args, client).await?;
    install_crd::<SessionProfileCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = Client::try_default().await?;

    // Update CRDs
    install_crds(&args.operator, &client).await?;

    let api_app = Api::namespaced(client.clone(), &args.namespace);
    let api_binding = Api::namespaced(client.clone(), &args.namespace);
    let api_node = Api::all(client.clone());
    let api_profile = Api::namespaced(client.clone(), &args.namespace);

    let delete_params = DeleteParams {
        dry_run: false,
        grace_period_seconds: None,
        propagation_policy: Some(PropagationPolicy::Foreground),
        preconditions: None,
    };
    let patch_params = PatchParams {
        dry_run: false,
        force: false,
        field_manager: Some(args.operator.controller_name.clone()),
        field_validation: Some(ValidationDirective::Strict),
    };
    let post_params = PostParams {
        dry_run: false,
        field_manager: Some(args.operator.controller_name.clone()),
    };

    let reporter = Reporter {
        controller: args.operator.controller_name.clone(),
        instance: args.controller_pod_name.clone(),
    };
    let recorder = Recorder::new(client, reporter);

    let watcher_config = Config {
        label_selector: Some(args.label_selector.clone()),
        ..Default::default()
    };

    let context = Arc::new(Context {
        api_app,
        api_binding,
        api_node: api_node.clone(),
        api_profile,
        args,
        delete_params,
        patch_params,
        post_params,
        recorder: recorder.clone(),
    });

    Controller::new(api_node.clone(), watcher_config)
        .run(reconcile, error_policy, context)
        .for_each(|res| async {
            match res {
                Ok((object, _)) => {
                    #[cfg(feature = "tracing")]
                    info!("reconciled {object:?}");
                    let _ = object;
                }
                Err(error) => report_error(&recorder, error).await,
            }
        })
        .await;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Session Operator!");

    try_main(args).await.expect("running an operator")
}
