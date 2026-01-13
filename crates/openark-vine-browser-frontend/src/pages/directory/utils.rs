use chrono::{DateTime, Utc};
use openark_vine_browser_api::{
    file::{FileRef, FileTimestamp},
    user::UserRef,
};
use web_sys::DataTransfer;
use yew::{MouseEvent, html::IntoEventCallback};
use yew_router::prelude::Navigator;

use crate::router::Route;

pub(super) fn format_date(current: DateTime<Utc>, timestamp: Option<&FileTimestamp>) -> String {
    let timestamp = match timestamp {
        Some(value) => value.timestamp,
        None => return "-".into(),
    };

    "-".into()
}

pub(super) fn format_initial(user: &UserRef) -> String {
    user.metadata.initial.clone().unwrap_or_else(|| {
        let name = user.name.as_str();
        name[..name.len().min(2)].into()
    })
}

pub(super) fn format_size(is_dir: bool, size: Option<u64>) -> String {
    let size = match size {
        Some(value) => value,
        None => return "-".into(),
    };

    let mut base = size.highest_one().unwrap_or(0) / 10;
    let unit = match base {
        0 => "",
        1 => "Ki",
        2 => "Mi",
        3 => "Gi",
        4 => "Ti",
        5 => "Pi",
        6 => "Ei",
        7 => "Zi",
        8 => "Yi",
        9 => "Ri",
        10.. => {
            base = 10;
            "Qi"
        }
    };
    let value = (size as f64) / ((1u64 << (base * 10)) as f64);
    let suffix = if is_dir { " files" } else { "B" };
    format!(
        "{value} {unit}{suffix}",
        value = format!("{value:.02}")
            .trim_end_matches('0')
            .trim_end_matches('.'),
    )
}

/// Resolve to the `file`.
///
pub(super) fn push_entry(
    nav: Option<Navigator>,
    file: &FileRef,
) -> impl IntoEventCallback<MouseEvent> {
    let path = file.path.trim_matches('/').to_string();
    move |_| {
        if let Some(nav) = nav.clone() {
            nav.push(&Route::FileEntry { path: path.clone() })
        }
    }
}

/// Upload a file into the `dst` directory.
///
pub(super) fn upload(src: DataTransfer, dst: FileRef) {
    // TODO: To be implemented!
    tracing::info!("src: {src:#?}, dst: {}", &dst.path)
}
