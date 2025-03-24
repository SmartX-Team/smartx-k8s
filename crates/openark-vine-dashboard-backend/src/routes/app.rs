use std::collections::BTreeMap;

use actix_web::{HttpResponse, Responder, get, web::Data};
use kube::{
    Api, ResourceExt,
    api::{ListParams, PartialObjectMeta},
};
use openark_vine_dashboard_api::{
    app::{App, AppMetadata, AppSpec},
    catalog::{CatalogCategory, CatalogItem, CatalogItemCrd},
    page::{PageKind, PageMetadata, PageRef},
    table::TableCrd,
};
use openark_vine_oauth::{KubernetesClient, User};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::LabelArgs;

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("")]
pub async fn get(
    app: Data<AppMetadata>,
    labels: Data<LabelArgs>,
    user: KubernetesClient<Option<User>>,
) -> impl Responder {
    let catalog_items = {
        let api = Api::<CatalogItemCrd>::default_namespaced(user.client.clone());
        let lp = ListParams::default();

        match api.list(&lp).await {
            Ok(list) => list.items,
            Err(error) => {
                #[cfg(feature = "tracing")]
                warn!("Failed to list catalog items: {error}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    };

    let pages = {
        let api = Api::<TableCrd>::default_namespaced(user.client);
        let lp = ListParams::default();

        match api.list_metadata(&lp).await {
            Ok(list) => list
                .items
                .into_iter()
                .filter_map(|ref cr| convert_table_metadata(labels.as_ref(), cr))
                .collect(),
            Err(error) => {
                #[cfg(feature = "tracing")]
                warn!("Failed to list tables: {error}");
                return HttpResponse::InternalServerError().finish();
            }
        }
    };

    HttpResponse::Ok().json(App {
        metadata: app.get_ref().clone(),
        spec: AppSpec {
            catalog: convert_catalog(&labels, catalog_items),
            pages,
        },
    })
}

fn convert_catalog(labels: &LabelArgs, items: Vec<CatalogItemCrd>) -> Vec<CatalogCategory> {
    let mut categories = BTreeMap::default();
    for item in items {
        let category_name = item
            .labels()
            .get(&labels.label_category)
            .map(|name| name.as_str())
            .unwrap_or("default")
            .to_string();

        let category = categories
            .entry(category_name.clone())
            .or_insert_with(|| CatalogCategory {
                name: category_name,
                title: None,
                description: None,
                children: Default::default(),
            });

        let annotations = item.annotations();
        category.children.push(CatalogItem {
            name: item.name_any(),
            title: annotations.get(&labels.label_title).cloned(),
            description: annotations.get(&labels.label_description).cloned(),
            created_at: item.creation_timestamp().map(|time| time.0),
            updated_at: None,
            spec: item.spec,
        });
    }
    categories.into_values().collect()
}

fn convert_table_metadata(
    labels: &LabelArgs,
    cr: &PartialObjectMeta<TableCrd>,
) -> Option<PageMetadata> {
    let annotations = cr.annotations();

    Some(PageMetadata {
        object: PageRef {
            kind: PageKind::Table,
            namespace: cr.namespace()?,
            name: cr.name_any(),
        },
        title: annotations.get(&labels.label_title).cloned(),
        description: annotations.get(&labels.label_description).cloned(),
    })
}
