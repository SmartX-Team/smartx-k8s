use strum::{Display, EnumString};

#[derive(Copy, Clone, Debug, Display, EnumString, PartialEq, Eq)]
pub(crate) enum Reason {
    SessionError,
    SessionUpdated,
}

impl ::openark_core::operator::Reason for Reason {
    fn accepted(&self) -> bool {
        matches!(self, Self::SessionUpdated)
    }
}
