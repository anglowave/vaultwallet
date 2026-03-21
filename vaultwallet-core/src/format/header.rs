use crate::crypto::hash::{ct_eq_32, hmac_sha256, sha256, sha512, HEADER_HMAC_BLOCK_INDEX};
use crate::crypto::kdf::{aes_kdf_transform_checked, argon2_derive, Argon2Flavor};
use crate::error::WvError;
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

pub const OUTER_MAGIC: [u8; 8] = [0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5];

pub const HDR_END: u8 = 0;
pub const HDR_CIPHER: u8 = 2;
pub const HDR_COMPRESSION: u8 = 3;
pub const HDR_MASTER_SEED: u8 = 4;
pub const HDR_ENCRYPTION_IV: u8 = 7;
pub const HDR_KDF: u8 = 11;
pub const HDR_PUBLIC_CUSTOM: u8 = 12;

pub const KDF_AES: Uuid = uuid::uuid!("c9d9f39a-628a-4460-bf74-0d08c18a4fea");
pub const KDF_ARGON2D: Uuid = uuid::uuid!("ef636ddf-8c29-444b-91f7-a9a403e30a0c");
pub const KDF_ARGON2ID: Uuid = uuid::uuid!("9e298b19-56db-4773-b23d-fc3ec6f0a1e6");

#[derive(Clone, Debug)]
pub enum VariantValue {
	UInt32(u32),
	UInt64(u64),
	Int32(i32),
	Int64(i64),
	Bool(bool),
	String(String),
	Bytes(Vec<u8>),
}

#[derive(Clone, Debug, Default)]
pub struct VariantMap {
	pub version: u16,
	pub entries: HashMap<String, VariantValue>,
}

impl VariantMap {
	pub fn decode(raw: &[u8]) -> Result<Self, WvError> {
		if raw.len() < 2 {
			return Err(WvError::InvalidFormat("VariantMap too short".into()));
		}
		let version = u16::from_le_bytes([raw[0], raw[1]]);
		let mut i = 2;
		let mut entries = HashMap::new();
		while i < raw.len() {
			if raw[i] == 0 {
				break;
			}
			let ty = raw[i];
			i += 1;
			let key_len = read_u32(raw, &mut i)?;
			let key = read_bytes(raw, &mut i, key_len as usize)?;
			let key_s = String::from_utf8(key)
				.map_err(|_| WvError::InvalidFormat("VariantMap key not UTF-8".into()))?;
			let val_len = read_u32(raw, &mut i)?;
			let val_bytes = read_bytes(raw, &mut i, val_len as usize)?;
			let val = decode_variant(ty, &val_bytes)?;
			entries.insert(key_s, val);
		}
		Ok(VariantMap { version, entries })
	}

	pub fn encode(&self) -> Vec<u8> {
		let mut out = Vec::new();
		out.extend_from_slice(&self.version.to_le_bytes());
		let mut keys: Vec<_> = self.entries.keys().cloned().collect();
		keys.sort();
		for k in keys {
			let v = self.entries.get(&k).unwrap();
			encode_entry(&mut out, &k, v);
		}
		out.push(0);
		out
	}
}

fn decode_variant(ty: u8, bytes: &[u8]) -> Result<VariantValue, WvError> {
	match ty {
		0x04 => {
			if bytes.len() != 4 {
				return Err(WvError::InvalidFormat("Variant u32 size".into()));
			}
			Ok(VariantValue::UInt32(u32::from_le_bytes(bytes.try_into().unwrap())))
		}
		0x05 => {
			if bytes.len() != 8 {
				return Err(WvError::InvalidFormat("Variant u64 size".into()));
			}
			Ok(VariantValue::UInt64(u64::from_le_bytes(bytes.try_into().unwrap())))
		}
		0x08 => {
			if bytes.len() != 1 {
				return Err(WvError::InvalidFormat("Variant bool size".into()));
			}
			Ok(VariantValue::Bool(bytes[0] != 0))
		}
		0x0C => {
			if bytes.len() != 4 {
				return Err(WvError::InvalidFormat("Variant i32 size".into()));
			}
			Ok(VariantValue::Int32(i32::from_le_bytes(bytes.try_into().unwrap())))
		}
		0x0D => {
			if bytes.len() != 8 {
				return Err(WvError::InvalidFormat("Variant i64 size".into()));
			}
			Ok(VariantValue::Int64(i64::from_le_bytes(bytes.try_into().unwrap())))
		}
		0x18 => Ok(VariantValue::String(
			String::from_utf8(bytes.to_vec())
				.map_err(|_| WvError::InvalidFormat("Variant string not UTF-8".into()))?,
		)),
		0x42 => Ok(VariantValue::Bytes(bytes.to_vec())),
		_ => Err(WvError::InvalidFormat(format!("unknown VariantMap type {ty}"))),
	}
}

fn encode_entry(out: &mut Vec<u8>, key: &str, v: &VariantValue) {
	let (ty, payload): (u8, Vec<u8>) = match v {
		VariantValue::UInt32(x) => (0x04, x.to_le_bytes().to_vec()),
		VariantValue::UInt64(x) => (0x05, x.to_le_bytes().to_vec()),
		VariantValue::Bool(b) => (0x08, vec![if *b { 1 } else { 0 }]),
		VariantValue::Int32(x) => (0x0C, x.to_le_bytes().to_vec()),
		VariantValue::Int64(x) => (0x0D, x.to_le_bytes().to_vec()),
		VariantValue::String(s) => (0x18, s.as_bytes().to_vec()),
		VariantValue::Bytes(b) => (0x42, b.clone()),
	};
	out.push(ty);
	let kb = key.as_bytes();
	out.extend_from_slice(&(kb.len() as u32).to_le_bytes());
	out.extend_from_slice(kb);
	out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
	out.extend_from_slice(&payload);
}

fn read_u32(buf: &[u8], i: &mut usize) -> Result<u32, WvError> {
	if *i + 4 > buf.len() {
		return Err(WvError::InvalidFormat("truncated u32".into()));
	}
	let v = u32::from_le_bytes(buf[*i..*i + 4].try_into().unwrap());
	*i += 4;
	Ok(v)
}

fn read_bytes(buf: &[u8], i: &mut usize, len: usize) -> Result<Vec<u8>, WvError> {
	if *i + len > buf.len() {
		return Err(WvError::InvalidFormat("truncated bytes".into()));
	}
	let v = buf[*i..*i + len].to_vec();
	*i += len;
	Ok(v)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KdfParams {
	AesKdf { seed: [u8; 32], rounds: u64 },
	Argon2 {
		flavor: Argon2Flavor,
		salt: Vec<u8>,
		iterations: u32,
		memory_bytes: u64,
		parallelism: u32,
		version: u32,
	},
}

impl KdfParams {
	pub fn from_variant_map(m: &VariantMap) -> Result<Self, WvError> {
		let uuid_bytes = match m.entries.get("$UUID") {
			Some(VariantValue::Bytes(b)) if b.len() == 16 => {
				let mut a = [0u8; 16];
				a.copy_from_slice(b);
				Uuid::from_bytes(a)
			}
			_ => return Err(WvError::InvalidFormat("KDF missing $UUID".into())),
		};
		if uuid_bytes == KDF_AES {
			let rounds = match m.entries.get("R") {
				Some(VariantValue::UInt64(r)) => *r,
				_ => return Err(WvError::InvalidFormat("AES-KDF missing R".into())),
			};
			let seed = match m.entries.get("S") {
				Some(VariantValue::Bytes(b)) if b.len() == 32 => {
					let mut s = [0u8; 32];
					s.copy_from_slice(b);
					s
				}
				_ => return Err(WvError::InvalidFormat("AES-KDF missing S".into())),
			};
			Ok(KdfParams::AesKdf { seed, rounds })
		} else if uuid_bytes == KDF_ARGON2D || uuid_bytes == KDF_ARGON2ID {
			let salt = match m.entries.get("S") {
				Some(VariantValue::Bytes(b)) if !b.is_empty() => b.clone(),
				_ => return Err(WvError::InvalidFormat("Argon2 missing salt S".into())),
			};
			let iterations = match m.entries.get("I") {
				Some(VariantValue::UInt64(x)) => u32::try_from(*x)
					.map_err(|_| WvError::InvalidFormat("Argon2 I too large".into()))?,
				_ => return Err(WvError::InvalidFormat("Argon2 missing I".into())),
			};
			let memory_bytes = match m.entries.get("M") {
				Some(VariantValue::UInt64(x)) => *x,
				_ => return Err(WvError::InvalidFormat("Argon2 missing M".into())),
			};
			let parallelism = match m.entries.get("P") {
				Some(VariantValue::UInt32(x)) => *x,
				_ => return Err(WvError::InvalidFormat("Argon2 missing P".into())),
			};
			let version = match m.entries.get("V") {
				Some(VariantValue::UInt32(x)) => *x,
				_ => 0x13,
			};
			let flavor = if uuid_bytes == KDF_ARGON2D {
				Argon2Flavor::D
			} else {
				Argon2Flavor::Id
			};
			Ok(KdfParams::Argon2 {
				flavor,
				salt,
				iterations,
				memory_bytes,
				parallelism,
				version,
			})
		} else {
			Err(WvError::InvalidFormat("unsupported KDF UUID".into()))
		}
	}

	pub fn to_variant_map(&self) -> VariantMap {
		let mut m = VariantMap {
			version: 0x100,
			entries: HashMap::new(),
		};
		match self {
			KdfParams::AesKdf { seed, rounds } => {
				m.entries.insert(
					"$UUID".into(),
					VariantValue::Bytes(KDF_AES.as_bytes().to_vec()),
				);
				m.entries.insert("R".into(), VariantValue::UInt64(*rounds));
				m.entries.insert("S".into(), VariantValue::Bytes(seed.to_vec()));
			}
			KdfParams::Argon2 {
				flavor,
				salt,
				iterations,
				memory_bytes,
				parallelism,
				version,
			} => {
				let u = match flavor {
					Argon2Flavor::D => KDF_ARGON2D,
					Argon2Flavor::Id => KDF_ARGON2ID,
				};
				m.entries
					.insert("$UUID".into(), VariantValue::Bytes(u.as_bytes().to_vec()));
				m.entries.insert("S".into(), VariantValue::Bytes(salt.clone()));
				m.entries
					.insert("I".into(), VariantValue::UInt64(*iterations as u64));
				m.entries
					.insert("M".into(), VariantValue::UInt64(*memory_bytes));
				m.entries.insert("P".into(), VariantValue::UInt32(*parallelism));
				m.entries.insert("V".into(), VariantValue::UInt32(*version));
			}
		}
		m
	}
}

#[derive(Clone, Debug)]
pub struct OuterHeader {
	pub version_minor: u16,
	pub version_major: u16,
	pub cipher_uuid: Uuid,
	pub compression: u32,
	pub master_seed: [u8; 32],
	pub encryption_iv: Vec<u8>,
	pub kdf_parameters: VariantMap,
	pub public_custom_data: Option<VariantMap>,
	/// Raw bytes from file start through EndOfHeader field (inclusive).
	pub raw_pre_hash: Vec<u8>,
}

impl OuterHeader {
	pub fn parse(data: &[u8]) -> Result<(Self, usize), WvError> {
		if data.len() < 12 {
			return Err(WvError::InvalidFormat("file too short".into()));
		}
		if data[..8] != OUTER_MAGIC {
			return Err(WvError::InvalidSignature);
		}
		let version_minor = u16::from_le_bytes([data[8], data[9]]);
		let version_major = u16::from_le_bytes([data[10], data[11]]);
		if version_major != 4 {
			return Err(WvError::UnsupportedVersion);
		}
		let mut pos = 12;
		let mut cipher_uuid = None;
		let mut compression = 0u32;
		let mut master_seed = None;
		let mut encryption_iv = None;
		let mut kdf_parameters = None;
		let mut public_custom_data = None;
		loop {
			if pos >= data.len() {
				return Err(WvError::InvalidFormat("unterminated outer header".into()));
			}
			let field_ty = data[pos];
			pos += 1;
			let size = read_u32(data, &mut pos)? as usize;
			let payload = read_bytes(data, &mut pos, size)?;
			match field_ty {
				HDR_END => break,
				HDR_CIPHER => {
					if payload.len() != 16 {
						return Err(WvError::InvalidFormat("CipherID length".into()));
					}
					let mut b = [0u8; 16];
					b.copy_from_slice(&payload);
					cipher_uuid = Some(Uuid::from_bytes(b));
				}
				HDR_COMPRESSION => {
					if payload.len() != 4 {
						return Err(WvError::InvalidFormat("Compression length".into()));
					}
					compression = u32::from_le_bytes(payload[..4].try_into().unwrap());
				}
				HDR_MASTER_SEED => {
					if payload.len() != 32 {
						return Err(WvError::InvalidFormat("MasterSeed length".into()));
					}
					let mut s = [0u8; 32];
					s.copy_from_slice(&payload);
					master_seed = Some(s);
				}
				HDR_ENCRYPTION_IV => {
					encryption_iv = Some(payload);
				}
				HDR_KDF => {
					kdf_parameters = Some(VariantMap::decode(&payload)?);
				}
				HDR_PUBLIC_CUSTOM => {
					public_custom_data = Some(VariantMap::decode(&payload)?);
				}
				_ => { /* ignore unknown outer fields */ }
			}
		}
		let raw_pre_hash = data[..pos].to_vec();
		let cipher_uuid = cipher_uuid.ok_or_else(|| WvError::InvalidFormat("missing CipherID".into()))?;
		let master_seed = master_seed.ok_or_else(|| WvError::InvalidFormat("missing MasterSeed".into()))?;
		let encryption_iv = encryption_iv
			.ok_or_else(|| WvError::InvalidFormat("missing EncryptionIV".into()))?;
		let kdf_parameters =
			kdf_parameters.ok_or_else(|| WvError::InvalidFormat("missing KdfParameters".into()))?;
		Ok((
			OuterHeader {
				version_minor,
				version_major,
				cipher_uuid,
				compression,
				master_seed,
				encryption_iv,
				kdf_parameters,
				public_custom_data,
				raw_pre_hash,
			},
			pos,
		))
	}

	pub fn transformed_key(
		&self,
		composite_key_hash: &[u8; 32],
	) -> Result<Zeroizing<[u8; 32]>, WvError> {
		let kdf = KdfParams::from_variant_map(&self.kdf_parameters)?;
		match kdf {
			KdfParams::AesKdf { seed, rounds } => {
				aes_kdf_transform_checked(&seed, composite_key_hash, rounds)
			}
			KdfParams::Argon2 {
				flavor,
				ref salt,
				iterations,
				memory_bytes,
				parallelism,
				version,
			} => argon2_derive(
				flavor,
				composite_key_hash,
				salt,
				iterations,
				memory_bytes,
				parallelism,
				version,
			),
		}
	}

	pub fn derive_keys(
		&self,
		composite_key_hash: &[u8; 32],
	) -> Result<(Zeroizing<[u8; 32]>, Zeroizing<[u8; 64]>), WvError> {
		let mut transformed = self.transformed_key(composite_key_hash)?;
		let mut enc_input = Zeroizing::new(Vec::with_capacity(64));
		enc_input.extend_from_slice(&self.master_seed);
		enc_input.extend_from_slice(transformed.as_ref());
		let enc_key = Zeroizing::new(sha256(&enc_input));
		let mut hmac_input = Zeroizing::new(Vec::with_capacity(64 + 1));
		hmac_input.extend_from_slice(&self.master_seed);
		hmac_input.extend_from_slice(transformed.as_ref());
		hmac_input.push(1u8);
		let hmac_root = Zeroizing::new(sha512(&hmac_input));
		transformed.zeroize();
		enc_input.zeroize();
		hmac_input.zeroize();
		Ok((enc_key, hmac_root))
	}

	pub fn verify_header_hmac(
		&self,
		composite_key_hash: &[u8; 32],
		stored_hmac: &[u8; 32],
	) -> Result<(), WvError> {
		let (_enc, hmac_root) = self.derive_keys(composite_key_hash)?;
		let block_key = crate::crypto::hash::block_hmac_key(hmac_root.as_ref(), HEADER_HMAC_BLOCK_INDEX)?;
		let mut msg = Vec::with_capacity(8 + 4 + self.raw_pre_hash.len());
		msg.extend_from_slice(&HEADER_HMAC_BLOCK_INDEX.to_le_bytes());
		msg.extend_from_slice(&(self.raw_pre_hash.len() as u32).to_le_bytes());
		msg.extend_from_slice(&self.raw_pre_hash);
		let calc = hmac_sha256(block_key.as_ref(), &msg);
		if !ct_eq_32(&calc, stored_hmac) {
			return Err(WvError::HmacMismatch);
		}
		Ok(())
	}

	pub fn compute_header_hmac(
		&self,
		composite_key_hash: &[u8; 32],
	) -> Result<[u8; 32], WvError> {
		let (_enc, hmac_root) = self.derive_keys(composite_key_hash)?;
		let block_key = crate::crypto::hash::block_hmac_key(hmac_root.as_ref(), HEADER_HMAC_BLOCK_INDEX)?;
		let mut msg = Vec::with_capacity(8 + 4 + self.raw_pre_hash.len());
		msg.extend_from_slice(&HEADER_HMAC_BLOCK_INDEX.to_le_bytes());
		msg.extend_from_slice(&(self.raw_pre_hash.len() as u32).to_le_bytes());
		msg.extend_from_slice(&self.raw_pre_hash);
		Ok(hmac_sha256(block_key.as_ref(), &msg))
	}

	pub fn header_sha256(&self) -> [u8; 32] {
		sha256(&self.raw_pre_hash)
	}
}

/// Build outer header bytes (through EndOfHeader) for writing.
pub fn build_outer_header_bytes(
	version_minor: u16,
	version_major: u16,
	cipher_uuid: Uuid,
	compression: u32,
	master_seed: &[u8; 32],
	encryption_iv: &[u8],
	kdf: &VariantMap,
	public_custom: Option<&VariantMap>,
) -> Vec<u8> {
	let mut out = Vec::new();
	out.extend_from_slice(&OUTER_MAGIC);
	out.extend_from_slice(&version_minor.to_le_bytes());
	out.extend_from_slice(&version_major.to_le_bytes());
	push_field(&mut out, HDR_CIPHER, cipher_uuid.as_bytes());
	push_field(
		&mut out,
		HDR_COMPRESSION,
		&compression.to_le_bytes(),
	);
	push_field(&mut out, HDR_MASTER_SEED, master_seed);
	push_field(&mut out, HDR_ENCRYPTION_IV, encryption_iv);
	let kdf_enc = kdf.encode();
	push_field(&mut out, HDR_KDF, &kdf_enc);
	if let Some(pc) = public_custom {
		push_field(&mut out, HDR_PUBLIC_CUSTOM, &pc.encode());
	}
	push_field(&mut out, HDR_END, b"\r\n\r\n");
	out
}

fn push_field(out: &mut Vec<u8>, ty: u8, data: &[u8]) {
	out.push(ty);
	out.extend_from_slice(&(data.len() as u32).to_le_bytes());
	out.extend_from_slice(data);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn variant_map_round_trip() {
		let mut m = VariantMap::default();
		m.version = 0x100;
		m.entries
			.insert("a".into(), VariantValue::UInt32(42));
		m.entries
			.insert("b".into(), VariantValue::Bytes(vec![1, 2, 3]));
		let enc = m.encode();
		let dec = VariantMap::decode(&enc).unwrap();
		assert_eq!(dec.version, m.version);
		assert_eq!(dec.entries.len(), m.entries.len());
	}
}
