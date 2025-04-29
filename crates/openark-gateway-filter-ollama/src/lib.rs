use std::{borrow::Cow, collections::BTreeMap, rc::Rc};

use proxy_wasm::{
    main, set_log_level, set_root_context,
    traits::{Context, HttpContext, RootContext},
    types::{Action, ContextType, LogLevel},
};
use serde::Deserialize;
use tracing::info;

#[derive(Clone, Default, Deserialize)]
struct Config {
    #[serde(default)]
    models: BTreeMap<String, String>,

    #[serde(default)]
    passthrough: bool,
}

#[derive(Deserialize)]
struct RequestBody {
    model: String,
}

struct Body {
    config: Rc<Config>,
    path: Option<String>,
}

impl Context for Body {}

impl HttpContext for Body {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        if self.config.passthrough {
            self.path = self.get_http_request_header(":path");
        }
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        match self.get_http_request_body(0, body_size) {
            Some(bytes) => match ::serde_json::from_slice(&bytes) {
                Ok(RequestBody { model }) => {
                    match self.config.models.get(&model) {
                        Some(location) => {
                            info!("Accepted model name: {model}");
                            self.send_http_response_redirect(location.as_str())
                        }
                        None => {
                            if self.config.passthrough {
                                if let Some(mut path) = self.path.take() {
                                    info!("Accepted model name (PT): {model}");
                                    let location = {
                                        path.push('/');
                                        path.push_str(&model);
                                        path
                                    };
                                    return self.send_http_response_redirect(location.as_str());
                                }
                            }

                            info!("Rejected model name: {model}");
                            let status_code = 404; // Not Found
                            let body = Cow::Owned(format!("model '{model}' not found"));
                            self.send_http_response_error(status_code, body)
                        }
                    }
                }
                Err(_) => {
                    let status_code = 403; // Forbidden
                    let body = Cow::Borrowed("invalid body");
                    self.send_http_response_error(status_code, body)
                }
            },
            None => Action::Continue,
        }
    }
}

impl Body {
    fn send_http_response_redirect(&self, location: &str) -> Action {
        let status_code = 308; // Permanent Redirect
        let headers = vec![("location", location)];
        let body = None;
        self.send_http_response(status_code, headers, body);
        Action::Pause
    }

    fn send_http_response_error<'a>(&self, status_code: u32, body: Cow<'a, str>) -> Action {
        let headers = vec![("content-type", "application/json; charset=utf-8")];
        let body = Some(format!(r#"{{"error": {:?}}}"#, body.as_ref()).into_bytes());
        self.send_http_response(status_code, headers, body.as_deref());
        Action::Pause
    }
}

#[derive(Default)]
struct BodyRoot {
    config: Rc<Config>,
}

impl Context for BodyRoot {}

impl RootContext for BodyRoot {
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        // Load config
        self.config = Rc::new(
            self.get_plugin_configuration()
                .and_then(|data| ::serde_json::from_slice(&data).ok())
                .unwrap_or_default(),
        );
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(Body {
            config: self.config.clone(),
            path: None,
        }))
    }
}

main! {{
    set_log_level(LogLevel::Info);
    set_root_context(|_| Box::new(BodyRoot::default()));
}}
