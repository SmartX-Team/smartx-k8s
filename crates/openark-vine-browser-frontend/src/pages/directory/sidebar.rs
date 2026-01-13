use yew::{Html, html};
use yew_router::prelude::Link;

use crate::router::Route;

pub(super) fn render(ctx: &super::Context) -> Html {
    // properties
    let super::RouteProps { conf, drawer_id } = ctx.props.route.clone();
    let shortcuts = conf
        .user
        .as_ref()
        .map(|user| user.shortcuts.as_slice())
        .unwrap_or_default();

    html! {
        <aside class="drawer-side z-20 h-full">
            <label for={ drawer_id } aria-label="close sidebar" class="drawer-overlay"></label>
            <div class="flex flex-col pt-4 w-64 h-full bg-white border-r border-gray-200">
                // Title
                <div class="p-6 lg:hidden select-none">
                    <h1 class="text-xl font-bold text-blue-600">{ conf.title.clone() }</h1>
                </div>

                // Add new entry
                <div class="dropdown pl-4 mb-4">
                    <div
                        tabindex="0"
                        role="button"
                        class="btn btn-ghost bg-slate-100 hover:bg-slate-50 shadow-md hover:shadow-lg rounded-full px-6 py-2 normal-case border border-slate-200 transition-all duration-200 gap-2 text-blue-500"
                    >
                        <svg
                            class="w-6 h-6"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            viewBox="0 0 24 24"
                        >
                            // heroicons:plus:outline
                            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                        </svg>
                        <h2 class="text-gray-700 font-medium">{ "신규" }</h2>
                    </div>

                    <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow-xl bg-slate-200 text-gray-700 rounded-lg w-56 mt-2">
                        <li class="hover:bg-slate-100 rounded-md">
                            <a class="p-2">
                                <svg
                                    class="w-5 h-5"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                >
                                    // heroicons:folder-plus:outline
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 10.5v6m3-3H9m4.06-7.19-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z" />
                                </svg>
                                <span class="ml-1">{ "New folder" }</span>
                            </a>
                        </li>
                        <hr class="my-1 border-gray-500" />
                        <li class="hover:bg-slate-100 rounded-md">
                            <a class="p-2">
                                <svg
                                    class="w-5 h-5"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                >
                                    // heroicons:document:outline
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
                                </svg>
                                <span class="ml-1">{ "File upload" }</span>
                            </a>
                        </li>
                        <li class="hover:bg-slate-100 rounded-md">
                            <a class="p-2">
                                <svg
                                    class="w-5 h-5"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                >
                                    // heroicons:folder-open:outline
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 9.776c.112-.017.227-.026.344-.026h15.812c.117 0 .232.009.344.026m-16.5 0a2.25 2.25 0 0 0-1.883 2.542l.857 6a2.25 2.25 0 0 0 2.227 1.932H19.05a2.25 2.25 0 0 0 2.227-1.932l.857-6a2.25 2.25 0 0 0-1.883-2.542m-16.5 0V6A2.25 2.25 0 0 1 6 3.75h3.879a1.5 1.5 0 0 1 1.06.44l2.122 2.12a1.5 1.5 0 0 0 1.06.44H18A2.25 2.25 0 0 1 20.25 9v.776" />
                                </svg>
                                <span class="ml-1">{ "Folder upload" }</span>
                            </a>
                        </li>
                    </ul>
                </div>

                // Shortcuts
                <nav class="flex-1 px-4 select-none space-y-1 overflow-y-auto">{
                    for shortcuts.iter().map(|file| {
                        let is_current = file.r.path.trim_matches('/') == ctx.props.path.trim_matches('/');
                        let is_dir = file.r.is_dir();
                        html! {
                            <div
                                class={ format!(
                                    "px-4 py-2 text-sm font-medium rounded-lg {}",
                                    if is_current { "bg-blue-600 text-white cursor-default" } else { "hover:bg-gray-50 text-gray-600 transition" }
                                ) }
                            >
                                <Link<Route>
                                    classes="flex items-center"
                                    disabled={ is_current }
                                    to={ Route::FileEntry {
                                        path: file.r.path.trim_matches('/').to_string(),
                                    } }
                                >
                                    {{
                                        let kind = file.kind;
                                        let ty = file.r.metadata.ty.as_ref();
                                        let color = "";
                                        let fill = false;
                                        let size = 5;
                                        super::mime::render_file_shortcut(kind, ty, is_dir, color, fill, size)
                                    }}
                                    <span class="ml-3">{ file.r.name.clone() }</span>
                                </Link<Route>>
                            </div>
                        }
                    })
                }</nav>

                // Subscriptions
                <div class="p-4 border-t border-gray-200 select-none">
                    <div class="flex justify-between items-start">
                        <div>
                            <h2 class="text-sm font-medium text-blue-600">{ "현재 구독 티어" }</h2>
                            <p class="text-2xl font-bold text-blue-700 mt-1">{ "Premium" }</p>
                        </div>
                        <div class="badge badge-primary badge-outline">{ "Active" }</div>
                    </div>
                </div>

                // Usage / Capacity
                <div class="p-4 border-t border-gray-200">
                    <div class="bg-blue-50 p-4 rounded-xl">
                        <p class="text-xs font-semibold text-blue-600 select-none uppercase">{ "용량 사용량" }</p>
                        <div class="mt-2 w-full bg-blue-200 rounded-full h-1.5">
                            <div class="bg-blue-600 h-1.5 rounded-full" style="width: 75%"></div>
                        </div>
                        <p class="mt-2 text-xs text-blue-700">{ "2.0PiB 중 1.5PiB 사용 중" }</p>
                    </div>
                </div>
            </div>
        </aside>
    }
}
