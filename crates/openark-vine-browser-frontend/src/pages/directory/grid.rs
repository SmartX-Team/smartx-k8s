use std::rc::Rc;

use openark_vine_browser_api::file::{FileEntry, FileRef};
use yew::{Callback, Html, Properties, function_component, html, use_state_eq};

use crate::i18n::DynI18n;

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
    i18n: DynI18n,
    io: super::io::UseIOReducerHandle,
    onreload: Callback<()>,
    ptr: UploadFileItemPtr,
}

#[function_component(FileItem)]
fn render_item(props: &ItemProps) -> Html {
    // properties
    let &ItemProps {
        dir_state,
        ref drag_state,
        ref file,
        ref i18n,
        ref io,
        ref onreload,
        ptr,
    } = props;

    let is_dir = file.is_dir();

    html! {
        <UploadFileItem
            id="directory-dropzone-grid"
            { dir_state }
            drag_state={ drag_state.clone() }
            file={ file.clone() }
            io={ io.clone() }
            layout={ UploadFileItemLayout::Grid }
            onreload={ onreload.clone() }
            { ptr }
        >
            <div class="bg-white rounded-lg group p-4 w-full sm:w-60 pointer-events-none">
                <div class="mb-3">{{
                    let ty = file.ty();
                    let color = None;
                    let fill = true;
                    let size = 10;
                    super::mime::render_file_entry(ty, is_dir, color, fill, size)
                }}</div>
                <p class="text-sm font-semibold text-gray-700 truncate">{ file.name.clone() }</p>
                <p class="text-xs text-gray-400 mt-1">{{
                    let size = file.metadata.size;
                    i18n.format_size(is_dir, size)
                }}</p>
            </div>
        </UploadFileItem>
    }
}

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) directory: Rc<FileEntry>,
    pub(super) i18n: DynI18n,
    pub(super) io: super::io::UseIOReducerHandle,
    pub(super) onreload: Callback<()>,
    pub(super) state: super::FileEntryState,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.directory, &other.directory)
            && self.i18n == other.i18n
            && self.onreload == other.onreload
            && self.state == other.state
    }
}

#[function_component(FileList)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let &Props {
        ref directory,
        ref i18n,
        ref io,
        ref onreload,
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
            file={ directory.r.clone() }
            i18n={ i18n.clone() }
            io={ io.clone() }
            layout={ UploadFileItemLayout::Grid }
            onreload={ onreload.clone() }
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
                        i18n={ i18n.clone() }
                        io={ io.clone() }
                        onreload={ onreload.clone() }
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
