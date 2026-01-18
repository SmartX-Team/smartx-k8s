use openark_vine_browser_api::file::FileRef;
use web_sys::{
    DataTransfer,
    wasm_bindgen::{JsCast, prelude::Closure},
};
use yew::{
    Callback, DragEvent, Html, MouseEvent, Properties, UseStateHandle, function_component, html,
    html::IntoEventCallback,
};
use yew_router::{hooks::use_navigator, prelude::Navigator};

use crate::{
    i18n::DynI18n,
    router::Route,
    widgets::{Empty, FileNotFound},
};

use super::io::UseIOReducerHandleExt;

const DATA_TRANSFER_KIND_CONTAINER: &str = "string";
const DATA_TRANSFER_TYPE_CONTAINER: &str = concat!(env!("CARGO_CRATE_NAME"), "/file-entry");

const DATA_TRANSFER_KIND_FILE: &str = "file";

/// Resolve to the `file`.
///
fn handle_onclick(nav: Option<Navigator>, file: &FileRef) -> impl IntoEventCallback<MouseEvent> {
    let path = file.path.trim_matches('/').to_string();
    move |_| {
        if let Some(nav) = nav.clone() {
            nav.push(&Route::FileEntry { path: path.clone() })
        }
    }
}

fn handle_ondragstart(file: &FileRef) -> impl IntoEventCallback<DragEvent> {
    let file = file.clone();
    move |event: DragEvent| {
        if let Some(dt) = event.data_transfer() {
            let items = dt.items();
            if let Ok(file) = ::serde_json::to_string(&file) {
                items
                    .add_with_str_and_type(&file, DATA_TRANSFER_TYPE_CONTAINER)
                    .unwrap();
            }
        }
    }
}

fn handle_ondragenter(
    global_index: Option<usize>,
    is_dir: bool,
    is_ready: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_dir || !is_ready {
            return;
        }
        if let Some((new_state, _)) = UploadDragState::parse(&event, global_index) {
            last_state.set(Some(new_state))
        }
    }
}

fn handle_ondragover(
    global_index: Option<usize>,
    is_dir: bool,
    is_ready: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_dir || !is_ready {
            return;
        }
        if is_ready && let Some((new_state, _)) = UploadDragState::parse(&event, global_index) {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
            last_state.set(Some(new_state))
        } else if last_state.is_some() {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
        }
    }
}

fn handle_ondragleave(
    global_index: Option<usize>,
    is_dir: bool,
    is_ready: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let dst = UploadDragDestination::from_global_index(global_index);
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_dir || !is_ready {
            return;
        }
        if matches!(dst, UploadDragDestination::Container) && last_state.is_some() {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
            last_state.set(None)
        }
    }
}

/// Upload a file into the `dst` directory.
///
fn handle_ondrop(
    dst: &FileRef,
    global_index: Option<usize>,
    is_dir: bool,
    is_ready: bool,
    io: &super::io::UseIOReducerHandle,
    last_state: &UseUploadFileStateHandle,
    oncomplete: &Callback<()>,
) -> impl IntoEventCallback<DragEvent> {
    let dst = dst.clone();
    let io = io.clone();
    let last_state = last_state.clone();
    let oncomplete = oncomplete.clone();
    move |event: DragEvent| {
        if !is_dir || !is_ready {
            return;
        }
        if let Some((_, dt)) = UploadDragState::parse(&event, global_index) {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
            last_state.set(None);

            let items = dt.items();
            for item in (0..items.length()).filter_map(|index| items.get(index)) {
                let kind = item.kind();
                let ty = item.type_();
                // Is it a container item?
                if kind == DATA_TRANSFER_KIND_CONTAINER && ty == DATA_TRANSFER_TYPE_CONTAINER {
                    let dst = dst.clone();
                    let io = io.clone();
                    let oncomplete = oncomplete.clone();
                    let closure = move |file: String| {
                        if let Ok(src) = ::serde_json::from_str(&file) {
                            io.r#move(src, &dst, oncomplete.clone())
                        }
                    };
                    // AddEventListener
                    let callback = Closure::once(Box::new(closure) as Box<dyn FnOnce(String)>);
                    item.get_as_string(Some(callback.as_ref().unchecked_ref()))
                        .unwrap();
                    callback.forget()
                }
                // Is it an external item?
                else if kind == DATA_TRANSFER_KIND_FILE
                    && let Ok(Some(src)) = item.get_as_file()
                {
                    io.upload_file(src, &dst, oncomplete.clone())
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum UploadDragSourceOrigin {
    Container,
    External,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum UploadDragSourceType {
    SingleFile,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct UploadDragSource {
    origin: UploadDragSourceOrigin,
    ty: UploadDragSourceType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum UploadDragDestination {
    Container,
    Item(usize),
}

impl UploadDragDestination {
    const fn from_global_index(global_index: Option<usize>) -> Self {
        match global_index {
            None => Self::Container,
            Some(index) => Self::Item(index),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct UploadDragState {
    src: UploadDragSource,
    dst: UploadDragDestination,
}

impl UploadDragState {
    fn parse(event: &DragEvent, global_index: Option<usize>) -> Option<(Self, DataTransfer)> {
        let dt = event.data_transfer()?;
        let items = dt.items();

        let mut src = None;
        for item in (0..items.length()).filter_map(|index| items.get(index)) {
            let kind = item.kind();
            let ty = item.type_();
            // Is it a container item?
            if kind == DATA_TRANSFER_KIND_CONTAINER && ty == DATA_TRANSFER_TYPE_CONTAINER {
                src = Some(UploadDragSource {
                    origin: UploadDragSourceOrigin::Container,
                    ty: UploadDragSourceType::SingleFile,
                });
                break;
            }
            // Is it an external item?
            else if kind == DATA_TRANSFER_KIND_FILE {
                src = Some(UploadDragSource {
                    origin: UploadDragSourceOrigin::External,
                    ty: UploadDragSourceType::SingleFile,
                })
            }
        }

        let this = Self {
            src: src?,
            dst: UploadDragDestination::from_global_index(global_index),
        };
        Some((this, dt))
    }
}

pub(super) type UseUploadFileStateHandle = UseStateHandle<Option<UploadDragState>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum UploadFileItemLayout {
    Grid,
    List,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct UploadFileItemPtr {
    pub(super) global_index: usize,
    pub(super) local_index: usize,
    pub(super) local_size: usize,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub(super) struct UploadFileItemProps {
    pub(super) id: String,
    pub(super) dir_state: super::FileEntryState,
    pub(super) drag_state: UseUploadFileStateHandle,
    pub(super) file: FileRef,
    pub(super) io: super::io::UseIOReducerHandle,
    pub(super) layout: UploadFileItemLayout,
    pub(super) onreload: Callback<()>,
    pub(super) ptr: UploadFileItemPtr,
    pub(super) children: Html,
}

#[function_component(UploadFileItem)]
pub(super) fn render(props: &UploadFileItemProps) -> Html {
    // properties
    let &UploadFileItemProps {
        ref id,
        dir_state,
        ref drag_state,
        ref file,
        ref io,
        layout,
        ref onreload,
        ptr:
            UploadFileItemPtr {
                global_index,
                local_index,
                local_size,
            },
        ref children,
    } = props;

    let is_dir = file.is_dir();
    let is_ready = matches!(
        dir_state,
        super::FileEntryState::Directory | super::FileEntryState::Empty
    );

    let mut default_class = String::from(concat!(
        "group border-l-2 border-r-2 cursor-pointer hover:shadow-lg transition-colors transition-shadow",
        " sm:select-all active:bg-blue-50 no-drag-highlight",
    ));
    match layout {
        UploadFileItemLayout::Grid => {
            default_class.push_str(" border-t-2 border-b-2 w-full sm:w-auto rounded-lg")
        }
        UploadFileItemLayout::List => {
            if local_index == 0 {
                default_class.push_str(" border-t-2")
            }
            if local_index + 1 == local_size {
                default_class.push_str(" border-b-2")
            }
        }
    }
    match **drag_state {
        Some(state) => match state.dst {
            UploadDragDestination::Item(i) if i == global_index => {
                default_class.push_str(" pointer-events-auto border-dashed border-blue-300");
                if matches!(layout, UploadFileItemLayout::List) {
                    if local_index > 0 {
                        default_class.push_str(" border-t-2")
                    }
                    if local_index + 1 < local_size {
                        default_class.push_str(" border-b-2")
                    }
                }
            }
            UploadDragDestination::Item(i) => {
                default_class.push_str(if is_dir {
                    " pointer-events-auto"
                } else {
                    " pointer-events-none"
                });
                if matches!(layout, UploadFileItemLayout::List) && i + 1 != global_index {
                    default_class.push_str(" border-t-2")
                }
                default_class.push_str(" border-gray-200")
            }
            UploadDragDestination::Container => {
                default_class.push_str(if is_dir {
                    " pointer-events-auto"
                } else {
                    " pointer-events-none"
                });
                if matches!(layout, UploadFileItemLayout::List) {
                    default_class.push_str(" border-t-2")
                }
                default_class.push_str(" border-gray-200")
            }
        },
        None => {
            default_class.push_str(" pointer-events-auto");
            if matches!(layout, UploadFileItemLayout::List) {
                default_class.push_str(" border-t-2")
            }
            default_class.push_str(" border-gray-200")
        }
    }

    // states
    let nav = use_navigator();

    html! { <>
        <tr
            id={ format!("{id}-upload-{global_index}") }
            class={ default_class }
            draggable="true"
            onclick={ handle_onclick(nav, file) }
            ondragstart={ handle_ondragstart(file) }
            ondragenter={ handle_ondragenter(Some(global_index), is_dir, is_ready, drag_state) }
            ondragover={ handle_ondragover(Some(global_index), is_dir, is_ready, drag_state) }
            ondragleave={ handle_ondragleave(Some(global_index), is_dir, is_ready, drag_state) }
            ondrop={ handle_ondrop(file, Some(global_index), is_dir, is_ready, io, drag_state, onreload) }
        >
            // Placeholder
            <input
                id={ format!("{id}-upload") }
                type="file"
                class="hidden pointer-events-none"
                disabled=true
            />

            // Contents
            { children.clone() }
        </tr>
    </> }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub(super) struct UploadFileProps {
    pub(super) id: String,
    pub(super) dir_state: super::FileEntryState,
    pub(super) drag_state: UseUploadFileStateHandle,
    pub(super) file: FileRef,
    pub(super) i18n: DynI18n,
    pub(super) io: super::io::UseIOReducerHandle,
    pub(super) layout: UploadFileItemLayout,
    pub(super) onreload: Callback<()>,
    #[prop_or_default]
    pub(super) children: Html,
}

#[function_component(UploadFile)]
pub(super) fn render(props: &UploadFileProps) -> Html {
    // properties
    let &UploadFileProps {
        ref id,
        dir_state,
        ref drag_state,
        ref file,
        ref i18n,
        ref io,
        layout,
        ref onreload,
        ref children,
    } = props;

    let global_index = None;
    let is_ready = matches!(
        dir_state,
        super::FileEntryState::Directory | super::FileEntryState::Empty
    );
    let is_dir = true;

    html! {
        <div class="flex-1 stack w-full min-h-full select-none">
            // Overlay
            <div
                class={ format!(
                    "flex w-full h-full pointer-events-none items-end justify-center {}",
                    if drag_state.is_some() { "" } else { "hidden" },
                ) }
            >
                <div
                    class={ format!(
                        "flex flex-col items-center justify-center p-2 pl-8 pr-8 group backdrop-blur-sm transition-all {} {}",
                        if matches!(layout, UploadFileItemLayout::Grid) { "rounded-lg" } else { "" },
                        if drag_state.is_some() { "" } else { "hidden" },
                    ) }
                >
                    <svg
                        class="w-10 h-10 text-blue-400 group-hover:text-primary animate-bounce"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        // https://flowbite-svelte.com/docs/forms/file-input#dropzone
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
                    </svg>
                    <p class="mt-2 text-sm text-base-content">
                        <span class="font-semibold text-blue-600">{ i18n.indicator_drop_to_upload() }</span>
                    </p>
                </div>
            </div>

            // Contents
            <div
                id={ format!("{id}-container") }
                class={ format!(
                    "pointer-events-none border-2 border-dashed overflow-x-auto pb-4 {} {} {}",
                    if matches!(dir_state, super::FileEntryState::Directory | super::FileEntryState::Failed) {
                        "w-full h-fit self-start"
                    } else {
                        ""
                    },
                    if matches!(layout, UploadFileItemLayout::Grid) { "rounded-lg" } else { "" },
                    match **drag_state {
                        Some(s) if matches!(s.dst, UploadDragDestination::Container) => "border-blue-300",
                        Some(_) | None => "border-transparent",
                    },
                ) }
            >
                { children.clone() }
                { match dir_state {
                    super::FileEntryState::Directory | super::FileEntryState::Failed => html! {},
                    super::FileEntryState::Empty => html! { <Empty i18n={ i18n.clone() } /> },
                    super::FileEntryState::NotFound => html! { <FileNotFound i18n={ i18n.clone() } /> },
                } }
            </div>

            // Underlay
            <label
                id={ format!("{id}-upload") }
                class="w-full h-full"
                ondragenter={ handle_ondragenter(global_index, is_dir, is_ready, drag_state) }
                ondragover={ handle_ondragover(global_index, is_dir, is_ready, drag_state) }
                ondragleave={ handle_ondragleave(global_index, is_dir, is_ready, drag_state) }
                ondrop={ handle_ondrop(file, global_index, is_dir, is_ready, io, drag_state, onreload) }
            >
                // Placeholder
                <input
                    id={ format!("{id}-upload") }
                    type="file"
                    class="hidden pointer-events-none"
                    disabled=true
                />
            </label>
        </div>
    }
}
