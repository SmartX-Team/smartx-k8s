use std::{borrow::Cow, path::PathBuf};

use actix_web::{HttpRequest, HttpResponse, Responder, http::Method, web};
use openark_vine_browser_api::file_type::FileType;
use serde::Deserialize;
use tokio::fs;
use tokio_util::io::ReaderStream;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Query {
    #[serde(default)]
    download: bool,
}

async fn get(
    data_dir: web::Data<PathBuf>,
    path: Cow<'_, str>,
    query: web::Query<Query>,
) -> HttpResponse {
    // Parse the query
    let Query { download } = query.into_inner();

    // Parse the path
    let path = path.trim_end_matches('/');
    let path = format!(".{path}");

    // Get file
    // FIXME: Do not follow symlinks, it will break the system security!
    // FIXME: Use `nix` and `O_NOFOLLOW` not to follow the symlinks
    let path: PathBuf = path.into();
    let file = match fs::File::open(&path).await {
        Ok(file) => file,
        Err(error) => {
            #[cfg(feature = "tracing")]
            ::tracing::warn!("Failed to get file: {error}");
            let _ = error;
            return HttpResponse::NotFound().finish();
        }
    };

    // Get metadata
    let metadata = match file.metadata().await {
        Ok(metadata) => metadata,
        Err(error) => {
            #[cfg(feature = "tracing")]
            ::tracing::debug!("Failed to get metadata: {error}");
            let _ = error;
            return HttpResponse::NotFound().finish();
        }
    };

    // Parse the extension
    let content_type = if download {
        "application/octet-stream"
    } else {
        path.extension()
            .and_then(|s| s.to_str())
            .and_then(FileType::from_known_extensions)
            .map(|ty| ty.mime_type())
            .unwrap_or("text/plain")
    };

    // Send the content
    let stream = ReaderStream::new(file);
    HttpResponse::Ok()
        .content_type(content_type)
        .no_chunking(metadata.len())
        .streaming(stream)
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub async fn handle(
    req: HttpRequest,
    base_url: web::Data<String>,
    data_dir: web::Data<PathBuf>,
    query: web::Query<Query>,
) -> impl Responder {
    let path = match super::parse_path(&req, base_url, "/data") {
        Some(path) => path,
        None => return HttpResponse::NotFound().finish(),
    };
    match *req.method() {
        Method::GET => get(data_dir, path, query).await,
        _ => HttpResponse::NotFound().finish(),
    }
}
