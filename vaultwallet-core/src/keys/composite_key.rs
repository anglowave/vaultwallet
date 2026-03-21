use crate::crypto::hash::sha256;
use crate::error::WvError;
use std::fs;
use std::path::Path;
use zeroize::{Zeroize, Zeroizing};

/// Builder for the vault composite key (password, optional key file).
#[derive(Clone)]
pub struct CompositeKey {
	password: Option<Zeroizing<Vec<u8>>>,
	key_file: Option<Zeroizing<Vec<u8>>>,
}

impl CompositeKey {
	pub fn new() -> Self {
		Self {
			password: None,
			key_file: None,
		}
	}

	pub fn with_password(mut self, password: &str) -> Self {
		let h = sha256(password.as_bytes());
		self.password = Some(Zeroizing::new(h.to_vec()));
		self
	}

	pub fn with_key_file(mut self, path: impl AsRef<Path>) -> Result<Self, WvError> {
		let raw = fs::read(path.as_ref())?;
		let key_material = Zeroizing::new(parse_key_file(&raw)?);
		self.key_file = Some(key_material);
		Ok(self)
	}

	/// Validates that at least one key source is present.
	pub fn build(self) -> Result<Self, WvError> {
		if self.password.is_none() && self.key_file.is_none() {
			return Err(WvError::InvalidFormat(
				"composite key requires password and/or key file".into(),
			));
		}
		Ok(self)
	}

	/// 32-byte hash fed to KDF (SHA256 of concatenated key parts).
	pub(crate) fn composite_hash(&self) -> [u8; 32] {
		let mut acc = Vec::new();
		if let Some(ref p) = self.password {
			acc.extend_from_slice(p);
		}
		if let Some(ref k) = self.key_file {
			acc.extend_from_slice(k);
		}
		sha256(&acc)
	}
}

impl Default for CompositeKey {
	fn default() -> Self {
		Self::new()
	}
}

impl Drop for CompositeKey {
	fn drop(&mut self) {
		if let Some(ref mut p) = self.password {
			p.zeroize();
		}
		if let Some(ref mut k) = self.key_file {
			k.zeroize();
		}
	}
}

fn parse_key_file(raw: &[u8]) -> Result<Vec<u8>, WvError> {
	if raw.len() == 32 {
		return Ok(raw.to_vec());
	}
	if raw.len() == 64 {
		if let Ok(s) = std::str::from_utf8(raw) {
			if s.chars().all(|c| c.is_ascii_hexdigit()) {
				let mut out = Vec::with_capacity(32);
				for chunk in s.as_bytes().chunks(2) {
					let hex = std::str::from_utf8(chunk)
						.map_err(|_| WvError::InvalidFormat("key file hex".into()))?;
					let b = u8::from_str_radix(hex, 16)
						.map_err(|_| WvError::InvalidFormat("key file hex digit".into()))?;
					out.push(b);
				}
				return Ok(out);
			}
		}
	}
	if raw.starts_with(b"<KeyFile>") {
		return parse_key_file_xml(raw);
	}
	Ok(sha256(raw).to_vec())
}

fn parse_key_file_xml(raw: &[u8]) -> Result<Vec<u8>, WvError> {
	let s = std::str::from_utf8(raw)
		.map_err(|_| WvError::InvalidFormat("key file XML not UTF-8".into()))?;
	let lower = s.to_ascii_lowercase();
	let data_start = lower
		.find("<data")
		.ok_or_else(|| WvError::InvalidFormat("key file XML missing Data".into()))?;
	let after = &s[data_start..];
	let gt = after
		.find('>')
		.ok_or_else(|| WvError::InvalidFormat("key file XML malformed".into()))?;
	let close = after[gt + 1..]
		.find("</Data>")
		.or_else(|| after[gt + 1..].find("</data>"))
		.ok_or_else(|| WvError::InvalidFormat("key file XML missing closing Data".into()))?;
	let inner = after[gt + 1..gt + 1 + close].trim();
	if inner.len() == 64 && inner.chars().all(|c| c.is_ascii_hexdigit()) {
		let mut out = Vec::with_capacity(32);
		for chunk in inner.as_bytes().chunks(2) {
			let hex = std::str::from_utf8(chunk).unwrap();
			out.push(
				u8::from_str_radix(hex, 16)
					.map_err(|_| WvError::InvalidFormat("key file hex".into()))?,
			);
		}
		return Ok(out);
	}
	use base64::Engine;
	let dec = base64::engine::general_purpose::STANDARD
		.decode(inner.as_bytes())
		.map_err(|_| WvError::InvalidFormat("key file base64".into()))?;
	if dec.len() != 32 {
		return Err(WvError::InvalidFormat("key file decoded length".into()));
	}
	Ok(dec)
}
