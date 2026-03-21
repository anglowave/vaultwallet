use crate::error::WvError;

pub const INNER_END: u8 = 0;
pub const INNER_STREAM_CIPHER: u8 = 1;
pub const INNER_STREAM_KEY: u8 = 2;
pub const INNER_BINARY: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum StreamCipherId {
	ArcFourVariant = 1,
	Salsa20 = 2,
	ChaCha20 = 3,
}

impl StreamCipherId {
	pub fn from_u32(v: u32) -> Option<Self> {
		match v {
			1 => Some(StreamCipherId::ArcFourVariant),
			2 => Some(StreamCipherId::Salsa20),
			3 => Some(StreamCipherId::ChaCha20),
			_ => None,
		}
	}
}

#[derive(Clone, Debug)]
pub struct InnerBinary {
	pub protected: bool,
	pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct InnerHeader {
	pub stream_cipher: Option<StreamCipherId>,
	pub stream_key: Option<Vec<u8>>,
	pub binaries: Vec<InnerBinary>,
}

impl InnerHeader {
	pub fn parse(data: &[u8]) -> Result<(Self, usize), WvError> {
		let mut pos = 0usize;
		let mut stream_cipher = None;
		let mut stream_key = None;
		let mut binaries = Vec::new();
		loop {
			if pos >= data.len() {
				return Err(WvError::InvalidFormat("unterminated inner header".into()));
			}
			let ty = data[pos];
			pos += 1;
			let size = read_u32(data, &mut pos)? as usize;
			let payload = read_bytes(data, &mut pos, size)?;
			match ty {
				INNER_END => break,
				INNER_STREAM_CIPHER => {
					if payload.len() != 4 {
						return Err(WvError::InvalidFormat("inner StreamCipher size".into()));
					}
					let v = u32::from_le_bytes(payload[..4].try_into().unwrap());
					stream_cipher = Some(
						StreamCipherId::from_u32(v)
							.ok_or_else(|| WvError::InvalidFormat("unknown stream cipher".into()))?,
					);
				}
				INNER_STREAM_KEY => {
					stream_key = Some(payload);
				}
				INNER_BINARY => {
					if payload.is_empty() {
						return Err(WvError::InvalidFormat("empty inner Binary".into()));
					}
					let flags = payload[0];
					binaries.push(InnerBinary {
						protected: (flags & 0x01) != 0,
						data: payload[1..].to_vec(),
					});
				}
				_ => { /* skip unknown */ }
			}
		}
		Ok((
			InnerHeader {
				stream_cipher,
				stream_key,
				binaries,
			},
			pos,
		))
	}

	pub fn encode(&self) -> Vec<u8> {
		let mut out = Vec::new();
		if let Some(sc) = self.stream_cipher {
			push_field(
				&mut out,
				INNER_STREAM_CIPHER,
				&(sc as u32).to_le_bytes(),
			);
		}
		if let Some(ref sk) = self.stream_key {
			push_field(&mut out, INNER_STREAM_KEY, sk);
		}
		for b in &self.binaries {
			let mut buf = Vec::with_capacity(1 + b.data.len());
			buf.push(if b.protected { 0x01 } else { 0x00 });
			buf.extend_from_slice(&b.data);
			push_field(&mut out, INNER_BINARY, &buf);
		}
		push_field(&mut out, INNER_END, &[]);
		out
	}
}

fn read_u32(buf: &[u8], i: &mut usize) -> Result<u32, WvError> {
	if *i + 4 > buf.len() {
		return Err(WvError::InvalidFormat("inner header truncated u32".into()));
	}
	let v = u32::from_le_bytes(buf[*i..*i + 4].try_into().unwrap());
	*i += 4;
	Ok(v)
}

fn read_bytes(buf: &[u8], i: &mut usize, len: usize) -> Result<Vec<u8>, WvError> {
	if *i + len > buf.len() {
		return Err(WvError::InvalidFormat("inner header truncated payload".into()));
	}
	let v = buf[*i..*i + len].to_vec();
	*i += len;
	Ok(v)
}

fn push_field(out: &mut Vec<u8>, ty: u8, data: &[u8]) {
	out.push(ty);
	out.extend_from_slice(&(data.len() as u32).to_le_bytes());
	out.extend_from_slice(data);
}
