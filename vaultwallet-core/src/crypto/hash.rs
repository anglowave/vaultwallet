use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;

/// Block index used for outer-header HMAC in KDBX4-style framing.
pub const HEADER_HMAC_BLOCK_INDEX: u64 = u64::MAX;

pub fn sha256(data: &[u8]) -> [u8; 32] {
	let mut h = Sha256::new();
	h.update(data);
	h.finalize().into()
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
	let mut h = Sha512::new();
	h.update(data);
	h.finalize().into()
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
	let mut mac = HmacSha256::new_from_slice(key)
		.expect("HMAC key length is valid for sha256");
	mac.update(data);
	let out = mac.finalize().into_bytes();
	let mut r = [0u8; 32];
	r.copy_from_slice(&out);
	r
}

/// HMAC key for a given block index (KDBX4): SHA512(index_le || hmac_root).
pub fn block_hmac_key(hmac_root: &[u8], block_index: u64) -> Result<Zeroizing<[u8; 64]>, crate::error::WvError> {
	if hmac_root.len() != 64 {
		return Err(crate::error::WvError::InvalidFormat(
			"HMAC root must be 64 bytes".into(),
		));
	}
	let mut buf = [0u8; 8 + 64];
	buf[..8].copy_from_slice(&block_index.to_le_bytes());
	buf[8..].copy_from_slice(hmac_root);
	Ok(Zeroizing::new(sha512(&buf)))
}

pub fn ct_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
	a.ct_eq(b).into()
}

pub fn ct_eq_slice_32(a: &[u8], b: &[u8]) -> bool {
	if a.len() != 32 || b.len() != 32 {
		return false;
	}
	let mut aa = [0u8; 32];
	let mut bb = [0u8; 32];
	aa.copy_from_slice(a);
	bb.copy_from_slice(b);
	ct_eq_32(&aa, &bb)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hmac_rfc4231_case1() {
		let key = [0x0b_u8; 20];
		let data = b"Hi There";
		let got = hmac_sha256(&key, data);
		let exp: [u8; 32] = [
			0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce,
			0xaf, 0x0b, 0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7,
			0x26, 0xe9, 0x37, 0x6c, 0x2e, 0x32, 0xcf, 0xf7,
		];
		assert!(ct_eq_32(&got, &exp));
	}
}
