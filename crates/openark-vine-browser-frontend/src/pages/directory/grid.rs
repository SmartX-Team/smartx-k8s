use std::rc::Rc;

use openark_vine_browser_api::file::{FileEntry, FileRef};
use yew::{Html, Properties, function_component, html, html::IntoEventCallback, use_state_eq};
use yew_router::hooks::use_navigator;

use super::upload::{
    UploadFile, UploadFileItem, UploadFileItemLayout, UploadFileItemPtr, UseUploadFileStateHandle,
};

#[derive(Clone, Debug, PartialEq, Properties)]
struct ItemProps {
    // checkboxes: UseReducerHandle<CheckBoxGroup>,
    // current: DateTime<Utc>,
    dir_state: super::FileEntryState,
    drag_state: UseUploadFileStateHandle,
    file: FileRef,
    ptr: UploadFileItemPtr,
}

#[function_component(FileItem)]
fn render_item(props: &ItemProps) -> Html {
    // properties
    let &ItemProps {
        dir_state,
        ref drag_state,
        ref file,
        ptr,
    } = props;

    let is_dir = file.is_dir();

    // states
    let nav = use_navigator();

    html! {
        <UploadFileItem
            id="directory-dropzone-grid"
            { dir_state }
            drag_disabled={ !is_dir }
            drag_state={ drag_state.clone() }
            layout={ UploadFileItemLayout::Grid }
            onclick={ super::utils::push_entry(nav, file).into_event_callback() }
            ondrop={{
                let dst = file.clone();
                move |event| super::utils::upload(event, dst.clone())
            }}
            { ptr }
        >
            <div class="bg-white rounded-lg group p-4 w-full sm:w-60 pointer-events-none">
                <div class="mb-3">{{
                    let ty = file.metadata.ty.as_ref();
                    let color = None;
                    let fill = true;
                    let size = 10;
                    super::mime::render_file_entry(ty, is_dir, color, fill, size)
                }}</div>
                <p class="text-sm font-semibold text-gray-700 truncate">{ file.name.clone() }</p>
                <p class="text-xs text-gray-400 mt-1">{{
                    let size = file.metadata.size;
                    super::utils::format_size(is_dir, size)
                }}</p>
            </div>
        </UploadFileItem>
    }
}

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) directory: Rc<FileEntry>,
    pub(super) state: super::FileEntryState,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.directory, &other.directory) && self.state == other.state
    }
}

#[function_component(FileList)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let &Props {
        ref directory,
        state,
    } = props;
    let global_index = 0;
    let local_size = directory.files.len();

    // states
    let drag_state: UseUploadFileStateHandle = use_state_eq(Default::default);

    html! {
        <UploadFile
            id="directory-dropzone"
            dir_state={ state }
            drag_state={ drag_state.clone() }
            layout={ UploadFileItemLayout::Grid }
            ondrop={{
                let dst = directory.r.clone();
                move |event| super::utils::upload(event, dst.clone())
            }}
        >
            // Files
            <div class="flex flex-wrap gap-4">{
                for directory.files.iter().enumerate().map(|(local_index, file)| {
                    html! { <FileItem
                        // checkboxes={ checkboxes.clone() }
                        // { current }
                        dir_state={ state }
                        drag_state={ drag_state.clone() }
                        file={ file.clone() }
                        ptr={ UploadFileItemPtr {
                            global_index: global_index + local_index,
                            local_index,
                            local_size,
                        } }
                    /> }
                })
            }</div>
        </UploadFile>
    }
}
