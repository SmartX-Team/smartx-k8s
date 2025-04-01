use thiserror::Error;

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl From<::kube::Error> for Error {
    #[inline]
    fn from(error: ::kube::Error) -> Self {
        Self(ErrorKind::Api(error))
    }
}

#[derive(Debug, Error)]
enum ErrorKind {
    #[error("Api Error: {0}")]
    Api(::kube::Error),
}
