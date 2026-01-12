use std::rc::Rc;

use chrono::{DateTime, Utc};
use openark_vine_browser_api::file::{FileEntry, FileRef};
use yew::{Html, Properties, function_component, html};
use yew_router::{hooks::use_navigator, prelude::Navigator};

fn render_file(nav: Option<&Navigator>, current: DateTime<Utc>, file: &FileRef) -> Html {
    let is_dir = file.is_dir();
    html! {
        <tr
            class="hover:bg-gray-100 cursor-pointer group transition-colors"
            onclick={ super::utils::push_dir(nav, file) }
        >
            <td>
                <div class="flex items-center space-x-3">
                    {{
                        let ty = file.metadata.ty.as_ref();
                        let size = 5;
                        super::mime::render(ty, is_dir, size)
                    }}
                    <span class="text-sm font-normal text-gray-700">{ file.name.clone() }</span>
                </div>
            </td>
            <td class="py-2 px-4">{
                for file.metadata.owner.as_ref().map(|user| html! {
                    <div class="flex items-center space-x-2">
                        <div class="avatar placeholder">
                            <div class="bg-blue-600 text-white rounded-full w-6 h-6 flex items-center justify-center overflow-hidden">
                                <span class="text-[10px] font-bold leading-none select-none flex items-center justify-center w-full h-full">
                                    { super::utils::format_initial(user) }
                                </span>
                            </div>
                        </div>
                        <span class="text-gray-600">{ user.name.clone() }</span>
                    </div>
                })
            }</td>
            <td class="text-sm text-gray-500">{{
                let timestamp = file.metadata.accessed.as_ref();
                super::utils::format_date(current, timestamp)
            }}</td>
            <td class="text-sm text-gray-500">{{
                let size = file.metadata.size;
                super::utils::format_size(is_dir, size)
            }}</td>
            <td class="text-right">
                <button class="btn btn-ghost btn-xs btn-circle opacity-0 group-hover:opacity-100">
                    <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                        <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
                    </svg>
                </button>
            </td>
        </tr>
    }
}

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) current: DateTime<Utc>,
    pub(super) directory: Rc<FileEntry>,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current && Rc::ptr_eq(&self.directory, &other.directory)
    }
}

#[function_component(FileList)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let Props { current, directory } = props.clone();

    // states
    let nav = use_navigator();

    html! {
        <div class="w-full overflow-x-auto">
            <table class="table w-full border-separate border-spacing-y-1">
                // Header
                <thead class="select-none">
                    <tr class="text-gray-500 border-b border-gray-100">
                        <th class="bg-transparent font-medium">{ "이름" }</th>
                        <th class="bg-transparent font-medium">{ "소유자" }</th>
                        <th class="bg-transparent font-medium">{ "마지막 수정" }</th>
                        <th class="bg-transparent font-medium">{ "파일 크기" }</th>
                        <th class="bg-transparent"></th>
                    </tr>
                </thead>

                // Body
                <tbody>{
                    for directory.files.iter().map(|file| render_file(nav.as_ref(), current, file))
                }</tbody>
            </table>
        </div>
    }
}
