use web_sys::DataTransfer;
use yew::{
    Callback, DragEvent, Html, MouseEvent, Properties, UseStateHandle, function_component, html,
};

fn is_data_transfer_file(dt: &DataTransfer) -> bool {
    let items = dt.items();
    (0..items.length()).any(|index| items.get(index).is_some_and(|item| item.kind() == "file"))
}

pub type UseUploadFileStateHandle = UseStateHandle<Option<UploadDragState>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UploadDragState {
    Container,
    Item(usize),
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct UploadFileProps {
    pub id: String,
    pub drag_state: UseUploadFileStateHandle,
    pub layout: UploadFileItemLayout,
    pub ondrop: Callback<DataTransfer, ()>,
    #[prop_or_default]
    pub children: Html,
}

#[function_component(UploadFile)]
pub fn render(props: &UploadFileProps) -> Html {
    // properties
    let &UploadFileProps {
        ref id,
        ref drag_state,
        layout,
        ref ondrop,
        ref children,
    } = props;

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
                    "pointer-events-none border-2 border-dashed w-fit h-fit self-start overflow-x-auto pb-4 {} {}",
                    if matches!(layout, UploadFileItemLayout::Grid) { "rounded-lg" } else { "" },
                    match **drag_state {
                        Some(UploadDragState::Container) => "border-blue-300",
                        Some(_) | None => "border-transparent",
                    },
                ) }
            >{
                children.clone()
            }</div>

            // Underlay
            <label
                id={ format!("{id}-upload") }
                class="w-full h-full"
                ondragenter={{
                    let drag_state = drag_state.clone();
                    move |event: DragEvent| {
                        if let Some(dt) = event.data_transfer() && is_data_transfer_file(&dt) {
                            event.prevent_default(); // Consume the event
                            event.stop_propagation(); // Do not propagate the event
                            drag_state.set(Some(UploadDragState::Container))
                        }
                    }
                }}
                ondragover={{
                    let drag_state = drag_state.clone();
                    move |event: DragEvent| {
                        if drag_state.is_some() || event.data_transfer().is_some_and(|dt| is_data_transfer_file(&dt))
                        {
                            event.prevent_default(); // Consume the event
                            event.stop_propagation(); // Do not propagate the event
                            drag_state.set(Some(UploadDragState::Container))
                        }
                    }
                }}
                ondragleave={{
                    let drag_state = drag_state.clone();
                    move |event: DragEvent| {
                        if drag_state.is_some() {
                            event.prevent_default(); // Consume the event
                            event.stop_propagation(); // Do not propagate the event
                            drag_state.set(None)
                        }
                    }
                }}
                ondrop={{
                    let drag_state = drag_state.clone();
                    let ondrop = ondrop.clone();
                    move |event: DragEvent| {
                        if *drag_state == Some(UploadDragState::Container)
                            && let Some(dt) = event.data_transfer()
                        {
                            event.prevent_default(); // Consume the event
                            event.stop_propagation(); // Do not propagate the event
                            drag_state.set(None);
                            ondrop.emit(dt)
                        }
                    }
                }}
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
pub enum UploadFileItemLayout {
    Grid,
    List,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UploadFileItemPtr {
    pub global_index: usize,
    pub local_index: usize,
    pub local_size: usize,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct UploadFileItemProps {
    pub id: String,
    pub ptr: UploadFileItemPtr,
    #[prop_or_default]
    pub drag_disabled: bool,
    pub drag_state: UseUploadFileStateHandle,
    pub layout: UploadFileItemLayout,
    #[prop_or_default]
    pub onclick: Option<Callback<MouseEvent, ()>>,
    pub ondrop: Callback<DataTransfer, ()>,
    #[prop_or_default]
    pub children: Html,
}

#[function_component(UploadFileItem)]
pub fn render(props: &UploadFileItemProps) -> Html {
    // properties
    let &UploadFileItemProps {
        ref id,
        ptr:
            UploadFileItemPtr {
                global_index,
                local_index,
                local_size,
            },
        drag_disabled,
        ref drag_state,
        layout,
        ref onclick,
        ref ondrop,
        ref children,
    } = props;

    let mut default_class = String::from(
        "group border-l-2 border-r-2 cursor-pointer hover:shadow-lg transition-colors",
    );
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
        Some(UploadDragState::Item(i)) if i == global_index => {
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
        Some(UploadDragState::Item(i)) => {
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
        Some(UploadDragState::Container) => {
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
            ondragenter={{
                let drag_state = drag_state.clone();
                move |event: DragEvent| {
                    if let Some(dt) = event.data_transfer() && is_data_transfer_file(&dt) {
                        event.prevent_default(); // Consume the event
                        event.stop_propagation(); // Do not propagate the event
                        drag_state.set(Some(if drag_disabled {
                            UploadDragState::Container
                        } else {
                            UploadDragState::Item(global_index)
                        }))
                    }
                }
            }}
            ondragover={{
                let drag_state = drag_state.clone();
                move |event: DragEvent| {
                    if drag_state.is_some() || event.data_transfer().is_some_and(|dt| is_data_transfer_file(&dt))
                    {
                        event.prevent_default(); // Consume the event
                        event.stop_propagation(); // Do not propagate the event
                        drag_state.set(Some(if drag_disabled {
                            UploadDragState::Container
                        } else {
                            UploadDragState::Item(global_index)
                        }))
                    }
                }
            }}
            ondragleave={{
                let drag_state = drag_state.clone();
                move |event: DragEvent| {
                    if drag_state.is_some() {
                        event.prevent_default(); // Consume the event
                        event.stop_propagation(); // Do not propagate the event
                        drag_state.set(None)
                    }
                }
            }}
            ondrop={{
                let drag_state = drag_state.clone();
                let ondrop = ondrop.clone();
                move |event: DragEvent| {
                    if !drag_disabled && *drag_state == Some(UploadDragState::Item(global_index))
                        && let Some(dt) = event.data_transfer()
                    {
                        event.prevent_default(); // Consume the event
                        event.stop_propagation(); // Do not propagate the event
                        drag_state.set(None);
                        ondrop.emit(dt)
                    }
                }
            }}
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
