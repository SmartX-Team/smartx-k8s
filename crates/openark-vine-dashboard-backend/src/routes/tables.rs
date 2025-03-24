use actix_web::{HttpResponse, Responder, get, web::Path};
use kube::Api;
use openark_vine_dashboard_api::{
    page::PageSpec,
    table::{TableCrd, TableSession},
};
use openark_vine_oauth::{KubernetesClient, User};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("{namespace}/{name}")]
pub async fn get(
    path: Path<(String, String)>,
    user: KubernetesClient<Option<User>>,
) -> impl Responder {
    let (namespace, name) = path.into_inner();
    let api = Api::<TableCrd>::namespaced(user.client, &namespace);

    match api.get_opt(&name).await {
        Ok(Some(cr)) => HttpResponse::Ok().json(PageSpec::Table(TableSession {
            total_rows: None,
            spec: cr.spec,
        })),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("Failed to get table: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
