//! The trust root is the reviewed manifest compiled into the executable. Neither
//! a URL, a cached ZIP, a sentinel nor runtime metadata can supply expected hashes.
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) const MANIFEST: &str = include_str!("../toolchain-manifest.json");
pub(crate) const PART_IDS: &[&str] = &["kidbright-frameworks", "kidbright-tools"];

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    schema_version: u32,
    assets: Vec<Asset>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Asset {
    pub id: String,
    asset_identity: String,
    platform: String,
    version: String,
    pub download_url: String,
    trust: String,
    expected_sha256: Option<String>,
    provenance: Option<String>,
    checked_date: String,
    reason: String,
}

pub(crate) fn host_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

impl Manifest {
    pub(crate) fn parse(json: &str) -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|_| "Toolchain integrity: bundled manifest is invalid.".to_string())?;
        if manifest.schema_version != 1 {
            return Err("Toolchain integrity: unsupported manifest schema.".into());
        }
        Ok(manifest)
    }

    pub(crate) fn plan(
        &self,
        platform: &str,
        version: &str,
        custom_url: Option<&str>,
    ) -> Result<Vec<Asset>, String> {
        if custom_url.is_some_and(|url| !url.trim().is_empty()) {
            return Err("Toolchain integrity: custom URL installation is disabled because it has no trusted manifest binding. The existing toolchain was retained.".into());
        }
        PART_IDS.iter().map(|id| {
            let entries: Vec<_> = self.assets.iter().filter(|asset| asset.id == *id).collect();
            if entries.is_empty() {
                return Err(format!("Toolchain integrity: missing manifest entry for {id}. The existing toolchain was retained."));
            }
            let matching: Vec<_> = entries.iter().filter(|asset| asset.platform == platform && asset.version == version).collect();
            if matching.len() != 1 {
                return Err(format!("Toolchain integrity: platform/version mismatch or ambiguous manifest for {id}. No trusted asset for this platform and version; the existing toolchain was retained."));
            }
            let asset = (*matching[0]).clone();
            if asset.trust != "verified" {
                return Err(format!("Toolchain integrity: {id} is unverified. Publisher and version/platform evidence is unavailable; installation is disabled. The existing toolchain was retained."));
            }
            if asset.asset_identity.is_empty() || asset.checked_date.is_empty()
                || asset.provenance.as_ref().is_none_or(|source| source.trim().is_empty())
                || !asset.expected_sha256.as_ref().is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()))
            {
                return Err(format!("Toolchain integrity: incomplete trusted manifest for {id}. Installation is disabled."));
            }
            // Keep the rejection rationale in the schema for unverified records.
            let _ = &asset.reason;
            Ok(asset)
        }).collect()
    }
}

/// Owns an anonymous temporary snapshot. Its path and handle never escape this
/// type after acquisition; hashing, extraction and activation checks all read
/// this same handle, never reopen a caller-controlled pathname.
pub(crate) struct VerifiedArchive {
    file: File,
    asset: Asset,
}

impl VerifiedArchive {
    #[cfg(test)]
    pub(crate) fn corrupt_snapshot_for_test(&mut self) {
        use std::io::Write;
        self.file.seek(SeekFrom::Start(0)).unwrap();
        self.file.write_all(b"modified snapshot").unwrap();
    }
    pub(crate) fn acquire(
        asset: Asset,
        cancel: &AtomicBool,
        acquire: impl FnOnce(&mut File) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut file = tempfile::tempfile().map_err(|e| {
            format!(
                "Toolchain integrity: cannot create private archive snapshot ({:?}).",
                e.kind()
            )
        })?;
        acquire(&mut file)?;
        let mut archive = Self { file, asset };
        archive.verify(cancel)?;
        Ok(archive)
    }

    pub(crate) fn verify(&mut self, cancel: &AtomicBool) -> Result<(), String> {
        let result = (|| -> std::io::Result<String> {
            self.file.seek(SeekFrom::Start(0))?;
            let mut digest = Sha256::new();
            let mut buffer = [0u8; 128 * 1024];
            loop {
                if cancel.load(Ordering::SeqCst) {
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                let count = self.file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                digest.update(&buffer[..count]);
            }
            self.file.seek(SeekFrom::Start(0))?;
            Ok(format!("{:x}", digest.finalize()))
        })();
        let actual = result.map_err(|e| format!("Toolchain integrity: cannot read archive or verification was cancelled ({:?}). The existing toolchain was retained.", e.kind()))?;
        if !self
            .asset
            .expected_sha256
            .as_ref()
            .is_some_and(|expected| expected.eq_ignore_ascii_case(&actual))
        {
            return Err(format!("Toolchain integrity: SHA-256 mismatch for {}. Installation stopped; the existing toolchain was retained.", self.asset.id));
        }
        Ok(())
    }

    pub(crate) fn extract(
        &mut self,
        destination: &Path,
        cancel: &AtomicBool,
        progress: impl FnMut(usize, usize, &str),
    ) -> Result<(), String> {
        self.verify(cancel)?;
        crate::archive::extract_zip_reader(&mut self.file, destination, cancel, progress).map_err(
            |error| {
                let cause = if error.contains("Unsafe")
                    || error.contains("outside")
                    || error.contains("symlink")
                {
                    "unsafe archive path"
                } else if error.contains("link/special") {
                    "archive link or special file"
                } else if error.contains("limit") {
                    "archive safety limit exceeded"
                } else if error.contains("Cancelled") {
                    "cancelled"
                } else {
                    "invalid ZIP or extraction I/O failure"
                };
                format!(
                    "Toolchain extraction stopped: {cause}. The existing toolchain was retained."
                )
            },
        )
    }
}

pub(crate) fn legacy_setup_policy() -> Result<String, String> {
    Err("Toolchain integrity: automatic Git/Python ESP-IDF setup is disabled because the checkout, tool archives and Python dependencies have no trusted manifest binding. The existing toolchain was retained. Configure an existing SDK or wait for verified assets.".into())
}
