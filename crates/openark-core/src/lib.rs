#[cfg(feature = "opentelemetry")]
extern crate openark_core_opentelemetry as opentelemetry;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "operator")]
pub mod operator;
#[cfg(feature = "tls")]
mod tls;

/// Initialize the global defaults.
///
pub fn init_once() {
    #[cfg(feature = "tls")]
    crate::tls::init_once();
    #[cfg(feature = "opentelemetry")]
    crate::opentelemetry::init_once();
}
