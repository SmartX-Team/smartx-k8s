use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const BY_ID_PREFIX: &str = "/dev/disk/by-id/";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInventory {
    pub generated_at_unix: u64,
    pub devices: Vec<DeviceInventory>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInventory {
    pub path: String,
    pub aliases: Vec<String>,
    pub kernel_name: String,
    pub size_bytes: u64,
    pub read_only: bool,
    pub removable: bool,
    pub rotational: bool,
    pub signature_free: bool,
    pub pcie: Option<PcieLink>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PcieLink {
    pub bdf: String,
    pub speed_gts: f64,
    pub width: u32,
    pub aggregate_gts: f64,
    pub hardware_class: String,
}

pub fn inventory_name(pod_uid: &str) -> Result<String> {
    const PREFIX: &str = "rook-ceph-inventory-";

    if pod_uid.is_empty()
        || PREFIX.len() + pod_uid.len() > 253
        || !pod_uid
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !pod_uid
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !pod_uid
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        bail!("pod UID is not a valid Kubernetes name component: {pod_uid:?}");
    }

    Ok(format!("{PREFIX}{pod_uid}"))
}

pub fn is_by_id_path(path: &str) -> bool {
    path.strip_prefix(BY_ID_PREFIX)
        .is_some_and(|name| !name.is_empty() && name != "." && name != ".." && !name.contains('/'))
}

pub fn is_pci_bdf(value: &str) -> bool {
    let Some((domain, rest)) = value.split_once(':') else {
        return false;
    };
    let Some((bus, rest)) = rest.split_once(':') else {
        return false;
    };
    let Some((slot, function)) = rest.split_once('.') else {
        return false;
    };
    [(domain, 4), (bus, 2), (slot, 2), (function, 1)]
        .into_iter()
        .all(|(part, length)| {
            part.len() == length && part.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

pub fn pcie_hardware_class(speed_gts: f64, width: u32) -> Option<String> {
    (speed_gts.is_finite() && speed_gts > 0.0 && width > 0).then(|| {
        let aggregate = (speed_gts * f64::from(width)).to_string().replace('.', "p");
        format!("nvme-pcie-{aggregate}gt")
    })
}

#[cfg(test)]
mod tests {
    use super::{inventory_name, is_by_id_path, is_pci_bdf, pcie_hardware_class};

    #[test]
    fn inventory_name_rejects_invalid_uids() {
        assert!(inventory_name("").is_err());
        assert!(inventory_name("ABC").is_err());
        assert!(inventory_name("-abc").is_err());
        assert!(inventory_name("abc-").is_err());
        assert_eq!(
            inventory_name("123e4567-e89b-12d3-a456-426614174000").unwrap(),
            "rook-ceph-inventory-123e4567-e89b-12d3-a456-426614174000"
        );
    }

    #[test]
    fn by_id_path_requires_one_normal_basename() {
        assert!(is_by_id_path("/dev/disk/by-id/wwn-a"));
        assert!(!is_by_id_path("/dev/sda"));
        assert!(!is_by_id_path("/dev/disk/by-id/"));
        assert!(!is_by_id_path("/dev/disk/by-id/../sda"));
        assert!(!is_by_id_path("/dev/disk/by-id/.."));
    }

    #[test]
    fn pcie_identity_and_class_are_canonical() {
        assert!(is_pci_bdf("0000:01:00.0"));
        assert!(!is_pci_bdf("device"));
        assert_eq!(
            pcie_hardware_class(2.5, 4).as_deref(),
            Some("nvme-pcie-10gt")
        );
        assert!(pcie_hardware_class(f64::NAN, 4).is_none());
    }
}
