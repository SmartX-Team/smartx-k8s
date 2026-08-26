use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::ErrorKind,
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::{FileTypeExt, MetadataExt},
    },
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail, ensure};

use crate::model::{
    BY_ID_PREFIX, DeviceInventory, NodeInventory, PcieLink, is_pci_bdf, pcie_hardware_class,
};

pub fn scan(host_dev_root: &Path, host_sys_root: &Path) -> Result<NodeInventory> {
    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_secs();
    let dev_root = fs::canonicalize(host_dev_root).with_context(|| {
        format!(
            "canonicalizing host device root {}",
            host_dev_root.display()
        )
    })?;
    let by_id = dev_root.join("disk/by-id");
    let mut grouped = BTreeMap::<String, Vec<String>>::new();

    for entry in fs::read_dir(&by_id)
        .with_context(|| format!("reading host by-id directory {}", by_id.display()))?
    {
        let entry = entry.with_context(|| format!("reading an entry in {}", by_id.display()))?;
        let alias = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow!("by-id entry name is not valid UTF-8"))?;
        if is_partition_alias(&alias) {
            continue;
        }

        let resolved = fs::canonicalize(entry.path())
            .with_context(|| format!("resolving by-id alias {alias:?}"))?;
        let kernel_name = resolved_kernel_name(&dev_root, &resolved).with_context(|| {
            format!(
                "by-id alias {alias:?} resolved outside the host device root: {}",
                resolved.display()
            )
        })?;
        let sys_block = host_sys_root.join("class/block").join(&kernel_name);
        fs::canonicalize(&sys_block)
            .with_context(|| format!("resolving sysfs block device {}", sys_block.display()))?;
        if has_partition_marker(&sys_block)? {
            continue;
        }
        let metadata = fs::metadata(&resolved)
            .with_context(|| format!("reading by-id target {}", resolved.display()))?;
        if !metadata.file_type().is_block_device() {
            bail!("by-id alias {alias:?} does not resolve to a block device");
        }

        grouped.entry(kernel_name).or_default().push(alias);
    }

    let mut devices = Vec::new();
    for (kernel_name, aliases) in grouped {
        let sys_block = host_sys_root.join("class/block").join(&kernel_name);
        let nsid = read_nsid(&sys_block, &kernel_name)?;
        let mut aliases: Vec<_> = aliases
            .into_iter()
            .filter(|alias| {
                alias_priority(alias, nsid.as_deref()).is_some()
                    && (!alias.starts_with("nvme-")
                        || alias.starts_with("nvme-eui.")
                        || alias.starts_with("nvme-uuid.")
                        || generic_nvme_alias_matches_namespace(alias, nsid.as_deref()))
            })
            .collect();
        aliases.sort_by(|left, right| {
            alias_priority(left, nsid.as_deref())
                .cmp(&alias_priority(right, nsid.as_deref()))
                .then_with(|| left.cmp(right))
        });
        aliases.dedup();
        let Some(selected) = aliases.first().cloned() else {
            continue;
        };

        let host_path = by_id.join(&selected);
        let snapshot = snapshot_device(
            &host_path,
            &dev_root,
            &by_id,
            &aliases,
            &sys_block,
            &kernel_name,
            nsid.as_deref(),
        )?;
        let aliases = aliases
            .into_iter()
            .map(|alias| format!("{BY_ID_PREFIX}{alias}"))
            .collect();
        devices.push(DeviceInventory {
            path: format!("{BY_ID_PREFIX}{selected}"),
            aliases,
            kernel_name: kernel_name.clone(),
            size_bytes: snapshot.size_bytes,
            read_only: snapshot.read_only,
            removable: snapshot.removable,
            rotational: snapshot.rotational,
            signature_free: snapshot.signature_free,
            pcie: snapshot.pcie,
        });
    }

    devices.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(NodeInventory {
        generated_at_unix,
        devices,
    })
}

fn resolved_kernel_name(dev_root: &Path, resolved: &Path) -> Option<String> {
    let relative = resolved.strip_prefix(dev_root).ok()?;
    let mut components = relative.components();
    let name = components.next()?.as_os_str().to_str()?;
    if name.is_empty() || components.next().is_some() {
        return None;
    }
    Some(name.to_owned())
}

fn is_partition_alias(alias: &str) -> bool {
    alias.rsplit_once("-part").is_some_and(|(_, number)| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn has_partition_marker(sys_block: &Path) -> Result<bool> {
    match fs::symlink_metadata(sys_block.join("partition")) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| {
            format!(
                "checking partition marker for block device {}",
                sys_block.display()
            )
        }),
    }
}

fn alias_priority(alias: &str, nsid: Option<&str>) -> Option<u8> {
    if alias.starts_with("wwn-") {
        Some(0)
    } else if alias.starts_with("nvme-eui.") {
        Some(1)
    } else if alias.starts_with("nvme-uuid.") {
        Some(2)
    } else if alias.starts_with("dm-uuid-mpath-") {
        Some(3)
    } else if alias.starts_with("scsi-") {
        Some(4)
    } else if alias.starts_with("ata-") {
        Some(5)
    } else if alias.starts_with("nvme-")
        && nsid.is_some_and(|nsid| alias.ends_with(&format!("_{nsid}")))
    {
        Some(6)
    } else {
        None
    }
}

fn generic_nvme_alias_matches_namespace(alias: &str, nsid: Option<&str>) -> bool {
    let Some(nsid) = nsid else {
        return false;
    };
    let Some(base) = alias.strip_suffix(&format!("_{nsid}")) else {
        return false;
    };
    base.strip_prefix("nvme-")
        .and_then(|identity| identity.split_once('_'))
        .is_some_and(|(model, serial)| !model.is_empty() && !serial.is_empty())
}

struct DeviceSnapshot {
    size_bytes: u64,
    read_only: bool,
    removable: bool,
    rotational: bool,
    signature_free: bool,
    pcie: Option<PcieLink>,
}

fn snapshot_device(
    alias: &Path,
    dev_root: &Path,
    by_id: &Path,
    published_aliases: &[String],
    sys_block: &Path,
    expected_kernel: &str,
    expected_nsid: Option<&str>,
) -> Result<DeviceSnapshot> {
    let (before_kernel, before_rdev) = alias_identity(alias, dev_root)?;
    ensure_device_identity(expected_kernel, before_rdev, &before_kernel, before_rdev)?;
    let device = File::open(alias)
        .with_context(|| format!("opening {} for signature probing", alias.display()))?;
    let opened_rdev = device
        .metadata()
        .with_context(|| format!("reading opened device metadata for {}", alias.display()))?
        .rdev();
    ensure_device_identity(expected_kernel, before_rdev, &before_kernel, opened_rdev)?;
    ensure_sysfs_identity(sys_block, opened_rdev)?;
    let current_nsid = read_nsid(sys_block, expected_kernel)?;
    ensure!(
        current_nsid.as_deref() == expected_nsid,
        "NVMe namespace ID changed during device snapshot"
    );
    let size_bytes = read_number::<u64>(&sys_block.join("size"))?
        .checked_mul(512)
        .context("block device size overflows u64 bytes")?;
    let read_only = read_number::<u8>(&sys_block.join("ro"))? != 0;
    let removable = read_number::<u8>(&sys_block.join("removable"))? != 0;
    let rotational = read_number::<u8>(&sys_block.join("queue/rotational"))? != 0;
    let pcie = if expected_kernel.starts_with("nvme") {
        pcie_link(sys_block)?
    } else {
        None
    };
    let probe = BlkidProbe::new(device.as_raw_fd())
        .with_context(|| format!("creating signature probe for {}", alias.display()))?;
    let signature_free = probe
        .signature_free()
        .with_context(|| format!("probing signatures on {}", alias.display()))?;
    let final_nsid = read_nsid(sys_block, expected_kernel)?;
    ensure!(
        final_nsid.as_deref() == expected_nsid,
        "NVMe namespace ID changed before snapshot completion"
    );
    ensure_sysfs_identity(sys_block, opened_rdev)?;
    let (after_kernel, after_rdev) = alias_identity(alias, dev_root)?;
    ensure_device_identity(expected_kernel, opened_rdev, &after_kernel, after_rdev)?;
    for alias_name in published_aliases {
        let alias_path = by_id.join(alias_name);
        let (alias_kernel, alias_rdev) = alias_identity(&alias_path, dev_root)
            .with_context(|| format!("validating published by-id alias {alias_name:?}"))?;
        ensure_device_identity(expected_kernel, opened_rdev, &alias_kernel, alias_rdev)
            .with_context(|| format!("published by-id alias {alias_name:?} changed identity"))?;
    }
    Ok(DeviceSnapshot {
        size_bytes,
        read_only,
        removable,
        rotational,
        signature_free,
        pcie,
    })
}

fn read_nsid(sys_block: &Path, kernel_name: &str) -> Result<Option<String>> {
    if !kernel_name.starts_with("nvme") {
        return Ok(None);
    }
    let nsid = fs::read_to_string(sys_block.join("nsid"))
        .with_context(|| format!("reading NVMe namespace ID for {kernel_name:?}"))?;
    let nsid = nsid
        .trim()
        .parse::<u32>()
        .with_context(|| format!("parsing NVMe namespace ID for {kernel_name:?}"))?;
    ensure!(
        nsid > 0,
        "NVMe namespace {kernel_name:?} has namespace ID zero"
    );
    Ok(Some(nsid.to_string()))
}

fn ensure_sysfs_identity(sys_block: &Path, expected_rdev: u64) -> Result<()> {
    let raw = fs::read_to_string(sys_block.join("dev"))
        .with_context(|| format!("reading device number for {}", sys_block.display()))?;
    let (major, minor) = raw
        .trim()
        .split_once(':')
        .context("sysfs device number has no major:minor separator")?;
    let major = major.parse::<u64>().context("parsing sysfs major number")?;
    let minor = minor.parse::<u64>().context("parsing sysfs minor number")?;
    let (expected_major, expected_minor) = linux_device_numbers(expected_rdev);
    ensure!(
        (major, minor) == (expected_major, expected_minor),
        "sysfs block identity changed during device snapshot"
    );
    Ok(())
}

fn linux_device_numbers(device: u64) -> (u64, u64) {
    let major = ((device >> 8) & 0xfff) | ((device >> 32) & 0xfffff000);
    let minor = (device & 0xff) | ((device >> 12) & 0xffffff00);
    (major, minor)
}

fn alias_identity(alias: &Path, dev_root: &Path) -> Result<(String, u64)> {
    let resolved = fs::canonicalize(alias)
        .with_context(|| format!("resolving selected by-id alias {}", alias.display()))?;
    let kernel_name = resolved_kernel_name(dev_root, &resolved)
        .context("selected by-id alias resolved outside the host device root")?;
    let metadata = fs::metadata(&resolved)
        .with_context(|| format!("reading selected device metadata {}", resolved.display()))?;
    ensure!(
        metadata.file_type().is_block_device(),
        "selected by-id alias does not resolve to a block device"
    );
    Ok((kernel_name, metadata.rdev()))
}

fn ensure_device_identity(
    expected_kernel: &str,
    expected_rdev: u64,
    actual_kernel: &str,
    actual_rdev: u64,
) -> Result<()> {
    ensure!(
        expected_kernel == actual_kernel && expected_rdev == actual_rdev,
        "selected by-id alias changed device identity during signature probing"
    );
    Ok(())
}

enum BlkidProbeOpaque {}
type BlkidProbePtr = *mut BlkidProbeOpaque;

#[link(name = "blkid")]
unsafe extern "C" {
    fn blkid_new_probe() -> BlkidProbePtr;
    fn blkid_probe_set_device(probe: BlkidProbePtr, fd: RawFd, offset: i64, size: i64) -> i32;
    fn blkid_do_safeprobe(probe: BlkidProbePtr) -> i32;
    fn blkid_free_probe(probe: BlkidProbePtr);
}

struct BlkidProbe(BlkidProbePtr);

impl BlkidProbe {
    fn new(fd: RawFd) -> Result<Self> {
        // SAFETY: libblkid returns either a new owned probe or null.
        let probe = unsafe { blkid_new_probe() };
        ensure!(!probe.is_null(), "libblkid could not allocate a probe");
        let probe = Self(probe);
        // SAFETY: probe is non-null and fd remains open for the probe lifetime.
        let result = unsafe { blkid_probe_set_device(probe.0, fd, 0, 0) };
        ensure!(
            result == 0,
            "libblkid could not bind the block device descriptor"
        );
        Ok(probe)
    }

    fn signature_free(&self) -> Result<bool> {
        // SAFETY: self owns a live non-null libblkid probe.
        let result = unsafe { blkid_do_safeprobe(self.0) };
        signature_free_from_probe_result(result)
    }
}

impl Drop for BlkidProbe {
    fn drop(&mut self) {
        // SAFETY: this probe was allocated by blkid_new_probe and is owned here.
        unsafe {
            blkid_free_probe(self.0);
        }
    }
}

fn signature_free_from_probe_result(result: i32) -> Result<bool> {
    match result {
        1 => Ok(true),
        0 | -2 => Ok(false),
        -1 => Err(std::io::Error::last_os_error()).context("libblkid probe failed"),
        code => bail!("libblkid returned unexpected probe result {code}"),
    }
}

fn pcie_link(sys_block: &Path) -> Result<Option<PcieLink>> {
    let device = fs::canonicalize(sys_block.join("device"))
        .with_context(|| format!("resolving device ancestry for {}", sys_block.display()))?;
    let Some(pci) = device.ancestors().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_pci_bdf)
    }) else {
        return Ok(None);
    };
    let bdf = pci
        .file_name()
        .and_then(|name| name.to_str())
        .context("PCI device path has no UTF-8 BDF")?
        .to_owned();
    let speed_gts: f64 = read_number(&pci.join("current_link_speed"))?;
    let width: u32 = read_number(&pci.join("current_link_width"))?;
    ensure!(
        speed_gts.is_finite() && speed_gts > 0.0 && width > 0,
        "PCIe link attributes are not positive"
    );
    Ok(Some(build_pcie_link(&bdf, speed_gts, width)?))
}

fn read_number<T>(path: &Path) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value = content
        .split_whitespace()
        .next()
        .with_context(|| format!("{} has no value", path.display()))?;
    value
        .parse()
        .map_err(|error| anyhow!("parsing {}: {error}", path.display()))
}

fn build_pcie_link(bdf: &str, speed_gts: f64, width: u32) -> Result<PcieLink> {
    let aggregate_gts = speed_gts * f64::from(width);
    Ok(PcieLink {
        bdf: bdf.to_owned(),
        speed_gts,
        width,
        aggregate_gts,
        hardware_class: pcie_hardware_class(speed_gts, width)
            .context("PCIe link speed or width is invalid")?,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, OpenOptions},
        os::fd::AsRawFd,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        BlkidProbe, alias_priority, build_pcie_link, ensure_device_identity,
        generic_nvme_alias_matches_namespace, has_partition_marker, is_partition_alias,
        signature_free_from_probe_result,
    };

    #[test]
    fn stable_aliases_have_deterministic_priority() {
        let mut aliases = ["ata-z", "scsi-z", "wwn-z", "nvme-eui.z"];
        aliases.sort_by(|left, right| {
            alias_priority(left, Some("1"))
                .cmp(&alias_priority(right, Some("1")))
                .then_with(|| left.cmp(right))
        });
        assert_eq!(aliases, ["wwn-z", "nvme-eui.z", "scsi-z", "ata-z"]);
    }

    #[test]
    fn generic_nvme_alias_must_end_in_nsid() {
        assert_eq!(alias_priority("nvme-model_serial_1", Some("1")), Some(6));
        assert_eq!(alias_priority("nvme-model_serial_11", Some("1")), None);
        assert_eq!(alias_priority("nvme-model_serial_1", None), None);
        assert!(generic_nvme_alias_matches_namespace(
            "nvme-model_serial_1",
            Some("1"),
        ));
        assert!(generic_nvme_alias_matches_namespace(
            "nvme-model_serial_2",
            Some("2"),
        ));
        assert!(!generic_nvme_alias_matches_namespace(
            "nvme-model_serial",
            Some("1")
        ));
    }

    #[test]
    fn equivalent_pcie_links_share_a_class() {
        assert_eq!(
            build_pcie_link("0000:01:00.0", 8.0, 4)
                .unwrap()
                .hardware_class,
            build_pcie_link("0000:02:00.0", 16.0, 2)
                .unwrap()
                .hardware_class
        );
    }

    #[test]
    fn fractional_pcie_product_is_preserved() {
        let link = build_pcie_link("0000:01:00.0", 2.5, 1).unwrap();
        assert_eq!(link.aggregate_gts, 2.5);
        assert_eq!(link.hardware_class, "nvme-pcie-2p5gt");
    }

    #[test]
    fn partition_aliases_are_rejected() {
        assert!(is_partition_alias("wwn-disk-part1"));
        assert!(is_partition_alias("nvme-model_1-part12"));
        assert!(!is_partition_alias("ata-partitioned-disk"));

        let root = std::env::temp_dir().join(format!(
            "openark-rook-ceph-partition-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        assert!(!has_partition_marker(&root).unwrap());
        fs::write(root.join("partition"), "1\n").unwrap();
        assert!(has_partition_marker(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn signature_probe_fails_closed() {
        assert!(signature_free_from_probe_result(1).unwrap());
        assert!(!signature_free_from_probe_result(0).unwrap());
        assert!(!signature_free_from_probe_result(-2).unwrap());
        assert!(signature_free_from_probe_result(-1).is_err());
        assert!(signature_free_from_probe_result(2).is_err());
    }

    #[test]
    fn libblkid_reports_empty_file_as_no_signature() {
        let path =
            std::env::temp_dir().join(format!("openark-rook-ceph-blkid-{}", std::process::id()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        file.set_len(4096).unwrap();
        let probe = BlkidProbe::new(file.as_raw_fd()).unwrap();
        assert!(probe.signature_free().unwrap());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn retargeted_device_identity_is_rejected() {
        ensure_device_identity("nvme0n1", 259, "nvme0n1", 259).unwrap();
        assert!(ensure_device_identity("nvme0n1", 259, "nvme1n1", 260).is_err());
        assert!(ensure_device_identity("nvme0n1", 259, "nvme0n1", 260).is_err());
    }
}
