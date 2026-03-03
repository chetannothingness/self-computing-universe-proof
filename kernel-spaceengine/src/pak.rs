use kernel_types::{Hash32, hash};
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

/// Builds .pak files (real zip archives) for SpaceEngine addon distribution.
/// SpaceEngine natively reads .pak as virtual filesystem overlays in addons/.
pub struct PakBuilder;

impl PakBuilder {
    /// Package files into a .pak (zip archive).
    /// Files are sorted by name (BTreeMap guarantees this) for deterministic output.
    /// All files stored with Deflate compression, fixed timestamp for determinism.
    /// Returns (pak_bytes, pak_hash).
    pub fn build(files: &BTreeMap<String, Vec<u8>>) -> (Vec<u8>, Hash32) {
        let buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);

        // Fixed options for determinism: no timestamps, Deflate level 6.
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(
                zip::DateTime::from_date_and_time(2025, 1, 1, 0, 0, 0)
                    .unwrap_or_default()
            );

        // BTreeMap iteration is sorted — deterministic ordering.
        for (name, data) in files {
            zip.start_file(name, options).expect("zip: start_file must succeed");
            zip.write_all(data).expect("zip: write_all must succeed");
        }

        let result = zip.finish().expect("zip: finish must succeed");
        let pak_bytes = result.into_inner();
        let pak_hash = hash::H(&pak_bytes);
        (pak_bytes, pak_hash)
    }

    /// Extract files from a .pak (zip) archive.
    pub fn extract(pak_bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
        let reader = Cursor::new(pak_bytes);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| format!("Invalid zip/pak: {}", e))?;

        let mut files = BTreeMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("zip entry {}: {}", i, e))?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| format!("zip read {}: {}", name, e))?;
            files.insert(name, data);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pak_roundtrip() {
        let mut files = BTreeMap::new();
        files.insert("test/a.txt".into(), b"hello".to_vec());
        files.insert("test/b.sc".into(), b"Star \"Test\" {}".to_vec());

        let (pak_bytes, pak_hash) = PakBuilder::build(&files);
        assert!(!pak_bytes.is_empty());
        assert_ne!(pak_hash, [0u8; 32]);

        let extracted = PakBuilder::extract(&pak_bytes).unwrap();
        assert_eq!(extracted, files);
    }

    #[test]
    fn pak_deterministic() {
        let mut files = BTreeMap::new();
        files.insert("a.txt".into(), b"data".to_vec());

        let (pak1, hash1) = PakBuilder::build(&files);
        let (pak2, hash2) = PakBuilder::build(&files);
        assert_eq!(pak1, pak2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn pak_empty() {
        let files = BTreeMap::new();
        let (pak, _) = PakBuilder::build(&files);
        let extracted = PakBuilder::extract(&pak).unwrap();
        assert!(extracted.is_empty());
    }

    #[test]
    fn pak_is_valid_zip() {
        let mut files = BTreeMap::new();
        files.insert("catalogs/stars.sc".into(), b"Star \"Test\" {}".to_vec());
        files.insert("scripts/test.se".into(), b"SaveVars\nRestoreVars\n".to_vec());

        let (pak_bytes, _) = PakBuilder::build(&files);

        // Verify it's a valid zip by checking PK magic bytes.
        assert_eq!(&pak_bytes[0..2], b"PK", "Must start with PK zip magic");
    }
}
