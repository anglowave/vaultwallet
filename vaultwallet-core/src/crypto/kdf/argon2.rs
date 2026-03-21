use crate::error::WvError;
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Argon2Flavor {
	D,
	Id,
}

/// Derive the 32-byte transformed key using Argon2 parameters from the vault header.
pub fn argon2_derive(
	flavor: Argon2Flavor,
	password_hash: &[u8; 32],
	salt: &[u8],
	iterations: u32,
	memory_bytes: u64,
	parallelism: u32,
	version: u32,
) -> Result<Zeroizing<[u8; 32]>, WvError> {
	let mem_kib = u32::try_from(memory_bytes / 1024)
		.map_err(|_| WvError::InvalidFormat("Argon2 memory too large".into()))?;
	let params = Params::new(mem_kib, iterations, parallelism, Some(32))
		.map_err(|_| WvError::KeyDerivationFailed)?;
	let ver = match version {
		0x10 => Version::V0x10,
		0x13 => Version::V0x13,
		_ => return Err(WvError::InvalidFormat(format!("unknown Argon2 V: {version}"))),
	};
	let algo = match flavor {
		Argon2Flavor::D => Algorithm::Argon2d,
		Argon2Flavor::Id => Algorithm::Argon2id,
	};
	let argon2 = Argon2::new(algo, ver, params);
	let mut out = Zeroizing::new([0u8; 32]);
	argon2
		.hash_password_into(password_hash, salt, &mut *out)
		.map_err(|_| WvError::KeyDerivationFailed)?;
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;
	use argon2::Argon2;

	/// Smoke test: library produces stable output for fixed inputs.
	#[test]
	fn argon2d_deterministic_short() {
		let salt = [1u8; 16];
		let pwd = [2u8; 32];
		let params = Params::new(8, 2, 1, Some(32)).unwrap();
		let argon2 = Argon2::new(Algorithm::Argon2d, Version::V0x13, params);
		let mut out = [0u8; 32];
		argon2.hash_password_into(&pwd, &salt, &mut out).unwrap();
		let r = argon2_derive(
			Argon2Flavor::D,
			&pwd,
			&salt,
			2,
			8 * 1024,
			1,
			0x13,
		)
		.unwrap();
		assert_eq!(*r, out);
	}
}
