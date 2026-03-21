use crate::crypto::hash::{block_hmac_key, ct_eq_32, hmac_sha256};
use crate::error::WvError;
use zeroize::Zeroizing;

fn read_u64(buf: &[u8], i: &mut usize) -> Result<u64, WvError> {
	if *i + 8 > buf.len() {
		return Err(WvError::InvalidFormat("truncated u64".into()));
	}
	let v = u64::from_le_bytes(buf[*i..*i + 8].try_into().unwrap());
	*i += 8;
	Ok(v)
}

fn read_u32(buf: &[u8], i: &mut usize) -> Result<u32, WvError> {
	if *i + 4 > buf.len() {
		return Err(WvError::InvalidFormat("truncated u32".into()));
	}
	let v = u32::from_le_bytes(buf[*i..*i + 4].try_into().unwrap());
	*i += 4;
	Ok(v)
}

/// Read HMAC-block stream into a single payload (KDBX4).
pub fn read_hmac_block_stream(data: &[u8], hmac_root: &[u8]) -> Result<Zeroizing<Vec<u8>>, WvError> {
	if hmac_root.len() != 64 {
		return Err(WvError::InvalidFormat("HMAC root length".into()));
	}
	let mut i = 0usize;
	let mut out = Zeroizing::new(Vec::new());
	let mut index = 0u64;
	loop {
		let block_index = read_u64(data, &mut i)?;
		if block_index != index {
			return Err(WvError::InvalidFormat("HMAC block index mismatch".into()));
		}
		if i + 32 + 4 > data.len() {
			return Err(WvError::InvalidFormat("truncated HMAC block".into()));
		}
		let stored_hmac: [u8; 32] = data[i..i + 32].try_into().unwrap();
		i += 32;
		let size = read_u32(data, &mut i)? as usize;
		if size == 0 {
			let block_key = block_hmac_key(hmac_root, block_index)?;
			let mut msg = Vec::with_capacity(8 + 4);
			msg.extend_from_slice(&block_index.to_le_bytes());
			msg.extend_from_slice(&0u32.to_le_bytes());
			let calc = hmac_sha256(block_key.as_ref(), &msg);
			if !ct_eq_32(&calc, &stored_hmac) {
				return Err(WvError::HmacMismatch);
			}
			if i != data.len() {
				return Err(WvError::InvalidFormat("trailing data after HMAC stream".into()));
			}
			break;
		}
		if i + size > data.len() {
			return Err(WvError::InvalidFormat("HMAC block data truncated".into()));
		}
		let chunk = &data[i..i + size];
		i += size;
		let block_key = block_hmac_key(hmac_root, block_index)?;
		let mut msg = Vec::with_capacity(8 + 4 + chunk.len());
		msg.extend_from_slice(&block_index.to_le_bytes());
		msg.extend_from_slice(&(size as u32).to_le_bytes());
		msg.extend_from_slice(chunk);
		let calc = hmac_sha256(block_key.as_ref(), &msg);
		if !ct_eq_32(&calc, &stored_hmac) {
			return Err(WvError::HmacMismatch);
		}
		out.extend_from_slice(chunk);
		index = index
			.checked_add(1)
			.ok_or_else(|| WvError::InvalidFormat("HMAC block index overflow".into()))?;
	}
	Ok(out)
}

/// Write payload as HMAC-block stream (1 MiB blocks).
pub fn write_hmac_block_stream(payload: &[u8], hmac_root: &[u8]) -> Result<Vec<u8>, WvError> {
	if hmac_root.len() != 64 {
		return Err(WvError::InvalidFormat("HMAC root length".into()));
	}
	const BLOCK: usize = 1024 * 1024;
	let mut out = Vec::new();
	let mut index = 0u64;
	let mut offset = 0usize;
	while offset < payload.len() {
		let end = (offset + BLOCK).min(payload.len());
		let chunk = &payload[offset..end];
		let size = chunk.len() as u32;
		let block_key = block_hmac_key(hmac_root, index)?;
		let mut msg = Vec::with_capacity(8 + 4 + chunk.len());
		msg.extend_from_slice(&index.to_le_bytes());
		msg.extend_from_slice(&size.to_le_bytes());
		msg.extend_from_slice(chunk);
		let mac = hmac_sha256(block_key.as_ref(), &msg);
		out.extend_from_slice(&index.to_le_bytes());
		out.extend_from_slice(&mac);
		out.extend_from_slice(&size.to_le_bytes());
		out.extend_from_slice(chunk);
		offset = end;
		index += 1;
	}
	let block_key = block_hmac_key(hmac_root, index)?;
	let mut msg = Vec::with_capacity(8 + 4);
	msg.extend_from_slice(&index.to_le_bytes());
	msg.extend_from_slice(&0u32.to_le_bytes());
	let mac = hmac_sha256(block_key.as_ref(), &msg);
	out.extend_from_slice(&index.to_le_bytes());
	out.extend_from_slice(&mac);
	out.extend_from_slice(&0u32.to_le_bytes());
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::crypto::random::random_bytes;

	#[test]
	fn hmac_stream_round_trip() {
		let mut root = [0u8; 64];
		random_bytes(&mut root);
		let payload = b"hello vaultwallet hmac blocks";
		let enc = write_hmac_block_stream(payload, &root).unwrap();
		let dec = read_hmac_block_stream(&enc, &root).unwrap();
		assert_eq!(dec.as_slice(), payload);
	}
}
