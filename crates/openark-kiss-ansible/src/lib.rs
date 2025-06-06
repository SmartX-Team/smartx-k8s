pub mod cluster;
mod config;
pub mod job;

use anyhow::Result;
use convert_case::{Case, Casing};
use k8s_openapi::{
    api::{
        batch::v1::{CronJob, CronJobSpec, Job, JobSpec, JobTemplateSpec},
        core::v1::{
            ConfigMapKeySelector, ConfigMapVolumeSource, Container, EnvVar, EnvVarSource,
            KeyToPath, PodDNSConfig, PodSpec, PodTemplateSpec, ResourceRequirements,
            SecretKeySelector, SecretVolumeSource, Toleration, Volume, VolumeMount,
        },
    },
    apimachinery::pkg::api::resource::Quantity,
};
use kube::{
    Api, Client, Error,
    api::{DeleteParams, ListParams, PostParams},
    core::ObjectMeta,
};
use openark_kiss_api::r#box::{BoxCrd, BoxGroupRole, BoxGroupSpec, BoxPowerType, BoxState};
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

pub struct AnsibleClient {
    client: Client,
    pub kiss: self::config::KissConfig,
    namespace: String,
}

impl AnsibleClient {
    pub const LABEL_BOX_NAME: &'static str = "kiss.ulagbulag.io/box_name";
    pub const LABEL_BOX_MACHINE_UUID: &'static str = "kiss.ulagbulag.io/box_machine_uuid";
    pub const LABEL_COMPLETED_STATE: &'static str = "kiss.ulagbulag.io/completed_state";
    pub const LABEL_JOB_NAME: &'static str = "kiss.ulagbulag.io/job_name";
    pub const LABEL_JOB_IS_CRITICAL: &'static str = "kiss.ulagbulag.io/is_critical";
    pub const LABEL_VERIFY_BIND_GROUP: &'static str = "kiss.ulagbulag.io/verify-bind-group";

    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip(kube), err(Display)))]
    pub async fn try_new(kube: &Client, namespace: &str) -> Result<Self> {
        Ok(Self {
            client: kube.clone(),
            kiss: self::config::KissConfig::try_default(kube, namespace).await?,
            namespace: namespace.into(),
        })
    }

    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip(self, job), err(Display)))]
    pub async fn spawn(&self, job: AnsibleJob<'_>) -> Result<bool, Error> {
        let box_name = job.r#box.spec.machine.uuid.to_string();
        let box_status = job.r#box.status.as_ref();
        let name = format!("box-{}-{}", &job.task, &box_name);

        let verify_bind_group = job
            .r#box
            .metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get(Self::LABEL_VERIFY_BIND_GROUP))
            .and_then(|value| value.parse().ok())
            .unwrap_or(true);

        let bind_group = job
            .r#box
            .status
            .as_ref()
            .and_then(|status| status.bind_group.as_ref());
        let group = &job.r#box.spec.group;
        let reset = self.kiss.group_force_reset || verify_bind_group && bind_group != Some(group);

        let priority_class_name = match group.role {
            BoxGroupRole::ControlPlane => "system-cluster-critical",
            _ => "k8s-cluster-critical",
        };

        {
            let dp = DeleteParams::background();
            let lp = ListParams {
                label_selector: Some(format!(
                    "{}={box_name},{}!=true",
                    AnsibleClient::LABEL_BOX_NAME,
                    AnsibleClient::LABEL_JOB_IS_CRITICAL,
                )),
                ..Default::default()
            };

            // delete all previous cronjobs
            {
                let api = Api::<CronJob>::namespaced(self.client.clone(), &self.namespace);
                api.delete_collection(&dp, &lp).await?;
            }
            // delete all previous jobs
            {
                let api = Api::<Job>::namespaced(self.client.clone(), &self.namespace);
                api.delete_collection(&dp, &lp).await?;
            }
        }

        // realize mutual exclusivity (QUEUE)
        let cluster_state = self::cluster::ClusterState::load(
            &self.client,
            &self.kiss,
            &job.r#box.spec,
            job.use_workers,
        )
        .await?;
        if let Some(new_state) = job.new_state {
            if matches!(new_state, BoxState::Joining) && !cluster_state.is_joinable() {
                #[cfg(feature = "tracing")]
                info!(
                    "Cluster is not ready: {} {} {} -> {}",
                    new_state,
                    job.r#box.spec.group.role,
                    &box_name,
                    &job.r#box.spec.group.cluster_name,
                );
                return Ok(false);
            }
        }

        // define the object
        let metadata = ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(self.namespace.clone()),
            labels: Some(
                vec![
                    Some((Self::LABEL_BOX_NAME.into(), box_name.clone())),
                    Some((
                        Self::LABEL_BOX_MACHINE_UUID.into(),
                        job.r#box.spec.machine.uuid.to_string(),
                    )),
                    Some((
                        Self::LABEL_JOB_IS_CRITICAL.into(),
                        job.is_critical.to_string(),
                    )),
                    Some((Self::LABEL_JOB_NAME.into(), job.task.into())),
                    Some(("serviceType".into(), "ansible-task".to_string())),
                    job.new_state
                        .and_then(|state| state.complete())
                        .as_ref()
                        .map(ToString::to_string)
                        .map(|state| (Self::LABEL_COMPLETED_STATE.into(), state)),
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            ..Default::default()
        };
        let spec = JobSpec {
            ttl_seconds_after_finished: Some(0),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: metadata.labels.clone(),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    affinity: Some(crate::job::affinity()),
                    dns_config: Some(PodDNSConfig {
                        nameservers: Some(vec![
                            self.kiss.bootstrapper_network_dns_server_ns1.to_string(),
                            self.kiss.bootstrapper_network_dns_server_ns2.to_string(),
                        ]),
                        ..Default::default()
                    }),
                    host_network: Some(true),
                    priority_class_name: Some(priority_class_name.into()),
                    restart_policy: Some("OnFailure".into()),
                    service_account: Some("ansible-playbook".into()),
                    tolerations: if job.is_critical {
                        Some(vec![
                            Toleration {
                                operator: Some("Exists".into()),
                                effect: Some("NoExecute".into()),
                                ..Default::default()
                            },
                            Toleration {
                                operator: Some("Exists".into()),
                                effect: Some("NoSchedule".into()),
                                ..Default::default()
                            },
                        ])
                    } else {
                        None
                    },
                    containers: vec![Container {
                        name: "ansible".into(),
                        image: Some(self.kiss.kubespray_image.clone()),
                        image_pull_policy: Some("Always".into()),
                        command: Some(vec!["ansible-playbook".into()]),
                        args: Some(vec![
                            "--become".into(),
                            "--become-user=root".into(),
                            "--inventory".into(),
                            "/root/ansible/defaults/defaults.yaml".into(),
                            "--inventory".into(),
                            "/root/ansible/defaults/all.yaml".into(),
                            "--inventory".into(),
                            "/root/ansible/config.yaml".into(),
                            "--inventory".into(),
                            "/root/ansible/hosts.yaml".into(),
                            format!("/opt/playbook/{}", group.role.to_playbook()),
                        ]),
                        env: Some(vec![
                            EnvVar {
                                name: "ansible_host".into(),
                                value: Some(job.r#box.spec.machine.hostname()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "ansible_host_id".into(),
                                value: Some(box_name.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "ansible_host_uuid".into(),
                                value: Some(job.r#box.spec.machine.uuid.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "ansible_ssh_host".into(),
                                value: box_status
                                    .and_then(|status| status.access.management())
                                    .map(|interface| interface.address.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "ansible_ssh_private_key_file".into(),
                                value: Some("/root/.ssh/id_ed25519".into()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "ansible_user".into(),
                                value_from: Some(EnvVarSource {
                                    config_map_key_ref: Some(ConfigMapKeySelector {
                                        name: "kiss-config".into(),
                                        key: "auth_ssh_username".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_allow_critical_commands".into(),
                                value: Some(self.kiss.allow_critical_commands.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_allow_pruning_network_interfaces".into(),
                                value: Some(self.kiss.allow_pruning_network_interfaces.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_ansible_task_name".into(),
                                value: Some(job.task.into()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_control_planes".into(),
                                value: Some(
                                    if matches!(job.new_state, None | Some(BoxState::Joining)) {
                                        cluster_state.get_control_planes_as_string()
                                    } else {
                                        Default::default()
                                    },
                                ),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_etcd_nodes".into(),
                                value: Some(
                                    if matches!(job.new_state, None | Some(BoxState::Joining)) {
                                        cluster_state.get_etcd_nodes_as_string()
                                    } else {
                                        Default::default()
                                    },
                                ),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_name".into(),
                                value: Some(group.cluster_name.clone()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_name_snake_case".into(),
                                value: Some(group.cluster_name.to_case(Case::Snake)),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_domain".into(),
                                value: Some(group.cluster_domain()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_is_default".into(),
                                value: Some(group.is_default().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_is_new".into(),
                                value: Some(cluster_state.is_new().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_cluster_worker_nodes".into(),
                                value: Some(if job.use_workers {
                                    cluster_state.get_worker_nodes_as_string()
                                } else {
                                    Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_enable_default_cluster".into(),
                                value: Some(self.kiss.group_enable_default_cluster.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_force_reset".into(),
                                value: Some(reset.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_force_reset_os".into(),
                                value: Some(self.kiss.group_force_reset_os.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_reset_storage".into(),
                                value: Some(self.kiss.group_reset_storage.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_role".into(),
                                value: Some(group.role.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_role_is_domain_specific".into(),
                                value: Some(group.role.is_domain_specific().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_group_role_is_member".into(),
                                value: Some(group.role.is_member().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_interface_mtu_size".into(),
                                value: Some(self.kiss.network_interface_mtu_size.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_dhcp_duration".into(),
                                value: Some(self.kiss.network_ipv4_dhcp_duration.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_dhcp_range_begin".into(),
                                value: Some(self.kiss.network_ipv4_dhcp_range_begin.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_dhcp_range_end".into(),
                                value: Some(self.kiss.network_ipv4_dhcp_range_end.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_gateway".into(),
                                value: Some(self.kiss.network_ipv4_gateway.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_subnet".into(),
                                value: Some(self.kiss.network_ipv4_subnet.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_subnet_address".into(),
                                value: Some(self.kiss.network_ipv4_subnet.network().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_subnet_mask".into(),
                                value: Some(self.kiss.network_ipv4_subnet.netmask().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_ipv4_subnet_mask_prefix".into(),
                                value: Some(self.kiss.network_ipv4_subnet.prefix_len().to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_nameserver_incluster_ipv4".into(),
                                value: Some(
                                    self.kiss.network_nameserver_incluster_ipv4.to_string(),
                                ),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_wireless_wifi_key_mgmt".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "network_wireless_wifi_key_mgmt".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_wireless_wifi_key_psk".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "network_wireless_wifi_key_psk".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_network_wireless_wifi_ssid".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "network_wireless_wifi_ssid".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_os_dist".into(),
                                value: Some(self.kiss.os_dist.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_os_kernel".into(),
                                value: Some(self.kiss.os_kernel.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_os_version".into(),
                                value: Some(self.kiss.os_version.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_intel_amt_host".into(),
                                value: job
                                    .r#box
                                    .spec
                                    .power
                                    .as_ref()
                                    .filter(|power| matches!(power.r#type, BoxPowerType::IntelAMT))
                                    .and_then(|power| power.address.as_ref())
                                    .map(|address| address.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_intel_amt_username".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "power_intel_amt_username".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_intel_amt_password".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "power_intel_amt_password".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_ipmi_host".into(),
                                value: job
                                    .r#box
                                    .spec
                                    .power
                                    .as_ref()
                                    .filter(|power| matches!(power.r#type, BoxPowerType::Ipmi))
                                    .and_then(|power| power.address.as_ref())
                                    .map(|address| address.to_string()),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_ipmi_username".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "power_ipmi_username".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_power_ipmi_password".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "power_ipmi_password".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            EnvVar {
                                name: "kiss_submit_base_url".into(),
                                value_from: Some(EnvVarSource {
                                    secret_key_ref: Some(SecretKeySelector {
                                        name: "kiss-config".into(),
                                        key: "submit_base_url".into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ]),
                        resources: Some(job.resource_type.into()),
                        volume_mounts: Some(vec![
                            VolumeMount {
                                name: "ansible".into(),
                                mount_path: "/root/ansible".into(),
                                ..Default::default()
                            },
                            VolumeMount {
                                name: "ansible-defaults".into(),
                                mount_path: "/root/ansible/defaults".into(),
                                ..Default::default()
                            },
                            VolumeMount {
                                name: "playbook".into(),
                                mount_path: "/opt/playbook".into(),
                                ..Default::default()
                            },
                            VolumeMount {
                                name: "tasks".into(),
                                mount_path: "/opt/playbook/tasks".into(),
                                ..Default::default()
                            },
                            VolumeMount {
                                name: "ssh".into(),
                                mount_path: "/root/.ssh".into(),
                                ..Default::default()
                            },
                        ]),
                        ..Default::default()
                    }],
                    volumes: Some(vec![
                        Volume {
                            name: "ansible".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: format!("ansible-control-planes-{}", &group.cluster_name,),
                                default_mode: Some(0o400),
                                optional: Some(!self.kiss.group_enforce_ansible_control_planes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Volume {
                            name: "ansible-defaults".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: "ansible-control-planes-default".into(),
                                default_mode: Some(0o400),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Volume {
                            name: "playbook".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: "ansible-task-common".into(),
                                default_mode: Some(0o400),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Volume {
                            name: "tasks".into(),
                            config_map: Some(ConfigMapVolumeSource {
                                name: format!("ansible-task-{}", &job.task),
                                default_mode: Some(0o400),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Volume {
                            name: "ssh".into(),
                            secret: Some(SecretVolumeSource {
                                secret_name: Some("kiss-config".into()),
                                default_mode: Some(0o400),
                                items: Some(vec![KeyToPath {
                                    key: "auth_ssh_key_id_ed25519".into(),
                                    path: "id_ed25519".into(),
                                    ..Default::default()
                                }]),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                }),
            },
            ..Default::default()
        };
        let pp = PostParams {
            dry_run: false,
            field_manager: Some("kiss-ansible".into()),
        };

        match job.cron {
            Some(schedule) => {
                let api = Api::<CronJob>::namespaced(self.client.clone(), &self.namespace);
                let job = CronJob {
                    metadata: metadata.clone(),
                    spec: Some(CronJobSpec {
                        concurrency_policy: Some("Replace".into()),
                        schedule: schedule.into(),
                        starting_deadline_seconds: Some(180 /* 3m */),
                        job_template: JobTemplateSpec {
                            metadata: Some(metadata),
                            spec: Some(spec),
                        },
                        ..Default::default()
                    }),
                    status: None,
                };
                api.create(&pp, &job).await?;
            }
            None => {
                let api = Api::<Job>::namespaced(self.client.clone(), &self.namespace);
                let job = Job {
                    metadata,
                    spec: Some(spec),
                    status: None,
                };
                api.create(&pp, &job).await?;
            }
        }

        #[cfg(feature = "tracing")]
        info!("spawned a job: {name}");
        Ok(true)
    }
}

pub struct AnsibleJob<'a> {
    pub cron: Option<&'static str>,
    pub task: &'static str,
    pub r#box: &'a BoxCrd,
    pub new_group: Option<&'a BoxGroupSpec>,
    pub new_state: Option<BoxState>,
    pub is_critical: bool,
    pub resource_type: AnsibleResourceType,
    pub use_workers: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum AnsibleResourceType {
    Minimal,
    #[default]
    Normal,
}

impl From<AnsibleResourceType> for ResourceRequirements {
    fn from(value: AnsibleResourceType) -> Self {
        match value {
            AnsibleResourceType::Minimal => Self {
                claims: None,
                requests: None,
                limits: Some(
                    vec![
                        ("cpu".into(), Quantity("100m".into())),
                        ("memory".into(), Quantity("500Mi".into())),
                    ]
                    .into_iter()
                    .collect(),
                ),
            },
            AnsibleResourceType::Normal => Self {
                claims: None,
                requests: None,
                limits: Some(
                    vec![
                        ("cpu".into(), Quantity("900m".into())),
                        ("memory".into(), Quantity("2Gi".into())),
                    ]
                    .into_iter()
                    .collect(),
                ),
            },
        }
    }
}
