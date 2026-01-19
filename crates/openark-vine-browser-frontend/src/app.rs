use std::rc::Rc;

use openark_vine_browser_api::{
    client::ClientExt,
    global::{GlobalConfiguration, GlobalConfigurationSpec},
};
use yew::{Html, function_component, html, use_state_eq};
use yew_router::{BrowserRouter, Switch};

use crate::{
    i18n::DynI18n,
    net::{Client, HttpState, UseHttpHandleOption, UseHttpHandleOptionRender},
    router::{Route, RouteProps, switch},
};

/// Parses the given [`HttpState`].
///
fn parse_conf(state: HttpState<GlobalConfiguration>) -> Rc<GlobalConfiguration> {
    /// Generates an offline configuration.
    ///
    fn build_offline() -> GlobalConfiguration {
        GlobalConfiguration {
            spec: GlobalConfigurationSpec {
                title: Default::default(),
                logo_url: None,
                redirect_url: None,
            },
            user: None,
        }
    }

    thread_local! {
        /// Samples for building skeletons.
        ///
        static OFFLINE: Rc<GlobalConfiguration> = Rc::new(build_offline());
    }

    match state {
        HttpState::Pending | HttpState::NotFound | HttpState::Failed => OFFLINE.with(|d| d.clone()),
        HttpState::Ready(value) => value,
    }
}

#[function_component(App)]
pub fn component() -> Html {
    // consts
    let drawer_id = "left-drawer";

    // states
    let global: UseHttpHandleOption<(), GlobalConfiguration> = use_state_eq(Default::default);
    let i18n = use_state_eq(DynI18n::detect_language);
    let user = global.ok().and_then(|conf| conf.user.as_ref());

    // fetch
    let key = &();
    let fetch = |client: Client| async move { client.get_global_conf().await };
    let render = {
        let i18n = i18n.clone();
        move |state| {
            let conf = parse_conf(state);

            html! {
                <>
                    // Navigation bar (top)
                    <header class="navbar bg-white border-b border-gray-200 px-4 h-16 shrink-0 z-10">
                        // Sidebar logo
                        <div class="flex-none lg:hidden pr-2 text-blue-600">
                            <label for={ drawer_id } class="btn btn-ghost btn-circle drawer-button">
                                <svg
                                    class="h-6 w-6"
                                    fill="currentColor"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                >
                                    // heroicons:bars-3:solid
                                    <path fill-rule="evenodd" d="M3 6.75A.75.75 0 0 1 3.75 6h16.5a.75.75 0 0 1 0 1.5H3.75A.75.75 0 0 1 3 6.75ZM3 12a.75.75 0 0 1 .75-.75h16.5a.75.75 0 0 1 0 1.5H3.75A.75.75 0 0 1 3 12Zm0 5.25a.75.75 0 0 1 .75-.75h16.5a.75.75 0 0 1 0 1.5H3.75a.75.75 0 0 1-.75-.75Z" clip-rule="evenodd" />
                                </svg>
                            </label>
                        </div>

                        // Title
                        <a
                            class="flex-1 sm:flex-0 flex lg:flex-none items-center select-none truncate"
                            href={ conf.spec.redirect_url.as_ref().map(|url| url.to_string()).unwrap_or_else(|| "/".into()) }
                        >
                            { for conf.spec.logo_url.as_ref().map(|url| html! { <img
                                class="h-12 lg:mr-2"
                                src={ url.to_string() }
                            /> }) }
                            <span class="hidden lg:flex text-xl font-bold text-blue-600">{ conf.spec.title.clone() }</span>
                        </a>

                        // Search bar
                        <div class="flex-1 w-full hidden sm:flex justify-center px-10">
                            <div class="flex relative w-full">
                                <div
                                    class="flex items-center pointer-events-none pr-4"
                                >
                                    <svg
                                        class="h-5 w-5 text-gray-400"
                                        fill="none"
                                        stroke="currentColor"
                                        viewBox="0 0 24 24"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="2"
                                            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                                        />
                                    </svg>
                                </div>
                                <input
                                    id="navbar_search"
                                    type="text"
                                    placeholder="Search"
                                    class="flex-1 input bg-gray-100 border-none focus:bg-white focus:ring-2 focus:ring-blue-500 pl-10 rounded-xl"
                                />
                                <div class="flex items-center pl-3">
                                    <button class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition">
                                        { i18n.search_button() }
                                    </button>
                                </div>
                            </div>
                        </div>

                        // Misc
                        <div class="flex gap-2">
                            // Help
                            <button class="btn btn-ghost btn-circle">
                                <svg
                                    class="h-6 w-6 text-gray-500"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                                    />
                                </svg>
                            </button>
                            // Profile
                            { for user.and_then(|user| user.metadata.thumbnail_url.as_ref()).map(|url| html! {
                                <div class="avatar ml-2 w-10">
                                    <div class="mask mask-squircle">
                                        <img
                                            class="object-cover"
                                            src={ url.to_string() }
                                        />
                                    </div>
                                </div>
                            }) }
                        </div>
                    </header>

                    // Main contents
                    <main class="drawer lg:drawer-open flex-1 overflow-hidden">
                        // Drawer state placeholder
                        <input id={ drawer_id } type="checkbox" class="drawer-toggle" />

                        // Actual contents
                        <BrowserRouter>
                            <Switch<Route> render={ move |route| {
                                let props = RouteProps {
                                    conf: conf.clone(),
                                    drawer_id: drawer_id.into(),
                                    i18n: i18n.clone(),
                                };
                                switch(route, props)
                            } }/>
                        </BrowserRouter>
                    </main>
                </>
            }
        }
    };
    html! {
        <div class="flex flex-col h-screen bg-gray-50 font-sans">
            { global.try_fetch_and_render(&i18n, key, fetch, render) }
        </div>
    }
}
