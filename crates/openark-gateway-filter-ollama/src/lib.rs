use proxy_wasm::{
    main, set_root_context,
    traits::{Context, HttpContext, RootContext},
    types::Action,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Model {
    model: String,
}

struct ModelExtractor;

impl Context for ModelExtractor {}

impl HttpContext for ModelExtractor {
    fn on_http_request_body(&mut self, body_size: usize, end_of_stream: bool) -> Action {
        if end_of_stream {
            if let Some(bytes) = self.get_http_request_body(0, body_size) {
                if let Ok(body) = ::serde_json::from_slice::<Model>(&bytes) {
                    self.set_http_request_header("X-MODEL-NAME", Some(&body.model));
                }
            }
        }
        Action::Continue
    }
}

impl RootContext for ModelExtractor {}

main! {{
    set_root_context(|_| Box::new(ModelExtractor));
}}
