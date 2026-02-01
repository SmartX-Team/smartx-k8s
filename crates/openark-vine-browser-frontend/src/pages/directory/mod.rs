mod grid;
mod io;
mod left_sidebar;
mod list;
mod mime;
mod navbar;
mod preview;
mod right_sidebar;
mod upload;

use std::{ops, rc::Rc};

use jiff::Timestamp;
use openark_vine_browser_api::{
    client::ClientExt,
    file::{FileEntry, FileMetadata, FileRef},
};
use web_sys::window;
use yew::{
    Callback, Html, Properties, Reducible, UseReducerHandle, UseStateHandle, function_component,
    html, use_reducer_eq, use_state_eq,
};

use crate::{
    net::{Client, HttpState, HttpStateRef, UseHttpHandleOption, UseHttpHandleOptionRender},
    router::RouteProps,
};

/// Parses the given [`HttpState`].
///
fn parse_file_entry(state: HttpState<FileEntry>) -> Rc<FileEntry> {
    /// Generates a dummy file entry.
    ///
    fn build_ref() -> FileRef {
        FileRef {
            name: "All files".into(),
            path: "/".into(),
            metadata: FileMetadata::default(),
        }
    }

    /// Generates an empty file entry.
    ///
    fn build_empty() -> FileEntry {
        FileEntry {
            r: build_ref(),
            files: vec![],
        }
    }

    /// Generates dummy samples for building skeletons.
    ///
    fn build_samples() -> FileEntry {
        FileEntry {
            r: build_ref(),
            files: vec![
                FileRef {
                    name: "My Project".into(),
                    path: "/My Project/".into(),
                    metadata: FileMetadata {
                        size: Some(12),
                        ..Default::default()
                    },
                },
                FileRef {
                    name: "My Image.jpg".into(),
                    path: "/My Image.jpg".into(),
                    metadata: FileMetadata {
                        size: Some(165378),
                        ..Default::default()
                    },
                },
                FileRef {
                    name: "My Document.pdf".into(),
                    path: "/My Document.pdf".into(),
                    metadata: FileMetadata {
                        size: Some(18810),
                        ..Default::default()
                    },
                },
            ],
        }
    }

    thread_local! {
        /// Samples for building skeletons.
        ///
        static EMPTY: Rc<FileEntry> = Rc::new(build_empty());

        /// Samples for building skeletons.
        ///
        static SAMPLES: Rc<FileEntry> = Rc::new(build_samples());
    }

    match state {
        HttpState::Pending => SAMPLES.with(|d| d.clone()),
        HttpState::NotFound | HttpState::Failed => EMPTY.with(|d| d.clone()),
        HttpState::Ready(value) => value,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FileEntryState {
    Directory,
    Empty,
    NotFound,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    pub path: String,
    pub route: RouteProps,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct FileIndices(Vec<usize>);

impl ops::Deref for FileIndices {
    type Target = [usize];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

#[derive(Clone, Debug)]
enum FileIndicesAction {
    Clear,
    Sorted(Vec<usize>),
}

impl Reducible for FileIndices {
    type Action = FileIndicesAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            FileIndicesAction::Clear => Default::default(),
            FileIndicesAction::Sorted(items) => Rc::new(Self(items)),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum ViewMode {
    Grid,
    List,
}

struct Context<'a> {
    props: &'a Props,
    current_timestamp: Timestamp,
    file_entry: UseHttpHandleOption<String, FileEntry>,
    indices: UseReducerHandle<FileIndices>,
    io: self::io::UseIOReducerHandle,
    reload: Callback<()>,
    selected_entry: UseStateHandle<Option<usize>>,
    state: FileEntryState,
    view_mode: UseStateHandle<ViewMode>,
}

fn render_file_entry_lookup(ctx: &Context) -> Html {
    // properties
    let current = ctx.current_timestamp;
    let file_entry = ctx.file_entry.clone();
    let i18n = &*ctx.props.route.i18n;

    html! {
        <div class="flex flex-1 flex-col overflow-hidden min-h-full">
            <div class="flex-1 overflow-y-scroll h-full">{{
                let key = &ctx.props.path;
                let path = ctx.props.path.clone();
                let fetch = |client: Client| async move {
                    client.get_file_entry(&path).await
                };
                let render = move |state| {
                    let dir_state = ctx.state;
                    let file_entry = parse_file_entry(state);
                    let i18n = &ctx.props.route.i18n;
                    let indices = &ctx.indices;
                    let io = &ctx.io;
                    let is_dir = file_entry.r.is_dir();
                    let onreload = &ctx.reload;
                    let selected = &ctx.selected_entry;
                    html! {
                        <div
                            class={ format!(
                                "flex flex-col w-full p-8 {}",
                                if !is_dir {
                                    "h-full"
                                } else if matches!(dir_state, FileEntryState::Failed) {
                                    ""
                                } else {
                                    "min-h-full"
                                }
                            ) }
                        >
                            // Navigation bar
                            { self::navbar::render(ctx) }

                            // File contents
                            {
                                if is_dir {
                                    match *ctx.view_mode {
                                        ViewMode::Grid => html! { <self::grid::FileList
                                            directory={ file_entry }
                                            i18n={ (**i18n).clone() }
                                            io={ io.clone() }
                                            onreload={ onreload.clone() }
                                            selected={ selected.clone() }
                                            state={ dir_state }
                                        /> },
                                        ViewMode::List => html! { <self::list::FileList
                                            { current }
                                            directory={ file_entry }
                                            i18n={ (**i18n).clone() }
                                            indices={ indices.clone() }
                                            io={ io.clone() }
                                            onreload={ onreload.clone() }
                                            selected={ selected.clone() }
                                            state={ dir_state }
                                        /> },
                                    }
                                } else {
                                    html! { <self::preview::Preview
                                        { file_entry }
                                        i18n={ (**i18n).clone() }
                                    /> }
                                }
                            }
                        </div>
                    }
                };
                file_entry.try_fetch_and_render(i18n, key, fetch, render)
            }}</div>
        </div>
    }
}

#[function_component(DirectoryPage)]
pub fn component(props: &Props) -> Html {
    // states
    let file_entry: UseHttpHandleOption<String, FileEntry> = use_state_eq(Default::default);
    let indices = use_reducer_eq(Default::default);
    let io: self::io::UseIOReducerHandle = use_reducer_eq(Default::default);
    let selected_entry = use_state_eq(Default::default);
    let state = match file_entry.try_get_state() {
        HttpStateRef::Pending => FileEntryState::Directory, // for building skeletons
        HttpStateRef::Ready(entry) => {
            if entry.files.is_empty() {
                FileEntryState::Empty
            } else {
                FileEntryState::Directory
            }
        }
        HttpStateRef::NotFound => FileEntryState::NotFound,
        HttpStateRef::Failed => FileEntryState::Failed,
    };
    let view_mode = use_state_eq(|| {
        if window()
            .and_then(|window| {
                Some((
                    window.inner_width().ok()?.as_f64()?,
                    window.inner_height().ok()?.as_f64()?,
                ))
            })
            // screen ratio
            .is_some_and(|(width, height)| width / height >= 0.8)
        {
            ViewMode::List
        } else {
            ViewMode::Grid
        }
    });

    // callbacks
    let reload = {
        let file_entry = file_entry.clone();
        Callback::from(move |()| file_entry.invalidate())
    };

    // context
    let ctx = Context {
        props,
        current_timestamp: Timestamp::now(),
        file_entry,
        indices,
        io,
        reload,
        selected_entry,
        state,
        view_mode,
    };

    html! {
        <>
            // Sidebar contents (left)
            { self::left_sidebar::render(&ctx) }

            // Directory lookup
            <div class="drawer-content flex overflow-hidden min-h-full">
                { render_file_entry_lookup(&ctx) }

                // Sidebar contents (right)
                { self::right_sidebar::render(&ctx) }
            </div>
        </>
    }
}
