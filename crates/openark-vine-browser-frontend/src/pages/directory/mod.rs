mod grid;
mod io;
mod list;
mod mime;
mod navbar;
mod preview;
mod sidebar;
mod upload;

use std::rc::Rc;

use chrono::{DateTime, Utc};
use openark_vine_browser_api::{
    client::ClientExt,
    file::{FileEntry, FileMetadata, FileRef},
    file_type::{DocumentType, FileType, ImageType},
};
use web_sys::window;
use yew::{
    Html, Properties, UseStateHandle, function_component, html, use_reducer_eq, use_state_eq,
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
                        ty: Some(FileType::Image(ImageType::Jpeg)),
                        ..Default::default()
                    },
                },
                FileRef {
                    name: "My Document.pdf".into(),
                    path: "/My Document.pdf".into(),
                    metadata: FileMetadata {
                        size: Some(18810),
                        ty: Some(FileType::Document(DocumentType::Pdf)),
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

#[derive(Copy, Clone, PartialEq, Eq)]
enum ViewMode {
    Grid,
    List,
}

struct Context<'a> {
    props: &'a Props,
    current_timestamp: DateTime<Utc>,
    file_entry: UseHttpHandleOption<String, FileEntry>,
    io: self::io::UseIOReducerHandle,
    state: FileEntryState,
    view_mode: UseStateHandle<ViewMode>,
}

fn render_file_entry_lookup(ctx: Context) -> Html {
    // properties
    let current = ctx.current_timestamp;
    let file_entry = ctx.file_entry.clone();
    let i18n = &*ctx.props.route.i18n;

    html! {
        <div class="drawer-content flex flex-col overflow-hidden min-h-full">
            <div class="flex-1 overflow-auto h-full">{{
                let key = &ctx.props.path;
                let path = ctx.props.path.clone();
                let fetch = |client: Client| async move {
                    client.get_file_entry(&path).await
                };
                let render = move |state| {
                    let dir_state = ctx.state;
                    let file_entry = parse_file_entry(state);
                    let i18n = &ctx.props.route.i18n;
                    let is_dir = file_entry.r.is_dir();
                    html! {
                        <div
                            class={ format!(
                                "flex flex-col w-full p-8 {}",
                                if matches!(dir_state, FileEntryState::Failed) {
                                    ""
                                } else {
                                    "min-h-full"
                                }
                            ) }
                        >
                            // Navigation bar
                            { self::navbar::render(&ctx) }

                            // File contents
                            {
                                if is_dir {
                                    match *ctx.view_mode {
                                        ViewMode::Grid => html! { <self::grid::FileList
                                            directory={ file_entry }
                                            i18n={ (**i18n).clone() }
                                            state={ dir_state }
                                        /> },
                                        ViewMode::List => html! { <self::list::FileList
                                            { current }
                                            directory={ file_entry }
                                            i18n={ (**i18n).clone() }
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
    let io: self::io::UseIOReducerHandle = use_reducer_eq(Default::default);
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

    // context
    let ctx = Context {
        props,
        current_timestamp: Utc::now(),
        file_entry,
        io,
        state,
        view_mode,
    };

    html! {
        <>
            // Sidebar contents (left)
            { self::sidebar::render(&ctx) }

            // Directory lookup
            { render_file_entry_lookup(ctx) }
        </>
    }
}
