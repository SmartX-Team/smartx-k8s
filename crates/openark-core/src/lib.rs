#[cfg(feature = "opentelemetry")]
extern crate openark_core_opentelemetry as opentelemetry;

#[cfg(feature = "operator")]
pub mod operator;
#[cfg(all(feature = "std", any(feature = "openssl-tls", feature = "rustls-tls")))]
mod tls;

/// Initialize the global defaults.
///
pub fn init_once() {
    #[cfg(any(feature = "openssl-tls", feature = "rustls-tls"))]
    crate::tls::init_once();
    #[cfg(feature = "opentelemetry")]
    crate::opentelemetry::init_once();
}
