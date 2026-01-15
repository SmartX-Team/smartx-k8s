use std::rc::Rc;

use openark_vine_browser_api::global::GlobalConfiguration;
use url::Url;
use yew::{Html, UseStateHandle, html};
use yew_router::{Routable, prelude::Redirect};

use crate::{i18n::DynI18n, pages::DirectoryPage};

#[derive(Clone, Debug, Routable, PartialEq, Eq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/*path")]
    FileEntry { path: String },
    #[at("/e/404")]
    #[not_found]
    NotFound,
}

#[derive(Clone, Debug)]
pub struct RouteProps {
    pub conf: Rc<GlobalConfiguration>,
    pub drawer_id: String,
    pub i18n: UseStateHandle<DynI18n>,
}

impl PartialEq for RouteProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.conf, &other.conf)
            && self.drawer_id == other.drawer_id
            && self.i18n == other.i18n
    }
}

impl Eq for RouteProps {}

pub fn switch(routes: Route, props: RouteProps) -> Html {
    #[cfg(feature = "tracing")]
    ::tracing::info!("Route = {routes:?}");

    match routes {
        Route::NotFound => html! {
            <Redirect<Route> to={ Route::FileEntry {
                path: Default::default(),
            } } />
        },
        Route::Home => {
            let path = "";
            html! {
                <DirectoryPage { path } route={ props } />
            }
        }
        Route::FileEntry { path } => html! {
            <DirectoryPage { path } route={ props } />
        },
    }
}

fn location() -> ::web_sys::Location {
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
