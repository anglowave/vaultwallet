//! Biometric unlock (Windows Hello / Touch ID).
//!
//! ## Design
//!
//! A biometric authenticator cannot "store a password" on its own. Instead we
//! bind a wrapped copy of the master password to a biometric-protected
//! credential:
//!
//! 1. **Enroll** (only after a normal password unlock): create a platform
//!    credential (Windows Hello), sign a fixed random *challenge* with it. The
//!    signature is deterministic for a given key+challenge, so we derive a
//!    32-byte key from `SHA-256(signature)` and AES-256-GCM-encrypt the master
//!    password. We persist `{credential, challenge, nonce, ciphertext}` only —
//!    never the password or the derived key.
//! 2. **Unlock**: re-sign the *same* challenge. This prompts the user for their
//!    biometric/PIN. We derive the same key and decrypt the password.
//!
//! Without a successful biometric prompt there is no signature, no key, and the
//! ciphertext is useless. Because every sign requires user presence, local
//! malware cannot silently unwrap the secret.
//!
//! Non-Windows platforms currently report [`BioError::Unsupported`]; the command
//! surface and storage format are platform-neutral so macOS Touch ID (via the
//! Security framework / Keychain) can be added later without UI changes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const WRAP_VERSION: u32 = 1;
const CHALLENGE_LEN: usize = 32;
const NONCE_LEN: usize = 12;

#[derive(Debug)]
pub enum BioError {
	/// No biometric support on this platform / device.
	#[cfg_attr(target_os = "windows", allow(dead_code))]
	Unsupported,
	/// No wrapped secret stored for this vault.
	NotEnrolled,
	/// The user cancelled or failed the biometric prompt.
	Canceled,
	Io(String),
	Crypto(String),
	Other(String),
}

impl std::fmt::Display for BioError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BioError::Unsupported => {
				write!(f, "Biometric unlock is not available on this device")
			}
			BioError::NotEnrolled => {
				write!(f, "Biometric unlock is not set up for this vault")
			}
			BioError::Canceled => write!(f, "Biometric prompt was cancelled"),
			BioError::Io(m) => write!(f, "{m}"),
			BioError::Crypto(m) => write!(f, "{m}"),
			BioError::Other(m) => write!(f, "{m}"),
		}
	}
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for BioError {
	fn from(e: windows::core::Error) -> Self {
		BioError::Other(format!("Windows Hello error: {e}"))
	}
}

/// Stored, on-disk wrapped secret. Contains no plaintext key material.
#[derive(Serialize, Deserialize)]
struct WrappedSecret {
	version: u32,
	/// Platform credential name used to produce the signing key.
	credential: String,
	/// Fixed data that is re-signed on every unlock (base58).
	challenge: String,
	/// AES-256-GCM nonce (base58).
	nonce: String,
	/// Encrypted master password (base58).
	ciphertext: String,
}

fn to_hex(bytes: &[u8]) -> String {
	let mut s = String::with_capacity(bytes.len() * 2);
	for b in bytes {
		s.push_str(&format!("{b:02x}"));
	}
	s
}

/// Stable identifier for a vault path (case-insensitive, `.wlvlt` normalized).
fn vault_id(path: &str) -> String {
	use sha2::{Digest, Sha256};
	let t = path.trim().to_ascii_lowercase();
	let norm = if t.ends_with(".wlvlt") {
		t
	} else {
		format!("{t}.wlvlt")
	};
	let mut h = Sha256::new();
	h.update(norm.as_bytes());
	to_hex(&h.finalize())
}

fn credential_name(id: &str) -> String {
	format!("VaultWallet-{id}")
}

fn store_dir(app: &AppHandle) -> Result<PathBuf, BioError> {
	let dir = app
		.path()
		.app_local_data_dir()
		.map_err(|e| BioError::Io(format!("cannot resolve app data dir: {e}")))?
		.join("biometric");
	std::fs::create_dir_all(&dir)
		.map_err(|e| BioError::Io(format!("cannot create store dir: {e}")))?;
	Ok(dir)
}

fn blob_path(app: &AppHandle, id: &str) -> Result<PathBuf, BioError> {
	Ok(store_dir(app)?.join(format!("{id}.json")))
}

fn b58(bytes: &[u8]) -> String {
	bs58::encode(bytes).into_string()
}

fn unb58(s: &str) -> Result<Vec<u8>, BioError> {
	bs58::decode(s)
		.into_vec()
		.map_err(|e| BioError::Crypto(format!("corrupt stored secret: {e}")))
}

fn random_bytes(n: usize) -> Vec<u8> {
	use rand::RngCore;
	let mut v = vec![0u8; n];
	rand::thread_rng().fill_bytes(&mut v);
	v
}

fn derive_key(signature: &[u8]) -> [u8; 32] {
	use sha2::{Digest, Sha256};
	let mut h = Sha256::new();
	h.update(signature);
	let out = h.finalize();
	let mut key = [0u8; 32];
	key.copy_from_slice(&out);
	key
}

fn encrypt(key: &[u8; 32], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, BioError> {
	use aes_gcm::aead::{Aead, KeyInit};
	use aes_gcm::{Aes256Gcm, Nonce};
	let cipher = Aes256Gcm::new(key.into());
	cipher
		.encrypt(Nonce::from_slice(nonce), plaintext)
		.map_err(|_| BioError::Crypto("failed to wrap secret".into()))
}

fn decrypt(key: &[u8; 32], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, BioError> {
	use aes_gcm::aead::{Aead, KeyInit};
	use aes_gcm::{Aes256Gcm, Nonce};
	let cipher = Aes256Gcm::new(key.into());
	cipher
		.decrypt(Nonce::from_slice(nonce), ciphertext)
		.map_err(|_| BioError::Crypto("biometric unlock failed (key changed or data corrupt)".into()))
}

// ---------------------------------------------------------------------------
// Platform-neutral entry points
// ---------------------------------------------------------------------------

pub fn is_enrolled(app: &AppHandle, path: &str) -> Result<bool, BioError> {
	let id = vault_id(path);
	Ok(blob_path(app, &id)?.exists())
}

pub fn enroll(app: &AppHandle, path: &str, password: &str) -> Result<(), BioError> {
	let id = vault_id(path);
	let name = credential_name(&id);
	let challenge = random_bytes(CHALLENGE_LEN);

	let signature = platform_sign(&name, &challenge, /* create */ true)?;
	let key = derive_key(&signature);
	let nonce = random_bytes(NONCE_LEN);
	let ciphertext = encrypt(&key, &nonce, password.as_bytes())?;

	let wrapped = WrappedSecret {
		version: WRAP_VERSION,
		credential: name,
		challenge: b58(&challenge),
		nonce: b58(&nonce),
		ciphertext: b58(&ciphertext),
	};
	let json = serde_json::to_string_pretty(&wrapped)
		.map_err(|e| BioError::Io(format!("serialize secret: {e}")))?;
	std::fs::write(blob_path(app, &id)?, json)
		.map_err(|e| BioError::Io(format!("write secret: {e}")))?;
	Ok(())
}

pub fn unlock(app: &AppHandle, path: &str) -> Result<String, BioError> {
	let id = vault_id(path);
	let file = blob_path(app, &id)?;
	if !file.exists() {
		return Err(BioError::NotEnrolled);
	}
	let json = std::fs::read_to_string(&file)
		.map_err(|e| BioError::Io(format!("read secret: {e}")))?;
	let wrapped: WrappedSecret = serde_json::from_str(&json)
		.map_err(|e| BioError::Crypto(format!("corrupt stored secret: {e}")))?;
	if wrapped.version != WRAP_VERSION {
		return Err(BioError::Crypto("unsupported stored secret version".into()));
	}

	let challenge = unb58(&wrapped.challenge)?;
	let nonce = unb58(&wrapped.nonce)?;
	let ciphertext = unb58(&wrapped.ciphertext)?;

	let signature = platform_sign(&wrapped.credential, &challenge, /* create */ false)?;
	let key = derive_key(&signature);
	let plaintext = decrypt(&key, &nonce, &ciphertext)?;
	String::from_utf8(plaintext)
		.map_err(|_| BioError::Crypto("decrypted secret is not valid UTF-8".into()))
}

pub fn disable(app: &AppHandle, path: &str) -> Result<(), BioError> {
	let id = vault_id(path);
	let file = blob_path(app, &id)?;
	if file.exists() {
		std::fs::remove_file(&file)
			.map_err(|e| BioError::Io(format!("remove secret: {e}")))?;
	}
	// Best effort: also drop the platform credential.
	platform_delete(&credential_name(&id));
	Ok(())
}

pub fn available() -> bool {
	platform_available()
}

// ---------------------------------------------------------------------------
// Windows Hello implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn with_com<T>(f: impl FnOnce() -> Result<T, BioError>) -> Result<T, BioError> {
	use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
	// Each call runs on a dedicated blocking thread (see lib.rs), so initializing
	// the apartment here and balancing it on exit keeps the thread clean.
	let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
	let must_uninit = hr.is_ok();
	let result = f();
	if must_uninit {
		unsafe { CoUninitialize() };
	}
	result
}

#[cfg(target_os = "windows")]
fn platform_available() -> bool {
	with_com(|| {
		use windows::Security::Credentials::KeyCredentialManager;
		let supported = KeyCredentialManager::IsSupportedAsync()?.get()?;
		Ok(supported)
	})
	.unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn platform_sign(name: &str, challenge: &[u8], create: bool) -> Result<Vec<u8>, BioError> {
	with_com(|| {
		use windows::core::{Array, HSTRING};
		use windows::Security::Credentials::{
			KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
		};
		use windows::Security::Cryptography::CryptographicBuffer;

		let hname = HSTRING::from(name);
		let retrieval = if create {
			KeyCredentialManager::RequestCreateAsync(
				&hname,
				KeyCredentialCreationOption::ReplaceExisting,
			)?
			.get()?
		} else {
			KeyCredentialManager::OpenAsync(&hname)?.get()?
		};

		match retrieval.Status()? {
			KeyCredentialStatus::Success => {}
			KeyCredentialStatus::UserCanceled => return Err(BioError::Canceled),
			KeyCredentialStatus::NotFound => return Err(BioError::NotEnrolled),
			s => return Err(BioError::Other(format!("Windows Hello unavailable ({s:?})"))),
		}

		let credential = retrieval.Credential()?;
		let challenge_buf = CryptographicBuffer::CreateFromByteArray(challenge)?;
		let sign_result = credential.RequestSignAsync(&challenge_buf)?.get()?;

		match sign_result.Status()? {
			KeyCredentialStatus::Success => {}
			KeyCredentialStatus::UserCanceled => return Err(BioError::Canceled),
			s => return Err(BioError::Other(format!("Windows Hello sign failed ({s:?})"))),
		}

		let out_buf = sign_result.Result()?;
		let mut arr = Array::<u8>::new();
		CryptographicBuffer::CopyToByteArray(&out_buf, &mut arr)?;
		Ok(arr.as_slice().to_vec())
	})
}

#[cfg(target_os = "windows")]
fn platform_delete(name: &str) {
	let _ = with_com(|| {
		use windows::core::HSTRING;
		use windows::Security::Credentials::KeyCredentialManager;
		KeyCredentialManager::DeleteAsync(&HSTRING::from(name))?.get()?;
		Ok::<(), BioError>(())
	});
}

// ---------------------------------------------------------------------------
// Stubs for platforms without an implementation yet
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
fn platform_available() -> bool {
	false
}

#[cfg(not(target_os = "windows"))]
fn platform_sign(_name: &str, _challenge: &[u8], _create: bool) -> Result<Vec<u8>, BioError> {
	Err(BioError::Unsupported)
}

#[cfg(not(target_os = "windows"))]
fn platform_delete(_name: &str) {}
