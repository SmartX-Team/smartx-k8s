use actix_web::{HttpResponse, Responder, Scope, get, post, web};
use itertools::Itertools;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use kube::{Api, Client, ResourceExt, api::ListParams};
use kube_custom_resources_rs::argoproj_io::v1alpha1::applications::Application;
use kube_quantity::ParsedQuantity;
use openark_vine_oauth::User;
use openark_vine_session_api::{
    exec::ExecArgs,
    owned_profile::OwnedSessionProfileSpec,
    session::{
        Session, SessionLinks, SessionRegion, SessionResourceAnnotations, SessionResourceLabels,
        SessionState, SessionStatus, SessionStatusLevel, SessionUser,
    },
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::LabelArgs;

pub fn build() -> Scope {
    web::scope("bindings").service(list).service(exec)
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("")]
pub async fn list(
    labels: web::Data<LabelArgs>,
    // Add support for guest users
    kube: web::Data<Client>,
    user: User,
) -> impl Responder {
    let api = Api::<Application>::default_namespaced(kube.as_ref().clone());
    let lp = ListParams {
        label_selector: Some(build_label_selector(labels, None, &user)),
        ..Default::default()
    };

    match api.list(&lp).await {
        Ok(list) => {
            let mut items = list
                .items
                .into_iter()
                .filter_map(convert)
                .collect::<Vec<_>>();
            items.sort_by_key(|s| {
                (
                    s.user.name.clone(),
                    s.node.alias.clone(),
                    s.node.name.clone(),
                )
            });
            HttpResponse::Ok().json(items)
        }
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("Failed to list session bindings: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn convert(app: Application) -> Option<Session> {
    let values = app
        .spec
        .sources
        .as_ref()?
        .get(0)?
        .helm
        .as_ref()?
        .values_object
        .as_ref()?;

    let profile: OwnedSessionProfileSpec = ::serde_json::to_value(values)
        .and_then(::serde_json::from_value)
        .ok()?;

    let limits = profile
        .session
        .resources
        .as_ref()
        .and_then(|req| req.limits.as_ref());

    const GPU_PREFIX: &'static str = "nvidia.com/";
    let gpus = limits
        .map(|limits| {
            limits
                .keys()
                .filter(|&res| res.starts_with(GPU_PREFIX))
                .cloned()
                .collect_vec()
        })
        .unwrap_or_default();

    Some(Session {
        name: app.name_any(),
        region: SessionRegion {
            name: app.spec.destination.name.clone()?,
            title: Some(app.spec.destination.name?),
        },
        node: profile.node.clone(),
        user: profile
            .user
            .binding
            .name
            .map(|name| SessionUser { name })
            .unwrap_or_default(),
        // TODO: add groups support
        groups: vec![],
        snapshot: if profile.features.vm && profile.vm.enabled.unwrap_or(false) {
            // Windows Logo
            "https://upload.wikimedia.org/wikipedia/commons/8/87/Windows_logo_-_2021.svg"
                .parse()
                .ok()
        } else {
            // Ubuntu Logo
            "https://upload.wikimedia.org/wikipedia/commons/9/9e/UbuntuCoF.svg"
                .parse()
                .ok()
        },
        created_at: Some(app.metadata.creation_timestamp.map(|time| time.0)?),
        started_at: app
            .status
            .as_ref()
            .and_then(|status| status.operation_state.as_ref())
            .and_then(|state| state.finished_at.as_ref())
            .and_then(|time| time.parse().ok()),
        completed_at: None,
        resource_annotations: SessionResourceAnnotations {
            gpu: if gpus.is_empty() {
                None
            } else {
                Some(
                    gpus.iter()
                        .map(|res| &res[GPU_PREFIX.len()..])
                        .map(|res| {
                            if res == "gpu" {
                                "Generic NVIDIA GPU"
                            } else {
                                res
                            }
                        })
                        .join(", "),
                )
            },
        },
        resource_labels: SessionResourceLabels {
            cpu: limits
                .and_then(|limits| limits.get("cpu"))
                .and_then(convert_quantity),
            gpu: limits
                .and_then(|limits| sum_quantity(gpus.iter().filter_map(|gpu| limits.get(gpu)))),
            ram: limits
                .and_then(|limits| limits.get("memory"))
                .and_then(convert_quantity),
            storage: profile
                .volumes
                .local
                .as_ref()
                .and_then(|local| local.capacity.as_ref())
                .and_then(|requests| requests.get("storage"))
                .and_then(convert_quantity),
        },
        links: SessionLinks {
            notebook: if profile
                .services
                .notebook
                .as_ref()
                .and_then(|service| service.enabled)
                .unwrap_or(false)
            {
                format!(
                    "https://notebook.{node_name}.node.sessions.{domain_name}",
                    domain_name = &profile.ingress.domain_name,
                    node_name = &profile.node.name,
                )
                .parse()
                .ok()
            } else {
                None
            },
            rdp: if profile
                .services
                .rdp
                .as_ref()
                .and_then(|service| service.enabled)
                .unwrap_or(false)
            {
                format!(
                    "https://rdp.{node_name}.node.sessions.{domain_name}",
                    domain_name = &profile.ingress.domain_name,
                    node_name = &profile.node.name,
                )
                .parse()
                .ok()
            } else {
                None
            },
            vnc: if profile
                .services
                .novnc
                .as_ref()
                .and_then(|service| service.enabled)
                .unwrap_or(false)
            {
                format!(
                    "https://vnc.{node_name}.node.sessions.{domain_name}",
                    domain_name = &profile.ingress.domain_name,
                    node_name = &profile.node.name,
                )
                .parse()
                .ok()
            } else {
                None
            },
        },
        status: match app
            .status
            .as_ref()
            .and_then(|status| status.health.as_ref())
            .and_then(|health| health.status.as_deref())
        {
            Some("Healthy") => SessionStatus {
                level: Some(SessionStatusLevel::Info),
                state: Some(SessionState::Running),
            },
            Some(_) => SessionStatus {
                level: Some(SessionStatusLevel::Error),
                state: Some(SessionState::Unknown),
            },
            None => SessionStatus {
                level: Some(SessionStatusLevel::Info),
                state: Some(SessionState::Pending),
            },
        },
        events: vec![],
    })
}

fn convert_quantity(quantity: &Quantity) -> Option<Quantity> {
    ParsedQuantity::try_from(quantity).ok().map(Into::into)
}

fn sum_quantity<'a>(quantities: impl Iterator<Item = &'a Quantity>) -> Option<Quantity> {
    let parsed_quantities = quantities
        .filter_map(|quantity| ParsedQuantity::try_from(quantity).ok())
        .collect_vec();

    if parsed_quantities.is_empty() {
        None
    } else {
        let mut total = ParsedQuantity::default();
        for quantity in parsed_quantities {
            total += quantity;
        }
        Some(total.into())
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("exec")]
pub async fn exec(
    labels: web::Data<LabelArgs>,
    // Add support for guest users
    kube: web::Data<Client>,
    user: User,
    args: web::Json<ExecArgs>,
) -> impl Responder {
    let web::Json(mut args) = args;
    args.label_selector = Some(build_label_selector(
        labels,
        args.label_selector.as_deref(),
        &user,
    ));

    match ::openark_vine_session_exec::exec(kube.as_ref().clone(), &args).await {
        Ok(session) => {
            session.join().await;
            HttpResponse::Ok().json(())
        }
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("Failed to exec: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn build_label_selector(
    labels: web::Data<LabelArgs>,
    label_selector: Option<&str>,
    user: &User,
) -> String {
    let mut appended_label_selector = format!(
        "{k1}==true, {k2} in (, {v2})",
        k1 = &labels.label_bind,
        k2 = &labels.label_bind_user,
        v2 = user.username(),
    );

    match label_selector {
        Some(label_selector) => {
            appended_label_selector.push_str(label_selector);
            appended_label_selector
        }
        None => appended_label_selector,
    }
}
