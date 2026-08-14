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
