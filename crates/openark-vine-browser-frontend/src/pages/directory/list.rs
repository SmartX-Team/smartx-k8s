use std::rc::Rc;

use jiff::Timestamp;
use openark_vine_browser_api::file::{FileEntry, FileRef};
use yew::{
    Callback, Html, MouseEvent, Properties, Reducible, UseReducerHandle, UseStateHandle,
    function_component, html, use_reducer_eq, use_state_eq,
};

use crate::i18n::DynI18n;

use super::upload::{
    UploadFile, UploadFileItem, UploadFileItemLayout, UploadFileItemPtr, UseUploadFileStateHandle,
};

#[derive(Clone, Debug, PartialEq)]
enum CheckBoxAction {
    ToggleItem { global_index: usize },
    ToggleMany { global_index: usize, size: usize },
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CheckBoxGroup {
    values: Vec<bool>,
}

impl CheckBoxGroup {
    fn get_item(&self, global_index: usize) -> bool {
        self.values.get(global_index).copied().unwrap_or(false)
    }

    fn get_many(&self, global_index: usize, size: usize) -> bool {
        global_index + size <= self.values.len()
            && self.values.iter().skip(global_index).take(size).all(|v| *v)
    }

    fn reserve(&mut self, len: usize) {
        if let Some(more) = len.checked_sub(self.values.len())
            && more > 0
        {
            self.values.resize(len, false)
        }
    }
}

impl Reducible for CheckBoxGroup {
    type Action = CheckBoxAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut this = (*self).clone();
        match action {
            CheckBoxAction::ToggleItem { global_index } => {
                this.reserve(global_index + 1);
                this.values[global_index] ^= true
            }
            CheckBoxAction::ToggleMany { global_index, size } => {
                let old_value = this.get_many(global_index, size);
                let end = global_index + size;
                this.reserve(end);
                this.values[global_index..end].fill(!old_value)
            }
        }
        Rc::new(this)
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
struct ItemProps {
    checkboxes: UseReducerHandle<CheckBoxGroup>,
    current: Timestamp,
    dir_state: super::FileEntryState,
    drag_state: UseUploadFileStateHandle,
    file: FileRef,
    i18n: DynI18n,
    io: super::io::UseIOReducerHandle,
    onreload: Callback<()>,
    ptr: UploadFileItemPtr,
    selected: UseStateHandle<Option<usize>>,
}

#[function_component(FileItem)]
fn render_item(props: &ItemProps) -> Html {
    // properties
    let &ItemProps {
        ref checkboxes,
        current,
        dir_state,
        ref drag_state,
        ref file,
        ref i18n,
        ref io,
        ref onreload,
        ptr,
        ref selected,
    } = props;

    let is_dir = file.is_dir();
    let td_class = if !is_dir && drag_state.is_some() {
        "pointer-events-none"
    } else {
        "pointer-events-auto"
    };

    html! {
        <UploadFileItem
            id="directory-dropzone-list"
            { dir_state }
            drag_state={ drag_state.clone() }
            file={ file.clone() }
            io={ io.clone() }
            layout={ UploadFileItemLayout::List }
            onreload={ onreload.clone() }
            { ptr }
            selected={ selected.clone() }
        >
            // Checkbox
            <td class={ format!("text-sm text-gray-500 {td_class}") }>
                <input
                    id={ format!("directory-checkbox-item-{}", &file.path) }
                    class="checkbox border-gray-200 text-blue-800"
                    type="checkbox"
                    checked={ checkboxes.get_item(ptr.global_index) }
                    onclick={{
                        // Toggle
                        let checkboxes = checkboxes.clone();
                        move |event: MouseEvent| {
                            event.stop_propagation(); // Prevents the event from bubbling up
                            checkboxes.dispatch(CheckBoxAction::ToggleItem {
                                global_index: ptr.global_index,
                            })
                        }
                    }}
                />
            </td>
            // File name
            <td class={ td_class }>
                <div class={ format!("flex items-center space-x-3 {td_class}") }>
                    {{
                        let ty = file.ty();
                        let color = None;
                        let fill = true;
                        let size = 5;
                        super::mime::render_file_entry(ty, is_dir, color, fill, size)
                    }}
                    <span class="text-sm font-normal text-gray-700">{ file.name.clone() }</span>
                </div>
            </td>
            // Owner
            <td class={ format!("py-2 px-4 {td_class}") }>{
                for file.metadata.owner.as_ref().map(|user| html! {
                    <div class={ format!("flex items-center space-x-2 {td_class}") }>
                        <div class="avatar placeholder">
                            <div class="bg-blue-600 text-white rounded-full w-6 h-6 flex items-center justify-center overflow-hidden">
                                <span class="text-[10px] font-bold leading-none select-none flex items-center justify-center w-full h-full">
                                    { i18n.format_initial(user) }
                                </span>
                            </div>
                        </div>
                        <span class="text-gray-600">{ user.name.clone() }</span>
                    </div>
                })
            }</td>
            // Last modified
            <td class={ format!("text-sm text-gray-500 {td_class}") }>
                <span class={ td_class }>{{
                    let timestamp = file.metadata.modified.as_ref().map(|ts| ts.timestamp);
                    i18n.format_date(timestamp, current)
                }}</span>
            </td>
            // File size
            <td class={ format!("text-sm text-gray-500 {td_class}") }>
                <span class={ td_class }>{{
                    let size = file.metadata.size;
                    i18n.format_size(is_dir, size)
                }}</span>
            </td>
        </UploadFileItem>
    }
}

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) current: Timestamp,
    pub(super) directory: Rc<FileEntry>,
    pub(super) i18n: DynI18n,
    pub(super) indices: UseReducerHandle<super::FileIndices>,
    pub(super) io: super::io::UseIOReducerHandle,
    pub(super) onreload: Callback<()>,
    pub(super) selected: UseStateHandle<Option<usize>>,
    pub(super) state: super::FileEntryState,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
            && Rc::ptr_eq(&self.directory, &other.directory)
            && self.i18n == other.i18n
            && self.indices == other.indices
            && self.onreload == other.onreload
            && self.selected == other.selected
            && self.state == other.state
    }
}

#[function_component(FileList)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let &Props {
        current,
        ref directory,
        ref i18n,
        ref indices,
        ref io,
        ref onreload,
        ref selected,
        state,
    } = props;

    let has_indices = indices.len() == directory.files.len();
    let global_index = 0;
    let local_size = directory.files.len();

    // states
    let checkboxes = use_reducer_eq(CheckBoxGroup::default);
    let drag_state: UseUploadFileStateHandle = use_state_eq(Default::default);

    html! {
        <UploadFile
            id="directory-dropzone"
            dir_state={ state }
            drag_state={ drag_state.clone() }
            file={ directory.r.clone() }
            i18n={ i18n.clone() }
            io={ io.clone() }
            layout={ UploadFileItemLayout::List }
            onreload={ onreload.clone() }
        >
            <table class="table w-full border-collapse">
                // Header
                <thead class="select-none">
                    <tr
                        class={ format!(
                            "text-gray-500 border-t-2 border-l-2 border-r-2 border-gray-200 {}",
                            if local_size == 0 { "border-b-2" } else { "" },
                        ) }
                    >
                        // Checkbox
                        {
                            if directory.files.is_empty() {
                                // Hide if there is no file
                                html! {}
                            } else {
                                html! {
                                    <th class="bg-transparent font-medium pointer-events-auto">
                                        <input
                                            id="directory-grid-checkbox-all"
                                            class="checkbox border-gray-200 text-blue-800"
                                            type="checkbox"
                                            checked={ checkboxes.get_many(global_index, local_size) }
                                            onclick={{
                                                // Toggle
                                                let checkboxes = checkboxes.clone();
                                                move |event: MouseEvent| {
                                                    event.stop_propagation(); // Prevents the event from bubbling up
                                                    checkboxes.dispatch(CheckBoxAction::ToggleMany {
                                                        global_index,
                                                        size: local_size,
                                                    })
                                                }
                                            }}
                                        />
                                    </th>
                                }
                            }
                        }
                        // Metadata
                        <th class="pointer-events-auto cursor-pointer hover:bg-base-200 transition-colors">
                            <div class="flex items-center gap-2">
                                <span class="font-semibold">{ i18n.file_name() }</span>
                                <span class="flex flex-col -space-y-1">
                                    <svg class="h-3 w-3 fill-current opacity-30 hover:opacity-100" viewBox="0 0 20 20">
                                        <path d="M10 3l-7 7h14l-7-7z" />
                                    </svg>
                                    <svg class="h-3 w-3 fill-current opacity-100" viewBox="0 0 20 20">
                                        <path d="M10 17l7-7H3l7 7z" />
                                    </svg>
                                </span>
                            </div>
                        </th>
                        <th class="bg-transparent font-medium">{ i18n.file_owner() }</th>
                        <th class="bg-transparent font-medium">{ i18n.date_modified() }</th>
                        <th class="bg-transparent font-medium">{ i18n.file_size() }</th>
                    </tr>
                </thead>

                // Body
                <tbody>{
                    for (0..directory.files.len())
                        .map(|mut local_index| {
                            if has_indices {
                                local_index = indices[local_index];
                            }
                            let file = &directory.files[local_index];
                            (local_index, file)
                        })
                        .map(|(local_index, file)| {
                            html! { <FileItem
                                checkboxes={ checkboxes.clone() }
                                { current }
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
                                selected={ selected.clone() }
                            /> }
                        })
                }</tbody>
            </table>
        </UploadFile>
    }
}
