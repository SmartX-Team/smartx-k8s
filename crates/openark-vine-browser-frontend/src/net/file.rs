use openark_vine_browser_api::client::ClientExt;
use url::Url;

use crate::net::Client;

/// Returns a file content [`Url`].
///
#[inline]
pub fn get_file_content_url(path: &str) -> Result<Url, ::url::ParseError> {
    let client = Client::new();
    client.get_file_content_url(path)
}
