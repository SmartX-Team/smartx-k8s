use std::rc::Rc;

use openark_vine_browser_api::file::{FileEntry, FileRef};
use yew::{Html, Properties, function_component, html};
use yew_router::{hooks::use_navigator, prelude::Navigator};

fn render_file(nav: Option<&Navigator>, file: &FileRef) -> Html {
    let is_dir = file.is_dir();
    html! {
        <div
            class="group bg-white p-4 w-full sm:w-60 rounded-xl border border-gray-200 hover:shadow-md transition cursor-pointer"
            onclick={ super::utils::push_dir(nav, file) }
        >
            <div class="mb-3">{{
                let ty = file.metadata.ty.as_ref();
                let size = 10;
                super::mime::render(ty, is_dir, size)
            }}</div>
            <p class="text-sm font-semibold text-gray-700 truncate">{ file.name.clone() }</p>
            <p class="text-xs text-gray-400 mt-1">{{
                let size = file.metadata.size;
                super::utils::format_size(is_dir, size)
            }}</p>
        </div>
    }
}

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) directory: Rc<FileEntry>,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.directory, &other.directory)
    }
}

#[function_component(FileList)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let Props { directory } = props.clone();

    // states
    let nav = use_navigator();

    html! {
        <div class="flex flex-wrap gap-4">{
            for directory.files.iter().map(|file| render_file(nav.as_ref(), file))
        }</div>
    }
}
