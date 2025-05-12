use strum::{Display, EnumString};

pub type Status = ::openark_core::operator::Status<Reason>;

#[derive(Copy, Clone, Debug, Display, Default, EnumString, PartialEq, Eq)]
pub(crate) enum Reason {
    Accepted,
    InvalidBackendRef,
    InvalidHistogram,
    InvalidMetricsClass,
    InvalidPool,
    InvalidTarget,
    #[default]
    Pending,
    ProvisioningError,
}

impl ::openark_core::operator::Reason for Reason {
    fn accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }
}
