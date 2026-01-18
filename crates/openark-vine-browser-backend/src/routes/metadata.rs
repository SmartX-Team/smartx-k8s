use std::{borrow::Cow, os::unix::fs::MetadataExt, path::PathBuf};

use actix_web::{HttpRequest, HttpResponse, Responder, http::Method, web};
use chrono::DateTime;
use openark_vine_browser_api::file::{FileEntry, FileMetadata, FileRef, FileTimestamp};
use serde::Deserialize;
use tokio::fs;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Query {
    #[serde(default = "Query::max_limit")]
    limit: usize,

    #[serde(default)]
    offset: usize,
}

impl Query {
    #[inline]
    const fn max_limit() -> usize {
        usize::MAX
    }
}

async fn get(
    data_dir: web::Data<PathBuf>,
    path: Cow<'_, str>,
    query: web::Query<Query>,
) -> HttpResponse {
    fn parse_file_ref(path: &PathBuf, metadata: &::std::fs::Metadata) -> FileRef {
        // Get extension
        let is_dir = metadata.is_dir();

        // Get canonical path
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into();
        let mut path: String = path.to_string_lossy().trim_start_matches(".").into();
        if is_dir {
            path.push('/');
        }

        FileRef {
            name,
            path,
            metadata: FileMetadata {
                accessed: DateTime::from_timestamp(metadata.atime(), metadata.atime_nsec() as _)
                    .map(|timestamp| FileTimestamp {
                        by: None,
                        timestamp,
                    }),
                created: DateTime::from_timestamp(metadata.ctime(), metadata.ctime_nsec() as _)
                    .map(|timestamp| FileTimestamp {
                        by: None,
                        timestamp,
                    }),
                modified: DateTime::from_timestamp(metadata.mtime(), metadata.mtime_nsec() as _)
                    .map(|timestamp| FileTimestamp {
                        by: None,
                        timestamp,
                    }),
                owner: None,
                size: Some(metadata.size()),
            },
        }
    }

    // Parse the query
    let Query { limit, mut offset } = query.into_inner();

    // Constraint the query
    let mut limit = limit.min(Query::max_limit());

    // Parse the path
    let path = path.trim_end_matches('/');
    let path = format!(".{path}");

    // Get metadata
    // FIXME: Do not follow symlinks, it will break the system security!
    // FIXME: Use `nix` and `O_NOFOLLOW` not to follow the symlinks
    let path: PathBuf = path.into();
    let metadata = match fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) => {
            #[cfg(feature = "tracing")]
            ::tracing::debug!("Failed to get metadata: {error}");
            let _ = error;
            return HttpResponse::NotFound().finish();
        }
    };

    // Get children
    let files = if metadata.is_dir() {
        match fs::read_dir(&path).await {
            Ok(mut read_dir) => {
                let mut files = Vec::with_capacity(metadata.size() as _);
                while let Some(result) = read_dir.next_entry().await.transpose() {
                    if let Ok(entry) = result {
                        // Handle `offset`
                        // NOTE: `O(N)` algorithm
                        if offset > 0 {
                            offset -= 1;
                            continue;
                        }

                        // Parse the path
                        let path = entry.path();
                        if path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s.starts_with('.'))
                        {
                            // Hidden files
                            continue;
                        }

                        // Get metadata
                        let metadata = match entry.metadata().await {
                            Ok(metadata) => metadata,
                            Err(error) => {
                                #[cfg(feature = "tracing")]
                                ::tracing::debug!("Failed to get child metadata: {error}");
                                let _ = error;
                                continue;
                            }
                        };

                        // Handle `limit`
                        if limit > 0 {
                            limit -= 1
                        } else {
                            break;
                        }

                        files.push(parse_file_ref(&path, &metadata))
                    }
                }
                files
            }
            Err(_) => Vec::with_capacity(0),
        }
    } else {
        Vec::with_capacity(0)
    };

    HttpResponse::Ok().json(FileEntry {
        r: parse_file_ref(&path, &metadata),
        files,
    })
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub async fn handle(
    req: HttpRequest,
    base_url: web::Data<String>,
    data_dir: web::Data<PathBuf>,
    query: web::Query<Query>,
) -> impl Responder {
    let path = match super::parse_path(&req, base_url, "/metadata") {
        Some(path) => path,
        None => return HttpResponse::NotFound().finish(),
    };
    match *req.method() {
        Method::GET => get(data_dir, path, query).await,
        _ => HttpResponse::NotFound().finish(),
    }
}
