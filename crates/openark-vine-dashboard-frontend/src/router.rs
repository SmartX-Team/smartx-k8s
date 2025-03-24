use openark_vine_dashboard_api::page::{PageKind, PageRef};
use url::Url;
use web_sys::Location;
use yew::prelude::*;
use yew_router::Routable;

use crate::pages::{DynPage, Home, ItemForm};

#[derive(Clone, Routable, PartialEq, Eq)]
pub enum Route {
    #[at("/")]
    #[not_found]
    Home,
    #[at("/u/:page_kind/:namespace/:page_name")]
    Page {
        page_kind: PageKind,
        namespace: String,
        page_name: String,
    },
    #[at("/u/:page_kind/:namespace/:page_name/create")]
    PageItemCreate {
        page_kind: PageKind,
        namespace: String,
        page_name: String,
    },
    #[at("/u/:page_kind/:namespace/:page_name/i/:item_name/update")]
    PageItemUpdate {
        page_kind: PageKind,
        namespace: String,
        page_name: String,
        item_name: String,
    },
}

impl Route {
    #[inline]
    pub fn page(page_ref: PageRef) -> Self {
        let PageRef {
            kind: page_kind,
            namespace,
            name: page_name,
        } = page_ref;

        Self::Page {
            page_kind,
            namespace,
            page_name,
        }
    }

    #[inline]
    pub fn page_item_update(page_ref: PageRef, item_name: Option<String>) -> Self {
        let PageRef {
            kind: page_kind,
            namespace,
            name: page_name,
        } = page_ref;

        match item_name {
            Some(item_name) => Self::PageItemUpdate {
                page_kind,
                namespace,
                page_name,
                item_name,
            },
            None => Self::PageItemCreate {
                page_kind,
                namespace,
                page_name,
            },
        }
    }
}

pub fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! {
            <Home />
        },
        Route::Page {
            page_kind,
            namespace,
            page_name,
        } => {
            let page_ref = PageRef {
                kind: page_kind,
                namespace,
                name: page_name,
            };
            html! {
                <DynPage { page_ref } />
            }
        }
        Route::PageItemCreate {
            page_kind,
            namespace,
            page_name,
        } => {
            let page_ref = PageRef {
                kind: page_kind,
                namespace,
                name: page_name,
            };
            html! {
                <ItemForm { page_ref } />
            }
        }
        Route::PageItemUpdate {
            page_kind,
            namespace,
            page_name,
            item_name,
        } => {
            let page_ref = PageRef {
                kind: page_kind,
                namespace,
                name: page_name,
            };
            html! {
                <ItemForm { page_ref } { item_name } />
            }
        }
    }
}

fn location() -> Location {
    ::web_sys::window()
        .expect("failed to find the window; currently only CSR is supported")
        .location()
}

/// Return the current window location href.
///
pub fn href() -> Url {
    let href = location()
        .href()
        .expect("failed to get current location href");

    href.parse().expect("invalid location")
}

/// Redirect to the specific URL for now
pub fn redirect_to(url: &str) -> ! {
    location()
        .replace(url)
        .expect("failed to redirect the page");

    unreachable!("The window should always be redirected")
}
