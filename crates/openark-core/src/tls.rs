pub fn init_once() {
    let provider = {
        #[cfg(feature = "tls-aws-lc-rs")]
        {
            ::rustls::crypto::aws_lc_rs::default_provider()
        }

        #[cfg(feature = "tls-ring")]
        {
            ::rustls::crypto::ring::default_provider()
        }
    };

    provider
        .install_default()
        .expect("Failed to install rustls crypto provider");
}
