use std::rc::Rc;

use chrono::{DateTime, Utc};
use openark_vine_browser_api::file::{FileEntry, FileRef};
use web_sys::DataTransfer;
use yew::{
    Html, MouseEvent, Properties, Reducible, UseReducerHandle, function_component, html,
    html::IntoEventCallback, use_reducer_eq, use_state_eq,
};
use yew_router::hooks::use_navigator;

use crate::widgets::{
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
    current: DateTime<Utc>,
    drag_state: UseUploadFileStateHandle,
    file: FileRef,
    ptr: UploadFileItemPtr,
}

#[function_component(FileItem)]
fn render_item(props: &ItemProps) -> Html {
    // properties
    let &ItemProps {
        ref checkboxes,
        current,
        ref drag_state,
        ref file,
        ptr,
    } = props;

    let is_dir = file.is_dir();
    let td_class = if !is_dir && drag_state.is_some() {
        "pointer-events-none"
    } else {
        "pointer-events-auto"
    };

    // states
    let nav = use_navigator();

    html! {
        <UploadFileItem
            id="directory-dropzone-list"
            { ptr }
            drag_disabled={ !is_dir }
            drag_state={ drag_state.clone() }
            layout={ UploadFileItemLayout::List }
            onclick={ super::utils::push_entry(nav, file).into_event_callback() }
            ondrop={{
                let dst = file.clone();
                move |dt: DataTransfer| super::utils::upload(dt, dst.clone())
            }}
        >
            // Checkbox
            <td class={ format!("text-sm text-gray-500 {td_class}") }>
                <input
                    id={ format!("directory-checkbox-item-{}", &file.path) }
                    class="checkbox"
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
                        let ty = file.metadata.ty.as_ref();
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
                                    { super::utils::format_initial(user) }
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
                    let timestamp = file.metadata.accessed.as_ref();
                    super::utils::format_date(current, timestamp)
                }}</span>
            </td>
            // File size
            <td class={ format!("text-sm text-gray-500 {td_class}") }>
                <span class={ td_class }>{{
                    let size = file.metadata.size;
                    super::utils::format_size(is_dir, size)
                }}</span>
            </td>
        </UploadFileItem>
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
    let global_index = 0;
    let local_size = directory.files.len();

    // states
    let checkboxes = use_reducer_eq(CheckBoxGroup::default);
    let drag_state: UseUploadFileStateHandle = use_state_eq(Default::default);

    html! {
        <UploadFile
            id="directory-dropzone"
            drag_state={ drag_state.clone() }
            layout={ UploadFileItemLayout::List }
            ondrop={{
                let dst = directory.r.clone();
                move |dt: DataTransfer| super::utils::upload(dt, dst.clone())
            }}
        >
            <table class="table w-full border-collapse">
                // Header
                <thead class="select-none">
                    <tr
                        class={ format!("text-gray-500 border-t-2 border-l-2 border-r-2 border-gray-200 {}",
                            if local_size > 0 { "" } else { "border-b-2" },
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
                                            class="checkbox"
                                            type="checkbox"
                                            checked={ checkboxes.get_many(global_index, local_size) }
                                            onclick={{
                                                // Toggle
                                                let checkboxes = checkboxes.clone();
                                                move |event: MouseEvent| {
                                                    event.stop_propagation(); // Prevents the event from bubbling up
                                                    checkboxes.dispatch(CheckBoxAction::ToggleMany {
                                                        global_index: global_index,
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
                        <th class="bg-transparent font-medium">{ "이름" }</th>
                        <th class="bg-transparent font-medium">{ "소유자" }</th>
                        <th class="bg-transparent font-medium">{ "마지막 수정" }</th>
                        <th class="bg-transparent font-medium">{ "파일 크기" }</th>
                    </tr>
                </thead>

                // Body
                <tbody>{
                    for directory.files.iter().enumerate().map(|(local_index, file)| {
                        html! { <FileItem
                            checkboxes={ checkboxes.clone() }
                            { current }
                            drag_state={ drag_state.clone() }
                            file={ file.clone() }
                            ptr={ UploadFileItemPtr {
                                global_index: global_index + local_index,
                                local_index,
                                local_size,
                            } }
                        /> }
                    })
                }</tbody>
            </table>
        </UploadFile>
    }
}
