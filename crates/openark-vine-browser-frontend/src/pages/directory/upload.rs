use web_sys::DataTransfer;
use yew::{
    Callback, DragEvent, Html, MouseEvent, Properties, UseStateHandle, function_component, html,
    html::IntoEventCallback,
};

use crate::widgets::{Empty, NotFound};

const DATA_TRANSFER_KIND_CONTAINER: &str = "string";
const DATA_TRANSFER_TYPE_CONTAINER: &str = concat!(env!("CARGO_CRATE_NAME"), "/file-entry");

const DATA_TRANSFER_KIND_FILE: &str = "file";

fn handle_ondragenter(
    dst: UploadDragDestination,
    global_index: Option<usize>,
    is_ready: bool,
    drag_disabled: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_ready || drag_disabled {
            return;
        }
        if let Some((new_state, _)) = UploadDragState::parse(&event, global_index, dst) {
            last_state.set(Some(new_state))
        }
    }
}

fn handle_ondragover(
    dst: UploadDragDestination,
    global_index: Option<usize>,
    is_ready: bool,
    drag_disabled: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_ready || drag_disabled {
            return;
        }
        if is_ready && let Some((new_state, _)) = UploadDragState::parse(&event, global_index, dst)
        {
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
    dst: UploadDragDestination,
    is_ready: bool,
    drag_disabled: bool,
    last_state: &UseUploadFileStateHandle,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    move |event: DragEvent| {
        if !is_ready || drag_disabled {
            return;
        }
        if matches!(dst, UploadDragDestination::Container) && last_state.is_some() {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
            last_state.set(None)
        }
    }
}

fn handle_ondrop(
    dst: UploadDragDestination,
    global_index: Option<usize>,
    is_ready: bool,
    drag_disabled: bool,
    last_state: &UseUploadFileStateHandle,
    ondrop: &Callback<UploadDragEvent, ()>,
) -> impl IntoEventCallback<DragEvent> {
    let last_state = last_state.clone();
    let ondrop = ondrop.clone();
    move |event: DragEvent| {
        if !is_ready || drag_disabled {
            return;
        }
        if let Some((new_state, dt)) = UploadDragState::parse(&event, global_index, dst) {
            event.prevent_default(); // Consume the event
            event.stop_propagation(); // Do not propagate the event
            last_state.set(None);
            ondrop.emit(UploadDragEvent {
                dt,
                state: new_state,
            })
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
    SingleDirectory,
    MultiFiles,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct UploadDragState {
    src: UploadDragSource,
    dst: UploadDragDestination,
}

impl UploadDragState {
    fn parse(
        event: &DragEvent,
        global_index: Option<usize>,
        dst: UploadDragDestination,
    ) -> Option<(Self, DataTransfer)> {
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

        let this = Self { src: src?, dst };
        Some((this, dt))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct UploadDragEvent {
    pub(super) dt: DataTransfer,
    pub(super) state: UploadDragState,
}

pub(super) type UseUploadFileStateHandle = UseStateHandle<Option<UploadDragState>>;

#[derive(Clone, Debug, PartialEq, Properties)]
pub(super) struct UploadFileProps {
    pub(super) id: String,
    pub(super) dir_state: super::FileEntryState,
    pub(super) drag_state: UseUploadFileStateHandle,
    pub(super) layout: UploadFileItemLayout,
    pub(super) ondrop: Callback<UploadDragEvent, ()>,
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
        layout,
        ref ondrop,
        ref children,
    } = props;

    let dst = UploadDragDestination::Container;
    let drag_disabled = false;
    let global_index = None;
    let is_ready = matches!(
        dir_state,
        super::FileEntryState::Directory | super::FileEntryState::Empty
    );

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
                        <span class="font-semibold text-blue-600">{ "Drop to upload" }</span>
                    </p>
                </div>
            </div>

            // Contents
            <div
                id={ format!("{id}-container") }
                class={ format!(
                    "pointer-events-none border-2 border-dashed overflow-x-auto pb-4 {} {} {}",
                    if matches!(dir_state, super::FileEntryState::Directory | super::FileEntryState::Failed) {
                        "w-fit h-fit self-start"
                    } else {
                        ""
                    },
                    if matches!(layout, UploadFileItemLayout::Grid) { "rounded-lg" } else { "" },
                    match **drag_state {
                        Some(s) if s.dst == dst => "border-blue-300",
                        Some(_) | None => "border-transparent",
                    },
                ) }
            >
                { children.clone() }
                { match dir_state {
                    super::FileEntryState::Directory | super::FileEntryState::Failed => html! {},
                    super::FileEntryState::Empty => html! { <Empty /> },
                    super::FileEntryState::NotFound => html! { <NotFound /> },
                } }
            </div>

            // Underlay
            <label
                id={ format!("{id}-upload") }
                class="w-full h-full"
                ondragenter={ handle_ondragenter(dst, global_index, is_ready, drag_disabled, drag_state) }
                ondragover={ handle_ondragover(dst, global_index, is_ready, drag_disabled, drag_state) }
                ondragleave={ handle_ondragleave(dst, is_ready, drag_disabled, drag_state) }
                ondrop={ handle_ondrop(dst, global_index, is_ready, drag_disabled, drag_state, ondrop) }
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
    pub(super) ptr: UploadFileItemPtr,
    pub(super) dir_state: super::FileEntryState,
    #[prop_or_default]
    pub(super) drag_disabled: bool,
    pub(super) drag_state: UseUploadFileStateHandle,
    pub(super) layout: UploadFileItemLayout,
    #[prop_or_default]
    pub(super) onclick: Option<Callback<MouseEvent, ()>>,
    pub(super) ondrop: Callback<UploadDragEvent, ()>,
    #[prop_or_default]
    pub(super) children: Html,
}

#[function_component(UploadFileItem)]
pub(super) fn render(props: &UploadFileItemProps) -> Html {
    // properties
    let &UploadFileItemProps {
        ref id,
        dir_state,
        drag_disabled,
        ref drag_state,
        layout,
        ref onclick,
        ref ondrop,
        ptr:
            UploadFileItemPtr {
                global_index,
                local_index,
                local_size,
            },
        ref children,
    } = props;

    let dst = UploadDragDestination::Item(global_index);
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
                default_class.push_str(if drag_disabled {
                    " pointer-events-none"
                } else {
                    " pointer-events-auto"
                });
                if matches!(layout, UploadFileItemLayout::List) && i + 1 != global_index {
                    default_class.push_str(" border-t-2")
                }
                default_class.push_str(" border-gray-200")
            }
            UploadDragDestination::Container => {
                default_class.push_str(if drag_disabled {
                    " pointer-events-none"
                } else {
                    " pointer-events-auto"
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

    html! { <>
        <tr
            id={ format!("{id}-upload-{global_index}") }
            class={ default_class }
            onclick={{
                let onclick = onclick.clone();
                move |event: MouseEvent| {
                    if let Some(onclick) = onclick.as_ref() {
                        onclick.emit(event)
                    }
                }
            }}
            ondragstart={
                move |event: DragEvent| {
                    if let Some(dt) = event.data_transfer() {
                        let items = dt.items();
                        let _ = items.add_with_str_and_type(
                            &global_index.to_string(),
                            DATA_TRANSFER_TYPE_CONTAINER,
                        );
                    }
                }
            }
            ondragenter={ handle_ondragenter(dst, Some(global_index), is_ready, drag_disabled, drag_state) }
            ondragover={ handle_ondragover(dst, Some(global_index), is_ready, drag_disabled, drag_state) }
            ondragleave={ handle_ondragleave(dst, is_ready, drag_disabled, drag_state) }
            ondrop={ handle_ondrop(dst, Some(global_index), is_ready, drag_disabled, drag_state, ondrop) }
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
