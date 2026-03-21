use crate::error::WvError;
use aes::cipher::generic_array::typenum::U16;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncryptMut, KeyInit};
use aes::Aes256;
use zeroize::Zeroizing;

/// Legacy AES-KDF: AES-256-ECB on the 32-byte composite hash (two 16-byte blocks per round).
pub fn aes_kdf_transform(seed: &[u8; 32], composite_hash: &[u8; 32], rounds: u64) -> Zeroizing<[u8; 32]> {
	let key = GenericArray::clone_from_slice(seed);
	let mut cipher = Aes256::new(&key);
	let mut buf = *composite_hash;
	for _ in 0..rounds {
		for chunk in buf.chunks_mut(16) {
			let mut block = GenericArray::<u8, U16>::clone_from_slice(chunk);
			cipher.encrypt_block_mut(&mut block);
			chunk.copy_from_slice(block.as_slice());
		}
	}
	Zeroizing::new(buf)
}

/// Same as `aes_kdf_transform` but validates round count for opening untrusted files.
pub fn aes_kdf_transform_checked(
	seed: &[u8; 32],
	composite_hash: &[u8; 32],
	rounds: u64,
) -> Result<Zeroizing<[u8; 32]>, WvError> {
	if rounds > 1_000_000_000 {
		return Err(WvError::InvalidFormat("AES-KDF rounds unreasonably large".into()));
	}
	Ok(aes_kdf_transform(seed, composite_hash, rounds))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn aes_kdf_zero_rounds_is_identity() {
		let seed = [7u8; 32];
		let h = [9u8; 32];
		let out = aes_kdf_transform(&seed, &h, 0);
		assert_eq!(*out, h);
	}
}
