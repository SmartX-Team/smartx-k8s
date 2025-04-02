use thiserror::Error;

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "datafusion")]
    #[error("{0}")]
    DataFusion(::datafusion::error::DataFusionError),
    #[error("File not found")]
    NotFound,
    #[cfg(feature = "std")]
    #[error("{0}")]
    IO(::std::io::Error),
}

#[cfg(feature = "datafusion")]
impl From<::datafusion::error::DataFusionError> for Error {
    #[inline]
    fn from(error: ::datafusion::error::DataFusionError) -> Self {
        Self::DataFusion(error)
    }
}

#[cfg(feature = "std")]
impl From<::std::io::Error> for Error {
    #[inline]
    fn from(error: ::std::io::Error) -> Self {
        Self::IO(error)
    }
}
