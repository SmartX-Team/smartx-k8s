mod grid;
mod list;
mod mime;
mod navbar;
mod preview;
mod sidebar;
mod utils;

use std::rc::Rc;

use chrono::{DateTime, Utc};
use openark_vine_browser_api::{
    client::ClientExt,
    file::{FileEntry, FileMetadata, FileRef},
    file_type::{DocumentType, FileType, ImageType},
};
use yew::{Html, Properties, UseStateHandle, function_component, html, use_state_eq};

use crate::{
    net::{Client, HttpState, UseHttpHandleOption, UseHttpHandleOptionRender},
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

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    pub path: String,
    pub route: RouteProps,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
enum ViewMode {
    #[default]
    Grid,
    List,
}

struct Context<'a> {
    props: &'a Props,
    current_timestamp: DateTime<Utc>,
    file_entry: UseHttpHandleOption<String, FileEntry>,
    view_mode: UseStateHandle<ViewMode>,
}

fn draw_file_entry_lookup(ctx: Context) -> Html {
    // properties
    let current = ctx.current_timestamp;
    let file_entry = ctx.file_entry.clone();

    html! {
        <div class="drawer-content flex flex-col overflow-hidden min-h-full">
            <div class="flex-1 overflow-auto h-full">{{
                let key = &ctx.props.path;
                let path = ctx.props.path.clone();
                let fetch = |client: Client| async move {
                    client.get_file_entry(&path).await
                };
                let render = move |state| {
                    let file_entry = parse_file_entry(state);
                    let is_dir = file_entry.r.is_dir();
                    html! { <div class="p-8">
                        // Navigation bar
                        { self::navbar::render(&ctx) }

                        // File contents
                        {
                            if is_dir {
                                match *ctx.view_mode {
                                    ViewMode::Grid => html! { <self::grid::FileList
                                        directory={ file_entry }
                                    /> },
                                    ViewMode::List => html! { <self::list::FileList
                                        { current }
                                        directory={ file_entry }
                                    /> },
                                }
                            } else {
                                html! { <self::preview::Preview
                                    { file_entry }
                                /> }
                            }
                        }
                    </div> }
                };
                file_entry.try_fetch_and_render(key, fetch, render)
            }}</div>
        </div>
    }
}

#[function_component(DirectoryPage)]
pub fn component(props: &Props) -> Html {
    // states
    let file_entry = use_state_eq(Default::default);
    let view_mode = use_state_eq(Default::default);

    // context
    let ctx = Context {
        props,
        current_timestamp: Utc::now(),
        file_entry,
        view_mode,
    };

    html! {
        <>
            // Sidebar contents (left)
            { self::sidebar::render(&ctx) }

            // Directory lookup
            { draw_file_entry_lookup(ctx) }
        </>
    }
}
