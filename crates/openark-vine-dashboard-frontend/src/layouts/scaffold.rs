use std::rc::Rc;

use convert_case::{Case, Casing};
use openark_vine_dashboard_api::{
    app::{App, AppMetadata, AppSpec},
    page::{PageMetadata, PageRef},
};
use tracing::Level;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

use crate::{
    router::Route,
    stores::client::{Alert, ClientStore},
    widgets::{Dialog, build_dialog},
};

fn build_button_kind(activated: bool) -> &'static str {
    if activated {
        "btn-secondary"
    } else {
        "btn-ghost"
    }
}

fn build_item(current_page: Option<&PageRef>, page: &PageMetadata) -> Html {
    let PageMetadata {
        object,
        title,
        description,
    } = page;

    let button_kind = build_button_kind(current_page == Some(object));

    let to = Route::page(object.clone());
    let body = html! {
        <Link<Route>
            classes={ Classes::from(format!("btn {button_kind} btn-lg")) }
            { to }
        >{
            title.clone().unwrap_or_else(|| object.name.to_case(Case::Title))
        }</Link<Route>>
    };

    match description {
        Some(description) => html! {
            <li class="tooltip tooltip-bottom" data-tip={ description.clone() }>{ body }</li>
        },
        None => html! {
            <li>{ body }</li>
        },
    }
}

fn build_alert(dispatch: &Dispatch<ClientStore>, index: usize, alert: &Rc<Alert>) -> Html {
    let Alert {
        back: _,
        level,
        message,
    } = &**alert;

    let onclick = {
        let dispatch = dispatch.clone();
        let alert = alert.clone();
        move |_| dispatch.reduce_mut(|store| store.dismiss_alert(index, &alert))
    };

    let class = match *level {
        Level::TRACE | Level::DEBUG => "",
        Level::INFO => "alert-info",
        Level::WARN => "alert-warning",
        Level::ERROR => "alert-error",
    };
    let symbol = match *level {
        Level::TRACE | Level::DEBUG => html! {
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-info h-6 w-6 shrink-0">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        },
        Level::INFO => html! {
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="h-6 w-6 shrink-0 stroke-current">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        },
        Level::WARN => html! {
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 shrink-0 stroke-current" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
        },
        Level::ERROR => html! {
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 shrink-0 stroke-current" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        },
    };

    html! {
        <li class="m-2">
            <div role="alert" class={ format!("alert {class}") } { onclick }>
                { symbol }
                <span>{ message }</span>
            </div>
        </li>
    }
}

fn build_alerts(store: Rc<ClientStore>, dispatch: Dispatch<ClientStore>) -> Html {
    let alerts = store
        .alerts()
        .iter()
        .enumerate()
        .map(|(index, alert)| build_alert(&dispatch, index, alert));

    html! {
        <ul class="bg-base-100 min-w-screen shadow-sm">
            { for alerts }
        </ul>
    }
}

#[derive(Clone, Debug, Properties)]
pub struct ScaffoldProps {
    pub app: Rc<App>,

    pub children: Html,

    #[prop_or_default]
    pub dialog: Option<UseReducerHandle<Dialog>>,

    #[prop_or_default]
    pub page_ref: Option<PageRef>,
}

impl PartialEq for ScaffoldProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.app, &other.app)
            && self.children == other.children
            && self.dialog == other.dialog
            && self.page_ref == other.page_ref
    }
}

#[function_component(Scaffold)]
pub fn component(props: &ScaffoldProps) -> Html {
    const DRAWER_IDDRAWER_ID: &'static str = "app-drawer";

    let ScaffoldProps {
        app,
        children,
        dialog,
        page_ref,
    } = props;

    let App {
        metadata:
            AppMetadata {
                name,
                title,
                description: _,
            },
        spec: AppSpec { catalog: _, pages },
    } = &**app;

    let (store, dispatch) = use_store::<ClientStore>();

    let is_home = page_ref.is_none();
    let page = page_ref
        .as_ref()
        .and_then(|object| pages.iter().find(|&page| page.object == *object));
    // let description = page.and_then(|page| page.description.clone());

    html! {
        <div class="min-h-screen">
            <div class="drawer min-h-screen">
                <input id={ DRAWER_IDDRAWER_ID } type="checkbox" class="drawer-toggle" />
                <div class="drawer-content flex flex-col">
                    // Navbar
                    <div class="navbar w-full bg-gradient-to-r from-neutral-100 to-neutral-300">
                        <div class="flex-none lg:hidden">
                            <label
                                for={ DRAWER_IDDRAWER_ID } aria-label="open sidebar"
                                class={ format!("btn btn-square {}", build_button_kind(is_home)) }
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    class="inline-block h-6 w-6 stroke-current"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M4 6h16M4 12h16M4 18h16"
                                    />
                                </svg>
                            </label>
                        </div>
                        // Navbar Title
                        <div class="mx-2 flex-1 px-2">
                            <button class="btn btn-ghost">
                                <Link<Route> to={ Route::Home } >
                                    <h1 class="text-xl font-bold text-blue-600">{
                                        title.clone().unwrap_or_else(|| name.to_case(Case::Title))
                                    }</h1>
                                </Link<Route>>
                            </button>
                            // <span class="align-middle select-none">{ for description }</span>
                        </div>
                        <div class="hidden flex-none lg:block">
                            <ul class="menu menu-horizontal">
                                // Navbar menu content here
                                { for pages.iter().map(|page| build_item(page_ref.as_ref(), page)) }
                            </ul>
                        </div>
                    </div>
                    // Alerts here
                    { build_alerts(store, dispatch) }
                    // Page content here
                    <div class="mx-8">
                        { children.clone() }
                    </div>
                </div>
                <div class="drawer-side">
                    <label for={ DRAWER_IDDRAWER_ID } aria-label="close sidebar" class="drawer-overlay"></label>
                    <ul class="menu bg-base-200 min-h-full w-80 p-4">
                        // Sidebar content here
                        <li class="text-center">{ "Hello, Guest!" }</li>
                        <div class="divider" />
                        { for pages.iter().map(|page| build_item(page_ref.as_ref(), page)) }
                    </ul>
                </div>
            </div>
            // Dialog here
            { for dialog.as_ref().and_then(build_dialog) }
        </div>
    }
}
