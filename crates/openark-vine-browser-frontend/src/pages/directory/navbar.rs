use yew::{Html, html};
use yew_router::prelude::Link;

use crate::{
    net::{UseHttpHandleOptionRender, get_file_content_url},
    router::Route,
};

/// Directory link item.
///
fn render_breadcrumb(name: &str, path: &str, active: bool) -> Html {
    let bg = if active { "bg-blue-600" } else { "bg-blue-500" };

    let item = html! {
        <div class={ format!("p-1 pl-2 pr-2 text-sm rounded-lg {bg} text-white") }>{
            name.to_string()
        }</div>
    };

    html! {
        <li>{
            if active {
                html! {
                    <Link<Route>
                        disabled={ !active }
                        to={ Route::FileEntry {
                            path: path.trim_matches('/').to_string(),
                        } }
                    >
                        { item }
                    </Link<Route>>

                }
            } else {
                item
            }
        }</li>
    }
}

/// Directory links.
///
fn render_breadcrumbs(path: &str) -> Html {
    // properties
    let path = path.trim_matches('/');

    let mut paths = vec![
        render_breadcrumb("/", "", !path.is_empty()), // root directory
    ];

    let mut last_index = 0;
    for (index, name) in path.match_indices('/') {
        last_index = index;
        let path = &path[..index];
        let active = true;
        paths.push(render_breadcrumb(name, path, active))
    }
    if !path.is_empty() && last_index < path.len() {
        let name = &path[last_index..];
        let active = false;
        paths.push(render_breadcrumb(name, path, active)) // current file entry
    }

    html! {
        <ul class="text-blue-600 select-none">{
            for paths
        }</ul>
    }
}

fn render_view_mode(ctx: &super::Context, mode: super::ViewMode, svg: Html) -> Html {
    // properties
    let enabled = *ctx.view_mode == mode;
    let i18n = &ctx.props.route.i18n;

    html! {
        <button
            class={
                if enabled {
                    // Enabled
                    "p-2 tooltip cursor-pointer transition-colors text-blue-600 bg-blue-50 rounded-lg"
                } else {
                    // Disabled
                    "p-2 tooltip cursor-pointer transition-colors text-gray-400 hover:text-gray-600"
                }
            }
            data-tip={ match mode {
                super::ViewMode::Grid => i18n.indicator_as_grid(),
                super::ViewMode::List => i18n.indicator_as_list(),
            } }
            disabled={ enabled }
            onclick={{
                let state = ctx.view_mode.clone();
                move |_| state.set(mode)
            }}
        >
            { svg }
        </button>
    }
}

pub(super) fn render(ctx: &super::Context) -> Html {
    // properties
    let file_entry = ctx.file_entry.ok();
    let i18n = &ctx.props.route.i18n;
    let is_dir = file_entry.is_none_or(|e| e.r.is_dir());

    let mut modes = vec![];
    if is_dir {
        // Grid mode button
        modes.push({
            let mode = super::ViewMode::Grid;
            let svg = html! {
                <svg
                    class="h-5 w-5"
                    fill="currentColor"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    // heroicons:squares-2x2:solid
                    <path fill-rule="evenodd" d="M3 6a3 3 0 0 1 3-3h2.25a3 3 0 0 1 3 3v2.25a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3V6Zm9.75 0a3 3 0 0 1 3-3H18a3 3 0 0 1 3 3v2.25a3 3 0 0 1-3 3h-2.25a3 3 0 0 1-3-3V6ZM3 15.75a3 3 0 0 1 3-3h2.25a3 3 0 0 1 3 3V18a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3v-2.25Zm9.75 0a3 3 0 0 1 3-3H18a3 3 0 0 1 3 3V18a3 3 0 0 1-3 3h-2.25a3 3 0 0 1-3-3v-2.25Z" clip-rule="evenodd" />
                </svg>
            };
            render_view_mode(ctx, mode, svg)
        });

        // List mode button
        modes.push({
            let mode = super::ViewMode::List;
            let svg = html! {
                <svg
                    class="h-5 w-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    // heroicons:bars-3:solid
                    <path fill-rule="evenodd" d="M3 6.75A.75.75 0 0 1 3.75 6h16.5a.75.75 0 0 1 0 1.5H3.75A.75.75 0 0 1 3 6.75ZM3 12a.75.75 0 0 1 .75-.75h16.5a.75.75 0 0 1 0 1.5H3.75A.75.75 0 0 1 3 12Zm0 5.25a.75.75 0 0 1 .75-.75h16.5a.75.75 0 0 1 0 1.5H3.75a.75.75 0 0 1-.75-.75Z" clip-rule="evenodd" />
                </svg>
            };
            render_view_mode(ctx, mode, svg)
        })
    } else {
        // Download button
        modes.push({
            html! {
                <a
                    class="p-2 cursor-pointer transition-colors bg-purple-100 hover:bg-purple-200 active:bg-purple-300 text-purple-400 hover:text-purple-600 rounded-lg shrink tooltip"
                    data-tip={ i18n.indicator_download() }
                    download=""
                    href={ file_entry
                        .and_then(|entry| get_file_content_url(&entry.r).ok())
                        .map(|url| url.to_string())
                        .unwrap_or_else(|| "#".into())
                    }
                >
                    <svg
                        class="h-5 w-5"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        viewBox="0 0 24 24"
                    >
                        // heroicons:arrow-down-tray:outline
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
                    </svg>
                </a>
            }
        })
    }
    modes.push(
        // I/O status button
        html! {
            <super::io::IOStatus
                i18n={ (**i18n).clone() }
                io={ ctx.io.clone() }
            />
        },
    );

    html! {
        <div class="flex flex-col items-start mb-6">
            <div class="flex items-center justify-between w-full h-12">
                <h2 class="text-xl font-bold select-none text-gray-800">{
                    i18n.indicator_my_files()
                }</h2>
                <div class="flex items-center space-x-2">{
                    for modes
                }</div>
            </div>
            <div class="flex breadcrumbs max-w-full">{
                render_breadcrumbs(&ctx.props.path)
            }</div>
        </div>
    }
}
