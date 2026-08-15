use std::fmt;

/// Stable, machine-readable codes for every failure category this tool can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ParseIo,
    ParseInvalidToml,
    ParseInvalidArtifact,
    ParseInvalidTarget,
    ParseInvalidBundle,
    ArtifactMissingManifest,
    ArtifactMissingLabel,
    ArtifactUnsupportedTarget,
    ArtifactRustupFailed,
    ArtifactCargoInvocationFailed,
    ArtifactBuildFailed,
    ArtifactMissingPackageMetadata,
    ArtifactArchiveFailed,
    CargoLockfileMissing,
    BundleUnknownProtocol,
    BundleMissingInput,
    BundleExecutionFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParseIo => "PARSE_IO",
            Self::ParseInvalidToml => "PARSE_INVALID_TOML",
            Self::ParseInvalidArtifact => "PARSE_INVALID_ARTIFACT",
            Self::ParseInvalidTarget => "PARSE_INVALID_TARGET",
            Self::ParseInvalidBundle => "PARSE_INVALID_BUNDLE",
            Self::ArtifactMissingManifest => "ARTIFACT_MISSING_MANIFEST",
            Self::ArtifactMissingLabel => "ARTIFACT_MISSING_LABEL",
            Self::ArtifactUnsupportedTarget => "ARTIFACT_UNSUPPORTED_TARGET",
            Self::ArtifactRustupFailed => "ARTIFACT_RUSTUP_FAILED",
            Self::ArtifactCargoInvocationFailed => "ARTIFACT_CARGO_INVOCATION_FAILED",
            Self::ArtifactBuildFailed => "ARTIFACT_BUILD_FAILED",
            Self::ArtifactMissingPackageMetadata => "ARTIFACT_MISSING_PACKAGE_METADATA",
            Self::ArtifactArchiveFailed => "ARTIFACT_ARCHIVE_FAILED",
            Self::CargoLockfileMissing => "CARGO_LOCKFILE_MISSING",
            Self::BundleUnknownProtocol => "BUNDLE_UNKNOWN_PROTOCOL",
            Self::BundleMissingInput => "BUNDLE_MISSING_INPUT",
            Self::BundleExecutionFailed => "BUNDLE_EXECUTION_FAILED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunError {
    pub code: ErrorCode,
    pub message: String,
}

impl RunError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}
