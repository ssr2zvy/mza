use tar::Builder;
use std::fs::File;
use std::path::Path;
use xz2::write::XzEncoder;

pub fn package_binary(
    compiled_binary: &Path,
    archive_path: &Path,
    archive_root: &str,
    binary_name: &str,
) -> Result<(), String> {
    let archive_file = File::create(archive_path)
        .map_err(|err| format!("Failed to create {}: {err}", archive_path.display()))?;
    let mut archive = Builder::new(XzEncoder::new(archive_file, 6));

    archive
        .append_path_with_name(compiled_binary, Path::new(archive_root).join(binary_name))
        .map_err(|err| format!("Failed to add {} to archive: {err}", compiled_binary.display()))?;

    let encoder = archive
        .into_inner()
        .map_err(|err| format!("Failed to finish {}: {err}", archive_path.display()))?;
    encoder
        .finish()
        .map_err(|err| format!("Failed to finish {}: {err}", archive_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn package_binary_produces_a_readable_tar_xz_archive() {
        let dir = tempfile::tempdir().unwrap();
        let binary_path = dir.path().join("lexicon_cli");
        std::fs::write(&binary_path, b"fake binary contents").unwrap();
        let archive_path = dir.path().join("out.tar.xz");

        package_binary(&binary_path, &archive_path, "lexicon_cli-0.1.0", "lexicon_cli").unwrap();

        let file = File::open(&archive_path).unwrap();
        let decoder = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        let mut entries = archive.entries().unwrap();

        let mut entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.path().unwrap().into_owned(), Path::new("lexicon_cli-0.1.0/lexicon_cli"));
        let mut contents = String::new();
        entry.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "fake binary contents");

        assert!(entries.next().is_none());
    }
}

