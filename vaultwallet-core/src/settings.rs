use crate::crypto::cipher::CipherId;
use crate::format::header::KdfParams;

/// Per-file crypto and format options (algorithm choices persist; salts/seeds rotate on save).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultSettings {
	pub format_version_minor: u16,
	pub format_version_major: u16,
	pub cipher: CipherId,
	pub compression_gzip: bool,
	pub kdf: KdfParams,
}
