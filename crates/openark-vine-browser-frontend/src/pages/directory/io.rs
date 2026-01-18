use std::{
    collections::VecDeque,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher},
    rc::Rc,
};

use chrono::{DateTime, Utc};
use openark_vine_browser_api::file::FileRef;
use web_sys::wasm_bindgen::{JsCast, prelude::Closure};
use yew::{
    Callback, Html, MouseEvent, Properties, Reducible, UseReducerHandle, function_component, html,
};

use crate::{i18n::DynI18n, net::get_file_content_url};

const MAX_CONCURRENT_TASKS: usize = 8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum IOKind {
    Download,
    Upload,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct IOTaskRef {
    pub timestamp: DateTime<Utc>,
    pub kind: IOKind,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
enum IOTaskResult {
    Pending {
        onstart: Callback<()>,
        oncancel: Callback<()>,
    },
    Running {
        oncancel: Callback<()>,
    },
    Completed,
    Failed {
        error: String,
    },
}

impl From<Result<(), String>> for IOTaskResult {
    fn from(value: Result<(), String>) -> Self {
        match value {
            Ok(()) => Self::Completed,
            Err(error) => Self::Failed { error },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct IOTask {
    r: Rc<IOTaskRef>,
    current: u64,
    total: u64,
    result: IOTaskResult,
}

impl IOTask {
    fn cancel(self) {
        match self.result {
            IOTaskResult::Pending { oncancel, .. } | IOTaskResult::Running { oncancel } => {
                oncancel.emit(())
            }
            _ => (),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
struct FileProps {
    index: usize,
    state: UseIOReducerHandle,
}

#[function_component(FileIO)]
fn render_file_io(props: &FileProps) -> Html {
    // properties
    let &FileProps { index, ref state } = props;
    let &IOTask {
        ref r,
        current,
        total,
        ref result,
    } = &state.queue[index];

    let hash = BuildHasherDefault::<DefaultHasher>::new().hash_one(Rc::as_ptr(&r));
    let &IOTaskRef {
        timestamp: _,
        kind,
        ref path,
    } = &**r;

    let (color, error_message, symbol) = match result {
        IOTaskResult::Pending { .. } => {
            let color = "gray";
            let error_message = None;
            let symbol = html! {
                // heroicons:ellipsis-horizontal-circle:micro
                <path fill-rule="evenodd" d="M15 8A7 7 0 1 1 1 8a7 7 0 0 1 14 0ZM8 9a1 1 0 1 0 0-2 1 1 0 0 0 0 2ZM5.5 8a1 1 0 1 1-2 0 1 1 0 0 1 2 0Zm6 1a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" clip-rule="evenodd" />
            };
            (color, error_message, symbol)
        }
        IOTaskResult::Running { .. } => match kind {
            IOKind::Download => {
                let color = "purple";
                let error_message = None;
                let symbol = html! {
                    // heroicons:arrow-down-circle:micro
                    <path fill-rule="evenodd" d="M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm.75-10.25a.75.75 0 0 0-1.5 0v4.69L6.03 8.22a.75.75 0 0 0-1.06 1.06l2.5 2.5a.75.75 0 0 0 1.06 0l2.5-2.5a.75.75 0 1 0-1.06-1.06L8.75 9.44V4.75Z" clip-rule="evenodd" />
                };
                (color, error_message, symbol)
            }
            IOKind::Upload => {
                let color = "yellow";
                let error_message = None;
                let symbol = html! {
                    // heroicons:arrow-up-circle:micro
                    <path fill-rule="evenodd" d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1Zm-.75 10.25a.75.75 0 0 0 1.5 0V6.56l1.22 1.22a.75.75 0 1 0 1.06-1.06l-2.5-2.5a.75.75 0 0 0-1.06 0l-2.5 2.5a.75.75 0 0 0 1.06 1.06l1.22-1.22v4.69Z" clip-rule="evenodd" />
                };
                (color, error_message, symbol)
            }
        },
        IOTaskResult::Completed => {
            let color = "green";
            let error_message = None;
            let symbol = html! {
                // heroicons:check-circle:micro
                <path fill-rule="evenodd" d="M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm3.844-8.791a.75.75 0 0 0-1.188-.918l-3.7 4.79-1.649-1.833a.75.75 0 1 0-1.114 1.004l2.25 2.5a.75.75 0 0 0 1.15-.043l4.25-5.5Z" clip-rule="evenodd" />
            };
            (color, error_message, symbol)
        }
        IOTaskResult::Failed { error } => {
            let color = "red";
            let error_message = Some(error);
            let symbol = html! {
                // heroicons:exclamation-circle:micro
                <path fill-rule="evenodd" d="M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14ZM8 4a.75.75 0 0 1 .75.75v3a.75.75 0 0 1-1.5 0v-3A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" clip-rule="evenodd" />
            };
            (color, error_message, symbol)
        }
    };
    let percent = if total > 0 {
        100.0 * ((current as f32) / (total as f32)).min(1.0)
    } else {
        0.0
    };

    html! {
        <li
            class="pointer-events-none transition-all"
            key={hash}
        >
            // Divider
            <p class="flex">
                <hr class="flex-1 border-gray-300" />
            </p>
            // Path
            <p class="flex items-center">
                <a
                    class={ format!("text-{color}-500") }
                >
                    <svg
                        class="w-5 h-5"
                        fill="currentColor"
                        viewBox="0 0 16 16"
                        stroke="currentColor"
                        stroke-width="0.5"
                    >{ symbol }</svg>
                </a>
                <span
                    class="flex-1 font-bold tooltip truncate"
                    data-tip={ path.clone() }
                >{ path.clone() }</span>
                <button
                    class="cursor-pointer pointer-events-auto text-gray-600"
                    onclick={{
                        let state = state.clone();
                        move |event: MouseEvent| {
                            event.prevent_default(); // Consume the event
                            event.stop_propagation(); // Do not propagate the event
                            let action = IOAction::Cancel { index };
                            state.dispatch(action)
                        }
                    }}
                >
                    <svg
                        class="w-5 h-5 text-gray-500"
                        fill="currentColor"
                        viewBox="0 0 16 16"
                        stroke="currentColor"
                        stroke-width="0.5"
                    >
                        // heroicons:x-circle:micro
                        <path fill-rule="evenodd" d="M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14Zm2.78-4.22a.75.75 0 0 1-1.06 0L8 9.06l-1.72 1.72a.75.75 0 1 1-1.06-1.06L6.94 8 5.22 6.28a.75.75 0 0 1 1.06-1.06L8 6.94l1.72-1.72a.75.75 0 1 1 1.06 1.06L9.06 8l1.72 1.72a.75.75 0 0 1 0 1.06Z" clip-rule="evenodd" />
                    </svg>
                </button>
            </p>
            // Progress bar
            <p class="flex items-center">
                <div class={ format!("flex-1 bg-{color}-100 rounded-full h-1.5") }>
                    <div
                        class={ format!("bg-{color}-400 h-1.5 rounded-full w-full") }
                        style={ format!("width: {percent:.0}%") }
                    />
                </div>
                <span class="ml-1 text-gray-600">{ format!("{percent:.0}%") }</span>
            </p>
            // Error message
            <p
                class={ format!(
                    "text-sm text-{color}-500 {}",
                    if error_message.is_some() { "" } else { "hidden" }
                ) }
            >
                <span>{ "Error: " }</span>
                { for error_message }
            </p>
        </li>
    }
}

#[derive(Clone, Debug)]
pub(super) enum IOAction {
    Enqueue {
        r: Rc<IOTaskRef>,
        onstart: Callback<()>,
        oncancel: Callback<()>,
        total: Option<u64>,
    },
    EnqueueAsCompleted {
        r: Rc<IOTaskRef>,
        total: Option<u64>,
        result: Result<(), String>,
    },
    Cancel {
        index: usize,
    },
    CancelAll,
    Progress {
        r: Rc<IOTaskRef>,
        current: u64,
        total: u64,
    },
    Complete {
        r: Rc<IOTaskRef>,
        result: Result<(), String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct IOState {
    max_concurrent_task: usize,
    queue: VecDeque<IOTask>,
}

impl IOState {
    fn get(&mut self, r: &Rc<IOTaskRef>) -> Option<&mut IOTask> {
        self.queue.iter_mut().find(|task| Rc::ptr_eq(&task.r, r))
    }

    /// Spawns a pending task.
    ///
    fn schedule(&mut self) {
        let num_running = self
            .queue
            .iter()
            .filter(|&task| matches!(task.result, IOTaskResult::Running { .. }))
            .count();

        if num_running < self.max_concurrent_task {
            let mut num_pending = self.max_concurrent_task - num_running;
            for task in &mut self.queue {
                match &task.result {
                    IOTaskResult::Pending { onstart, oncancel } => {
                        let onstart = onstart.clone();
                        task.result = IOTaskResult::Running {
                            oncancel: oncancel.clone(),
                        };
                        onstart.emit(());

                        num_pending -= 1;
                        if num_pending == 0 {
                            break;
                        }
                    }
                    _ => continue,
                };
            }
        }
    }
}

impl Default for IOState {
    fn default() -> Self {
        Self {
            max_concurrent_task: MAX_CONCURRENT_TASKS,
            queue: Default::default(),
        }
    }
}

impl Reducible for IOState {
    type Action = IOAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut this = (*self).clone();
        match action {
            IOAction::Enqueue {
                r,
                onstart,
                oncancel,
                total,
            } => {
                let total = total.unwrap_or_default();
                this.queue.push_back(IOTask {
                    r,
                    current: 0,
                    total,
                    result: IOTaskResult::Pending { onstart, oncancel },
                });
                this.schedule();
            }
            IOAction::EnqueueAsCompleted { r, total, result } => {
                let total = total.unwrap_or_default();
                this.queue.push_back(IOTask {
                    r,
                    current: total,
                    total,
                    result: result.into(),
                })
            }
            IOAction::Cancel { index } => {
                if let Some(item) = this.queue.remove(index) {
                    item.cancel()
                }
                this.schedule();
            }
            IOAction::CancelAll => {
                for item in this.queue.drain(..) {
                    item.cancel()
                }
            }
            IOAction::Progress { r, current, total } => {
                if let Some(task) = this.get(&r) {
                    task.current = current;
                    task.total = total;
                }
            }
            IOAction::Complete { r, result } => {
                if let Some(task) = this.get(&r) {
                    task.result = result.into();
                }
                this.schedule();
            }
        }
        Rc::new(this)
    }
}

pub(super) type UseIOReducerHandle = UseReducerHandle<IOState>;

pub(super) trait UseIOReducerHandleExt {
    fn download_file(&self, src: &FileRef);

    fn download_directory(&self, src: &FileRef);

    fn r#move(&self, src: FileRef, dst: &FileRef, oncomplete: Callback<()>);

    fn upload_file(&self, src: ::web_sys::File, dst: &FileRef, oncomplete: Callback<()>);
}

impl UseIOReducerHandleExt for UseIOReducerHandle {
    fn download_file(&self, src: &FileRef) {
        // Create an item
        let r = Rc::new(IOTaskRef {
            timestamp: Utc::now(),
            kind: IOKind::Download,
            path: src.path.clone(),
        });
        let total = src.metadata.size;

        // Create a full URL
        let url = match get_file_content_url(&r.path) {
            Ok(mut url) => {
                url.set_query(Some("download=true"));
                url.to_string()
            }
            Err(error) => {
                return self.dispatch(IOAction::EnqueueAsCompleted {
                    r,
                    total,
                    result: Err(error.to_string()),
                });
            }
        };

        // Create a virtual <a> tag
        let window = ::web_sys::window().unwrap();
        let document = window.document().unwrap();
        let link = document
            .create_element("a")
            .unwrap()
            .dyn_into::<::web_sys::HtmlAnchorElement>()
            .unwrap();

        // Invoke a click event
        link.set_download(r.path.split('/').last().unwrap_or_default());
        link.set_href(&url);
        link.set_rel("noopener noreferrer");
        link.set_target("_blank");
        link.click();
        link.remove();

        // Regard as completed
        self.dispatch(IOAction::EnqueueAsCompleted {
            r,
            total,
            result: Ok(()),
        })
    }

    fn download_directory(&self, src: &FileRef) {
        // TODO: To be implemented!
        tracing::info!("downloadDirectory: src={src:#?}");
    }

    fn r#move(&self, src: FileRef, dst: &FileRef, oncomplete: Callback<()>) {
        // TODO: To be implemented!
        tracing::info!("move: src={src:#?}, dst={dst:#?}");
        let _ = oncomplete;
    }

    fn upload_file(&self, src: ::web_sys::File, dst: &FileRef, oncomplete: Callback<()>) {
        // Enqueue an item
        let r = Rc::new(IOTaskRef {
            timestamp: Utc::now(),
            kind: IOKind::Upload,
            path: dst.path.clone(),
        });
        let total = dst.metadata.size;

        // Create a full URL
        let url = match get_file_content_url(&r.path) {
            Ok(url) => url,
            Err(error) => {
                return self.dispatch(IOAction::EnqueueAsCompleted {
                    r,
                    total,
                    result: Err(error.to_string()),
                });
            }
        };

        // Open the API URL
        let xhr = ::web_sys::XmlHttpRequest::new().unwrap();
        xhr.open("POST", &url.to_string()).unwrap();

        // Add a hook: onprogress
        let onprogress = {
            let closure = {
                let this = self.clone();
                let r = r.clone();
                move |event: ::web_sys::ProgressEvent| {
                    if event.length_computable() {
                        this.dispatch(IOAction::Progress {
                            r: r.clone(),
                            current: event.loaded() as _,
                            total: event.total() as _,
                        })
                    }
                }
            };
            let callback = Closure::wrap(Box::new(closure) as Box<dyn FnMut(_)>);

            xhr.upload()
                .unwrap()
                .set_onprogress(Some(callback.as_ref().unchecked_ref()));
            callback
        };

        // Add a hook: onload
        {
            let closure = {
                let this = self.clone();
                let r = r.clone();
                let xhr = xhr.clone();
                move |_: ::web_sys::Event| {
                    drop(onprogress);
                    let result = match xhr.status() {
                        Ok(200..300) => {
                            oncomplete.emit(());
                            Ok(())
                        }
                        Ok(_) | Err(_) => Err(xhr
                            .status_text()
                            .unwrap_or_else(|_| "Failed to fetch".into())),
                    };
                    this.dispatch(IOAction::Complete { r, result })
                }
            };
            let callback = Closure::once(Box::new(closure) as Box<dyn FnOnce(_)>);

            xhr.set_onload(Some(callback.as_ref().unchecked_ref()));
            callback.forget()
        }

        // Attach the data
        let form_data = ::web_sys::FormData::new().unwrap();
        form_data.append_with_blob(&src.name(), &src).unwrap();

        // Add hook: Send the request
        let onstart = {
            let xhr = xhr.clone();
            move |()| xhr.send_with_opt_form_data(Some(&form_data)).unwrap()
        };

        // Add hook: Cancel the request
        let oncancel = {
            let xhr = xhr.clone();
            move |()| {
                let _ = xhr.abort();
            }
        };

        // Regard as pending
        self.dispatch(IOAction::Enqueue {
            r,
            onstart: Callback::from(onstart),
            oncancel: Callback::from(oncancel),
            total,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub(super) struct Props {
    pub(super) i18n: DynI18n,
    pub(super) io: UseIOReducerHandle,
}

#[function_component(IOStatus)]
pub(super) fn render(props: &Props) -> Html {
    // properties
    let &Props { ref i18n, ref io } = props;

    html! {
        <div class="indicator">
            <span
                class={ format!(
                    "indicator-item badge badge-secondary badge-sm truncate pointer-event-none select-none right-0 translate-x-1/2 {}",
                    if io.queue.is_empty() { "hidden" } else { "" }
                ) }
            >{ io.queue.len() }</span>
            <details class="dropdown dropdown-end bg-none text-gray-400 hover:text-gray-600">
                <summary
                    class="cursor-pointer tooltip p-2 mt-2 h-fit rounded-lg transition-colors bg-blue-100 hover:bg-blue-200 active:bg-blue-300 text-blue-500 hover:text-blue-400"
                    data-tip={ i18n.indicator_io_status() }
                >
                    <svg
                        class="h-5 w-5"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        viewBox="0 0 24 24"
                    >
                        // heroicons:arrows-up-down:outline
                        <path stroke-linecap="round" stroke-linejoin="round" d="M3 7.5 7.5 3m0 0L12 7.5M7.5 3v13.5m13.5 0L16.5 21m0 0L12 16.5m4.5 4.5V7.5" />
                    </svg>
                </summary>

                <ul class="dropdown-content z-10 menu cursor-default pointer-events-auto transition-all py-4 border border-gray-300 shadow-xl bg-gray-100 text-gray-700 rounded-sm w-90 mt-4">
                    // header
                    <li
                        class="pointer-events-none transition-colors"
                        key="header" // not a hash
                    >
                        <p class="flex items-center justify-between">
                            <h2 class="font-bold">{ i18n.indicator_downloads_uploads() }</h2>
                            <button
                                class="cursor-pointer pointer-events-auto text-gray-600"
                                onclick={{
                                    let state = io.clone();
                                    move |event: MouseEvent| {
                                        event.prevent_default(); // Consume the event
                                        event.stop_propagation(); // Do not propagate the event
                                        let action = IOAction::CancelAll;
                                        state.dispatch(action)
                                    }
                                }}
                            >
                                <svg
                                    class="w-5 h-5"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                >
                                    // heroicons:backspace:outline
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 9.75 14.25 12m0 0 2.25 2.25M14.25 12l2.25-2.25M14.25 12 12 14.25m-2.58 4.92-6.374-6.375a1.125 1.125 0 0 1 0-1.59L9.42 4.83c.21-.211.497-.33.795-.33H19.5a2.25 2.25 0 0 1 2.25 2.25v10.5a2.25 2.25 0 0 1-2.25 2.25h-9.284c-.298 0-.585-.119-.795-.33Z" />
                                </svg>
                            </button>
                        </p>
                    </li>
                    // body
                    { for (0..io.queue.len()).map(|index| html! { <FileIO
                        { index }
                        state={ io.clone() }
                    /> }) }
                </ul>

                // Placeholders
                <div class="hidden">
                    <div class="bg-gray-100" />
                    <div class="bg-gray-400" />
                    <div class="text-gray-500" />
                    <div class="bg-red-100" />
                    <div class="bg-red-400" />
                    <div class="text-red-500" />
                    <div class="bg-yellow-100" />
                    <div class="bg-yellow-400" />
                    <div class="text-yellow-500" />
                    <div class="bg-green-100" />
                    <div class="bg-green-400" />
                    <div class="text-green-500" />
                    <div class="bg-blue-100" />
                    <div class="bg-blue-400" />
                    <div class="text-blue-500" />
                    <div class="bg-purple-100" />
                    <div class="bg-purple-400" />
                    <div class="text-purple-500" />
                </div>
            </details>
        </div>
    }
}
