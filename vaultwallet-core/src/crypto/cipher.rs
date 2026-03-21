use crate::error::WvError;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes256;
use chacha20::cipher::StreamCipher;
use chacha20::ChaCha20;
use cbc::{Decryptor, Encryptor};
use twofish::Twofish;
use uuid::Uuid;
use zeroize::Zeroizing;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;
type TwofishCbcEnc = Encryptor<Twofish>;
type TwofishCbcDec = Decryptor<Twofish>;

/// Symmetric cipher UUIDs (KDBX-compatible wire format).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CipherId {
	Aes256Cbc,
	ChaCha20,
	TwofishCbc,
}

impl CipherId {
	pub fn from_uuid(u: &Uuid) -> Option<Self> {
		if *u == uuid::uuid!("31c1f2e6-bf71-4350-be58-05216afc5aff") {
			Some(CipherId::Aes256Cbc)
		} else if *u == uuid::uuid!("d6038a2b-8b6f-4cb5-a524-339a31dbb59a") {
			Some(CipherId::ChaCha20)
		} else if *u == uuid::uuid!("ad68f29f-576f-4bb9-a36a-d47af965346c") {
			Some(CipherId::TwofishCbc)
		} else {
			None
		}
	}

	pub fn uuid(&self) -> Uuid {
		match self {
			CipherId::Aes256Cbc => uuid::uuid!("31c1f2e6-bf71-4350-be58-05216afc5aff"),
			CipherId::ChaCha20 => uuid::uuid!("d6038a2b-8b6f-4cb5-a524-339a31dbb59a"),
			CipherId::TwofishCbc => uuid::uuid!("ad68f29f-576f-4bb9-a36a-d47af965346c"),
		}
	}
}

pub fn encrypt_payload(
	cipher: CipherId,
	key: &[u8],
	iv: &[u8],
	plain: &[u8],
) -> Result<Zeroizing<Vec<u8>>, WvError> {
	if key.len() != 32 {
		return Err(WvError::InvalidFormat("symmetric key must be 32 bytes".into()));
	}
	let key: &[u8; 32] = key.try_into().unwrap();
	match cipher {
		CipherId::Aes256Cbc => {
			if iv.len() != 16 {
				return Err(WvError::InvalidFormat("AES-CBC IV must be 16 bytes".into()));
			}
			let enc = Aes256CbcEnc::new(key.into(), iv[..16].into());
			Ok(Zeroizing::new(
				enc.encrypt_padded_vec_mut::<Pkcs7>(plain),
			))
		}
		CipherId::TwofishCbc => {
			if iv.len() != 16 {
				return Err(WvError::InvalidFormat("Twofish-CBC IV must be 16 bytes".into()));
			}
			let enc = TwofishCbcEnc::new(key.into(), iv[..16].into());
			Ok(Zeroizing::new(
				enc.encrypt_padded_vec_mut::<Pkcs7>(plain),
			))
		}
		CipherId::ChaCha20 => {
			if iv.len() != 12 {
				return Err(WvError::InvalidFormat("ChaCha20 nonce must be 12 bytes".into()));
			}
			let mut out = Zeroizing::new(plain.to_vec());
			let mut cc = ChaCha20::new(key.into(), iv[..12].into());
			cc.apply_keystream(&mut *out);
			Ok(out)
		}
	}
}

pub fn decrypt_payload(
	cipher: CipherId,
	key: &[u8],
	iv: &[u8],
	data: &[u8],
) -> Result<Zeroizing<Vec<u8>>, WvError> {
	if key.len() != 32 {
		return Err(WvError::InvalidFormat("symmetric key must be 32 bytes".into()));
	}
	let key: &[u8; 32] = key.try_into().unwrap();
	match cipher {
		CipherId::Aes256Cbc => {
			if iv.len() != 16 {
				return Err(WvError::InvalidFormat("AES-CBC IV must be 16 bytes".into()));
			}
			let mut buf = data.to_vec();
			let dec = Aes256CbcDec::new(key.into(), iv[..16].into());
			let pt = dec
				.decrypt_padded_mut::<Pkcs7>(&mut buf)
				.map_err(|_| WvError::DecryptionFailed)?;
			Ok(Zeroizing::new(pt.to_vec()))
		}
		CipherId::TwofishCbc => {
			if iv.len() != 16 {
				return Err(WvError::InvalidFormat("Twofish-CBC IV must be 16 bytes".into()));
			}
			let mut buf = data.to_vec();
			let dec = TwofishCbcDec::new(key.into(), iv[..16].into());
			let pt = dec
				.decrypt_padded_mut::<Pkcs7>(&mut buf)
				.map_err(|_| WvError::DecryptionFailed)?;
			Ok(Zeroizing::new(pt.to_vec()))
		}
		CipherId::ChaCha20 => {
			if iv.len() != 12 {
				return Err(WvError::InvalidFormat("ChaCha20 nonce must be 12 bytes".into()));
			}
			let mut out = Zeroizing::new(data.to_vec());
			let mut cc = ChaCha20::new(key.into(), iv[..12].into());
			cc.apply_keystream(&mut *out);
			Ok(out)
		}
	}
}
