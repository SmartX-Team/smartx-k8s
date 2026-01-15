use yew::{Html, html};
use yew_router::prelude::Link;

use crate::{net::UseHttpHandleOptionRender, router::Route};

/// Directory link item.
///
fn draw_breadcrumb(name: &str, path: &str, active: bool) -> Html {
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
fn draw_breadcrumbs(path: &str) -> Html {
    // properties
    let path = path.trim_matches('/');

    let mut paths = vec![
        draw_breadcrumb("/", "", !path.is_empty()), // root directory
    ];

    let mut last_index = 0;
    for (index, name) in path.match_indices('/') {
        last_index = index;
        let path = &path[..index];
        let active = true;
        paths.push(draw_breadcrumb(name, path, active))
    }
    if !path.is_empty() && last_index < path.len() {
        let name = &path[last_index..];
        let active = false;
        paths.push(draw_breadcrumb(name, path, active)) // current file entry
    }

    html! {
        <ul class="text-blue-600 select-none">{
            for paths
        }</ul>
    }
}

fn draw_view_mode(ctx: &super::Context, mode: super::ViewMode, svg: Html) -> Html {
    // properties
    let is_enabled = *ctx.view_mode == mode;

    html! {
        <button
            class={
                if is_enabled {
                    // Enabled
                    "p-2 text-blue-600 bg-blue-50 rounded-lg"
                } else {
                    // Disabled
                    "p-2 text-gray-400 hover:text-gray-600"
                }
            }
            disabled={ is_enabled }
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
            draw_view_mode(ctx, mode, svg)
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
            draw_view_mode(ctx, mode, svg)
        });
    }

    html! {
        <div class="flex flex-col items-start mb-6">
            <div class="flex items-center justify-between w-full h-12">
                <h2 class="text-xl font-bold select-none text-gray-800">
                    { "내 파일" }
                </h2>
                <div class="flex space-x-2">{
                    for modes
                }</div>
            </div>
            <div class="flex breadcrumbs max-w-full">{
                draw_breadcrumbs(&ctx.props.path)
            }</div>
        </div>
    }
}
