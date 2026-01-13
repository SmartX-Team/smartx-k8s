use yew::{Html, html};
use yew_router::prelude::Link;

use crate::router::Route;

pub(super) fn render(ctx: &super::Context) -> Html {
    // properties
    let super::RouteProps { conf, drawer_id } = ctx.props.route.clone();

    html! {
        <aside class="drawer-side z-20 h-full">
            <label for={ drawer_id } aria-label="close sidebar" class="drawer-overlay"></label>
            <div class="flex flex-col pt-4 w-64 h-full bg-white border-r border-gray-200">
                <div class="p-6 lg:hidden select-none">
                    <h1 class="text-xl font-bold text-blue-600">{ conf.title.clone() }</h1>
                </div>

                <nav class="flex-1 px-4 select-none space-y-1 overflow-y-auto">
                    <Link<Route>
                        classes="flex items-center px-4 py-2 text-sm font-medium rounded-lg bg-blue-600 text-white"
                        to={ Route::FileEntry {
                            path: Default::default(),
                        } }
                    >
                        <svg
                            class="w-5 h-5 mr-3"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                            />
                        </svg>
                        { "모든 파일" }
                    </Link<Route>>
                    <Link<Route>
                        classes="flex items-center px-4 py-2 text-sm font-medium rounded-lg hover:bg-gray-50 text-gray-600 transition"
                        to={ Route::FileEntry {
                            path: Default::default(),
                        } }
                    >
                        <svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                        { "최근 항목" }
                    </Link<Route>>
                    <Link<Route>
                        classes="flex items-center px-4 py-2 text-sm font-medium rounded-lg hover:bg-gray-50 text-gray-600 transition"
                        to={ Route::FileEntry {
                            path: Default::default(),
                        } }
                    >
                        <svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.175 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.382-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"></path></svg>
                        { "즐겨찾기" }
                    </Link<Route>>
                    <Link<Route>
                        classes="flex items-center px-4 py-2 text-sm font-medium rounded-lg hover:bg-gray-50 text-gray-600 transition"
                        to={ Route::FileEntry {
                            path: Default::default(),
                        } }
                    >
                        <svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                        { "휴지통" }
                    </Link<Route>>
                </nav>

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
                        <p class="text-xs font-semibold text-blue-600 uppercase">{ "용량 사용량" }</p>
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
