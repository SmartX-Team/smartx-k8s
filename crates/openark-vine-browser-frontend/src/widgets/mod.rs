mod error;
mod warn;

pub use self::{
    error::{Error, NotFound},
    warn::{Empty, Warn},
};
