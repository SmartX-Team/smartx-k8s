pub fn init_once() {
    #[cfg(feature = "rustls-tls")]
    ::rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
}
