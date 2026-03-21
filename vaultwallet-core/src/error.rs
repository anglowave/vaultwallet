use std::fmt;
use std::io;

/// Errors returned by VaultWallet database operations.
#[derive(Debug)]
pub enum WvError {
	/// Path must use the `.wlvlt` extension for normal open/save.
	InvalidExtension { found: String },
	/// File does not begin with the expected VaultWallet outer signature.
	InvalidSignature,
	/// Outer format major/minor version is not supported.
	UnsupportedVersion,
	/// Header or block HMAC did not verify (wrong key or tampering).
	HmacMismatch,
	/// Key derivation failed (e.g. invalid parameters).
	KeyDerivationFailed,
	/// Symmetric decryption failed (wrong key or corrupt ciphertext).
	DecryptionFailed,
	/// Malformed structure; includes a short context message.
	InvalidFormat(String),
	Io(io::Error),
}

impl fmt::Display for WvError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			WvError::InvalidExtension { found } => write!(
				f,
				"invalid vault file extension (expected .wlvlt): {found}"
			),
			WvError::InvalidSignature => {
				write!(f, "invalid vault file signature")
			}
			WvError::UnsupportedVersion => {
				write!(f, "unsupported vault file format version")
			}
			WvError::HmacMismatch => write!(f, "authentication failed (HMAC mismatch)"),
			WvError::KeyDerivationFailed => write!(f, "key derivation failed"),
			WvError::DecryptionFailed => write!(f, "decryption failed"),
			WvError::InvalidFormat(msg) => write!(f, "invalid vault format: {msg}"),
			WvError::Io(e) => write!(f, "I/O error: {e}"),
		}
	}
}

impl std::error::Error for WvError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			WvError::Io(e) => Some(e),
			_ => None,
		}
	}
}

impl From<io::Error> for WvError {
	fn from(e: io::Error) -> Self {
		WvError::Io(e)
	}
}
