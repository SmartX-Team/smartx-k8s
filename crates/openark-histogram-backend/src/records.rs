use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct RecordArgs {
    /// A default prometheus record of service histogram
    #[arg(long, env = "DEFAULT_RECORD_SERVICE", value_name = "NAME")]
    pub(crate) default_record_service: String,

    #[arg(long, env = "OPENARK_LABEL_HISTOGRAM_CUSTOM_RECORD")]
    pub(crate) label_custom_record: String,
}
