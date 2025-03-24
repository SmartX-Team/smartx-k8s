use chrono::{Duration, Utc};
use convert_case::{Case, Casing};
use openark_vine_dashboard_api::catalog::{
    CatalogCategory, CatalogItem, CatalogItemSpec, CatalogItemType,
};
use yew::prelude::*;

use crate::{
    layouts::{Footer, Scaffold},
    stores::{app::use_app, client::use_api},
    unwrap_response,
};

fn build_catalog_item(item: &CatalogItem) -> Html {
    let CatalogItem {
        name,
        title,
        description,
        created_at,
        updated_at,
        spec:
            CatalogItemSpec {
                r#type,
                thumbnail_url,
                url,
            },
    } = item;

    let action = match r#type {
        CatalogItemType::Link => match url.as_ref() {
            Some(url) => Some(html! {
                <div class="card-actions justify-end">
                    <a class="btn btn-primary" href={ url.to_string() }>{ "Browse" }</a>
                </div>
            }),
            None => None,
        },
    };

    let now = Utc::now();
    let badge = if created_at.is_none_or(|timestamp| timestamp + Duration::days(30) > now) {
        Some(html! {
            <div class="badge badge-secondary">{ "NEW" }</div>
        })
    } else if updated_at.is_none_or(|timestamp| timestamp + Duration::days(30) > now) {
        Some(html! {
            <div class="badge badge-primary">{ "Updated" }</div>
        })
    } else {
        None
    };

    let body = html! {
        <div class="card-body">
            <h2 class="card-title">
                { title.clone().unwrap_or_else(|| name.to_case(Case::Title)) }
                { badge }
            </h2>
            { for description.as_ref().map(|text| html! { <p>{ text }</p> }) }
            { action }
        </div>
    };

    let body = match thumbnail_url {
        Some(url) => html! {
            <>
                <figure>
                    <img
                        src={ url.to_string() }
                        alt={ name.clone() }
                    />
                </figure>
                { body }
            </>
        },
        None => body,
    };

    html! {
        <div class="card bg-base-100 shadow-sm">
            { body }
        </div>
    }
}

fn build_catalog_category(category: &CatalogCategory) -> Html {
    let CatalogCategory {
        name,
        title,
        description,
        children,
    } = category;

    let items = children.iter().map(build_catalog_item);

    html! {
        <>
            <div class="divider divider-start">
                <h1>
                    { title.clone().unwrap_or_else(|| name.to_case(Case::Title)) }
                </h1>
                <h2>
                    { description.clone() }
                </h2>
            </div>
            <div class="grid grid-cols-4 gap-4">
                { for items }
            </div>
        </>
    }
}

fn build_catalog(categories: &[CatalogCategory]) -> Html {
    html! {
        <div class="flex flex-col pb-8 px-8 w-full">
            { for categories.iter().map(build_catalog_category) }
        </div>
    }
}

#[function_component(Home)]
pub fn component() -> Html {
    let api = use_api();
    let app = use_app(api.clone());
    let app = unwrap_response!(api, app);

    let body = build_catalog(&app.spec.catalog);

    html! {
        <Scaffold { app }>
            { body }
            <Footer />
        </Scaffold>
    }
}
