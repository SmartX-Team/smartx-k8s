#[cfg(feature = "opentelemetry")]
extern crate openark_core_opentelemetry as opentelemetry;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "operator")]
pub mod operator;
#[cfg(feature = "tls")]
mod tls;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(::clap::Parser))]
#[cfg_attr(feature = "clap", group(skip))]
pub struct OpenArkArgs {
    #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
    #[cfg_attr(feature = "clap", arg(
        long,
        env = "RUST_LOG",
        default_value_t = OpenArkArgs::default_log_level(),
    ))]
    pub log_level: ::tracing::Level,

    #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
    #[cfg_attr(feature = "clap", arg(long, env = "OPENTELEMETRY_EXPORT"))]
    pub opentelemetry_export: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for OpenArkArgs {
    fn default() -> Self {
        Self {
            #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
            log_level: Self::default_log_level(),
            #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
            opentelemetry_export: false,
        }
    }
}

#[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
impl OpenArkArgs {
    #[inline]
    const fn default_log_level() -> ::tracing::Level {
        ::tracing::Level::INFO
    }
}

/// Initialize the global defaults.
///
#[inline]
pub fn init_once() {
    let args = Default::default();
    init_once_with(args)
}

/// Initialize the global defaults with given [OpenArkArgs].
///
pub fn init_once_with(args: OpenArkArgs) {
    let OpenArkArgs {
        #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
            log_level: level,
        #[cfg(all(feature = "opentelemetry", not(target_arch = "wasm32")))]
            opentelemetry_export: export,
    } = args;

    #[cfg(feature = "tls")]
    crate::tls::init_once();

    #[cfg(feature = "opentelemetry")]
    {
        #[cfg(target_arch = "wasm32")]
        crate::opentelemetry::init_once();

        #[cfg(not(target_arch = "wasm32"))]
        crate::opentelemetry::init_once_with(level, export);
    }
}
