//! VaultWallet core: KDBX 4.1–compatible vault file encryption and parsing.
//!
//! On disk the format matches the standard KDBX4 binary layout; only the preferred
//! file extension and user-facing naming differ (`.wlvlt`).

#![forbid(unsafe_code)]

mod error;
pub mod crypto;
pub mod db;
pub mod format;
pub mod keys;
mod settings;

pub use crypto::cipher::CipherId;
pub use crypto::kdf::Argon2Flavor;
pub use db::{Database, Entry, Group, Metadata};
pub use error::WvError;
pub use format::header::KdfParams;
pub use keys::CompositeKey;
pub use settings::VaultSettings;

use crate::crypto::cipher::{decrypt_payload, encrypt_payload, CipherId as CId};
use crate::crypto::hash::ct_eq_32;
use crate::crypto::random::random_bytes;
use crate::format::header::{build_outer_header_bytes, OuterHeader};
use crate::format::hmac_block::{read_hmac_block_stream, write_hmac_block_stream};
use crate::format::inner_header::{InnerHeader, StreamCipherId};
use crate::format::xml::{decode_protected_xml, parse_database_xml, serialize_database};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Preferred vault file extension without the dot (`wlvlt`).
pub const DB_EXTENSION: &str = "wlvlt";
/// Preferred vault file extension including the dot (`.wlvlt`).
pub const DB_EXTENSION_WITH_DOT: &str = ".wlvlt";

/// Validated path that ends with `.wlvlt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WlvltPath(PathBuf);

impl WlvltPath {
	pub fn new(p: impl AsRef<Path>) -> Result<Self, WvError> {
		let p = p.as_ref();
		let lossy = p.as_os_str().to_string_lossy().to_ascii_lowercase();
		if !lossy.ends_with(DB_EXTENSION_WITH_DOT) {
			return Err(WvError::InvalidExtension {
				found: p.display().to_string(),
			});
		}
		Ok(WlvltPath(p.to_path_buf()))
	}

	/// Append `.wlvlt` to `stem` (replacing any existing extension).
	pub fn with_extension(stem: impl AsRef<Path>) -> Self {
		let stem = stem.as_ref();
		let base: PathBuf = stem
			.file_stem()
			.map(Path::new)
			.unwrap_or(stem)
			.to_path_buf();
		WlvltPath(base.with_extension(DB_EXTENSION))
	}

	pub fn as_path(&self) -> &Path {
		&self.0
	}
}

fn path_ends_with_wlvlt(path: &Path) -> bool {
	path
		.as_os_str()
		.to_string_lossy()
		.to_ascii_lowercase()
		.ends_with(DB_EXTENSION_WITH_DOT)
}

fn normalized_wlvlt_path(path: &Path) -> PathBuf {
	if path_ends_with_wlvlt(path) {
		path.to_path_buf()
	} else {
		path.with_extension(DB_EXTENSION)
	}
}

/// Open an existing vault file. The path must end in `.wlvlt`.
pub fn open(path: impl AsRef<Path>, key: CompositeKey) -> Result<Database, WvError> {
	let path = path.as_ref();
	if !path_ends_with_wlvlt(path) {
		return Err(WvError::InvalidExtension {
			found: path.display().to_string(),
		});
	}
	let mut f = File::open(path)?;
	let mut data = Vec::new();
	f.read_to_end(&mut data)?;
	open_bytes(&data, &key)
}

fn open_bytes(data: &[u8], key: &CompositeKey) -> Result<Database, WvError> {
	let ch = key.composite_hash();
	let (outer, mut pos) = OuterHeader::parse(data)?;
	let stored_hash: [u8; 32] = read_fixed32(data, &mut pos)?;
	if !ct_eq_32(&outer.header_sha256(), &stored_hash) {
		return Err(WvError::HmacMismatch);
	}
	let stored_hmac: [u8; 32] = read_fixed32(data, &mut pos)?;
	outer.verify_header_hmac(&ch, &stored_hmac)?;
	let hmac_body = &data[pos..];
	let (enc_key, hmac_root) = outer.derive_keys(&ch)?;
	let cipher = CId::from_uuid(&outer.cipher_uuid)
		.ok_or_else(|| WvError::InvalidFormat("unsupported cipher UUID".into()))?;
	let dec = decrypt_payload(cipher, enc_key.as_ref(), &outer.encryption_iv, &read_hmac_block_stream(
		hmac_body,
		hmac_root.as_ref(),
	)?)?;
	let plain = decompress_if_needed(outer.compression, &dec)?;
	let (inner, inner_len) = InnerHeader::parse(&plain)?;
	let xml_part = std::str::from_utf8(&plain[inner_len..])
		.map_err(|_| WvError::InvalidFormat("inner XML not UTF-8".into()))?;
	let sk = inner
		.stream_key
		.as_deref()
		.ok_or_else(|| WvError::InvalidFormat("missing inner stream key".into()))?;
	let xml_clear = match inner.stream_cipher {
		Some(StreamCipherId::ChaCha20) => decode_protected_xml(xml_part, sk)?,
		None => xml_part.to_string(),
		_ => {
			return Err(WvError::InvalidFormat(
				"unsupported inner stream cipher".into(),
			))
		}
	};
	let mut db = parse_database_xml(&xml_clear)?;
	let kdf = KdfParams::from_variant_map(&outer.kdf_parameters)?;
	db.settings = VaultSettings {
		format_version_minor: outer.version_minor,
		format_version_major: outer.version_major,
		cipher,
		compression_gzip: outer.compression == 1,
		kdf,
	};
	Ok(db)
}

fn read_fixed32(data: &[u8], pos: &mut usize) -> Result<[u8; 32], WvError> {
	if *pos + 32 > data.len() {
		return Err(WvError::InvalidFormat("truncated header hash/HMAC".into()));
	}
	let mut a = [0u8; 32];
	a.copy_from_slice(&data[*pos..*pos + 32]);
	*pos += 32;
	Ok(a)
}

fn decompress_if_needed(flags: u32, data: &[u8]) -> Result<Vec<u8>, WvError> {
	match flags {
		0 => Ok(data.to_vec()),
		1 => {
			use flate2::read::GzDecoder;
			let mut dec = GzDecoder::new(data);
			let mut out = Vec::new();
			dec
				.read_to_end(&mut out)
				.map_err(|_| WvError::InvalidFormat("gzip decompress".into()))?;
			Ok(out)
		}
		_ => Err(WvError::InvalidFormat("unknown compression flag".into())),
	}
}

fn compress_if_needed(gzip: bool, data: &[u8]) -> Result<Vec<u8>, WvError> {
	if !gzip {
		return Ok(data.to_vec());
	}
	use flate2::write::GzEncoder;
	use flate2::Compression;
	let mut enc = GzEncoder::new(Vec::new(), Compression::default());
	enc
		.write_all(data)
		.map_err(|e| WvError::Io(e))?;
	let b = enc.finish().map_err(|e| WvError::Io(e))?;
	Ok(b)
}

/// Save a vault. Appends `.wlvlt` when missing, writes atomically via a `.tmp` file,
/// and generates a new master seed, IV, KDF salt/seed, and inner stream key each time.
pub fn save(db: &Database, path: impl AsRef<Path>, key: CompositeKey) -> Result<(), WvError> {
	let dest = normalized_wlvlt_path(path.as_ref());
	let tmp = dest.with_extension(format!("{DB_EXTENSION}.tmp"));
	let data = save_bytes(db, &key)?;
	let mut f = File::create(&tmp)?;
	f.write_all(&data)?;
	drop(f);
	if dest.exists() {
		fs::remove_file(&dest).ok();
	}
	fs::rename(&tmp, &dest)?;
	Ok(())
}

fn save_bytes(db: &Database, key: &CompositeKey) -> Result<Vec<u8>, WvError> {
	let ch = key.composite_hash();
	let mut settings = db.settings.clone();
	refresh_crypto_material(&mut settings);

	let mut stream_key = Zeroizing::new([0u8; 64]);
	random_bytes(&mut *stream_key);
	let inner = InnerHeader {
		stream_cipher: Some(StreamCipherId::ChaCha20),
		stream_key: Some(stream_key.to_vec()),
		binaries: Vec::new(),
	};
	let inner_bytes = inner.encode();
	let xml = serialize_database(db, stream_key.as_ref())?;
	let mut payload = inner_bytes;
	payload.extend_from_slice(xml.as_bytes());
	let compressed = compress_if_needed(settings.compression_gzip, &payload)?;

	let mut master_seed = [0u8; 32];
	random_bytes(&mut master_seed);
	let mut iv = vec![0u8; if settings.cipher == CId::ChaCha20 { 12 } else { 16 }];
	random_bytes(&mut iv);

	let kdf_map = settings.kdf.to_variant_map();
	let outer_raw = build_outer_header_bytes(
		settings.format_version_minor,
		settings.format_version_major,
		settings.cipher.uuid(),
		if settings.compression_gzip { 1 } else { 0 },
		&master_seed,
		&iv,
		&kdf_map,
		None,
	);
	let outer = OuterHeader {
		version_minor: settings.format_version_minor,
		version_major: settings.format_version_major,
		cipher_uuid: settings.cipher.uuid(),
		compression: if settings.compression_gzip { 1 } else { 0 },
		master_seed,
		encryption_iv: iv.clone(),
		kdf_parameters: kdf_map,
		public_custom_data: None,
		raw_pre_hash: outer_raw.clone(),
	};
	let header_hash = outer.header_sha256();
	let hmac = outer.compute_header_hmac(&ch)?;
	let (enc_key, hmac_root) = outer.derive_keys(&ch)?;
	let ct = encrypt_payload(settings.cipher, enc_key.as_ref(), &outer.encryption_iv, &compressed)?;
	let blocks = write_hmac_block_stream(&ct, hmac_root.as_ref())?;

	let mut out = outer_raw;
	out.extend_from_slice(&header_hash);
	out.extend_from_slice(&hmac);
	out.extend_from_slice(&blocks);
	Ok(out)
}

fn refresh_crypto_material(s: &mut VaultSettings) {
	let mut new_seed = [0u8; 32];
	random_bytes(&mut new_seed);
	match &mut s.kdf {
		KdfParams::AesKdf { seed, .. } => *seed = new_seed,
		KdfParams::Argon2 { salt, .. } => {
			if salt.len() < 16 {
				salt.resize(32, 0);
			}
			random_bytes(salt);
		}
	}
}

/// One-time migration: read a legacy binary vault with any extension and write `.wlvlt`.
pub fn import_kdbx(
	kdbx_path: impl AsRef<Path>,
	out_path: impl AsRef<Path>,
	key: CompositeKey,
) -> Result<(), WvError> {
	let out = out_path.as_ref();
	if !path_ends_with_wlvlt(out) {
		return Err(WvError::InvalidExtension {
			found: out.display().to_string(),
		});
	}
	let mut f = File::open(kdbx_path.as_ref())?;
	let mut data = Vec::new();
	f.read_to_end(&mut data)?;
	let db = open_bytes(&data, &key)?;
	save(&db, out, key)
}

#[cfg(test)]
mod wlvlt_path_tests {
	use super::*;

	#[test]
	fn wlvlt_path_new_accepts() {
		let p = PathBuf::from("x.wlvlt");
		assert!(WlvltPath::new(&p).is_ok());
	}

	#[test]
	fn wlvlt_path_new_rejects() {
		let p = PathBuf::from("x.kdbx");
		let e = WlvltPath::new(&p).unwrap_err();
		match e {
			WvError::InvalidExtension { .. } => {}
			_ => panic!("expected InvalidExtension"),
		}
	}
}
