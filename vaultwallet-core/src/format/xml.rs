use crate::crypto::cipher::CipherId;
use crate::crypto::hash::sha512;
use crate::crypto::kdf::Argon2Flavor;
use crate::db::database::Database;
use crate::db::entry::Entry;
use crate::db::group::Group;
use crate::db::metadata::Metadata;
use crate::error::WvError;
use crate::format::header::KdfParams;
use crate::settings::VaultSettings;
use base64::Engine;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20::ChaCha20;
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::io::Cursor;
use uuid::Uuid;

/// Decrypt `Protected="True"` string values in document order (KDBX4 ChaCha20 stream).
pub fn decode_protected_xml(xml: &str, stream_key: &[u8]) -> Result<String, WvError> {
	let d = sha512(stream_key);
	let key: [u8; 32] = d[..32].try_into().unwrap();
	let nonce: [u8; 12] = d[32..44].try_into().unwrap();
	let mut cipher = ChaCha20::new(&key.into(), &nonce.into());
	let mut stream_pos: u64 = 0;

	let mut reader = Reader::from_str(xml);
	reader.trim_text(false);
	let mut writer = Writer::new(Cursor::new(Vec::new()));
	let mut buf = Vec::new();
	let mut in_protected_value = false;

	loop {
		buf.clear();
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				let e = e.into_owned();
				if e.local_name().as_ref() == b"Value" && value_has_protected_true(&e) {
					in_protected_value = true;
				}
				writer
					.write_event(Event::Start(e))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::Empty(e)) => {
				let e = e.into_owned();
				writer
					.write_event(Event::Empty(e))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::End(e)) => {
				let e = e.into_owned();
				if in_protected_value && e.local_name().as_ref() == b"Value" {
					in_protected_value = false;
				}
				writer
					.write_event(Event::End(e))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::Text(t)) => {
				if in_protected_value {
					let s = t
						.unescape()
						.map_err(|e| WvError::InvalidFormat(format!("xml decode: {e}")))?;
					let mut ct = base64::engine::general_purpose::STANDARD
						.decode(s.trim().as_bytes())
						.map_err(|_| WvError::InvalidFormat("protected base64".into()))?;
					let n = ct.len() as u64;
					cipher.seek(stream_pos);
					cipher.apply_keystream(&mut ct);
					stream_pos += n;
					let plain = String::from_utf8(ct)
						.map_err(|_| WvError::InvalidFormat("protected utf-8".into()))?;
					let bt = BytesText::new(&plain);
					writer
						.write_event(Event::Text(bt))
						.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
				} else {
					writer
						.write_event(Event::Text(t.into_owned()))
						.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
				}
			}
			Ok(Event::CData(t)) => {
				writer
					.write_event(Event::CData(t.into_owned()))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::Decl(d)) => {
				writer
					.write_event(Event::Decl(d.into_owned()))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::Comment(c)) => {
				writer
					.write_event(Event::Comment(c.into_owned()))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::PI(p)) => {
				writer
					.write_event(Event::PI(p.into_owned()))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::DocType(d)) => {
				writer
					.write_event(Event::DocType(d.into_owned()))
					.map_err(|e| WvError::InvalidFormat(format!("xml write: {e}")))?;
			}
			Ok(Event::Eof) => break,
			Err(e) => return Err(WvError::InvalidFormat(format!("xml read: {e}"))),
		}
	}
	let bytes = writer.into_inner().into_inner();
	String::from_utf8(bytes).map_err(|_| WvError::InvalidFormat("xml output utf-8".into()))
}

fn value_has_protected_true(e: &BytesStart<'_>) -> bool {
	for a in e.attributes().flatten() {
		let k = std::str::from_utf8(a.key.as_ref()).unwrap_or("");
		let is_prot = k == "Protected" || k.ends_with(":Protected");
		if !is_prot {
			continue;
		}
		let v = std::str::from_utf8(&a.value).unwrap_or("");
		if v.eq_ignore_ascii_case("true") || v == "1" {
			return true;
		}
	}
	false
}

/// Serialize a minimal database tree to KeePass-compatible XML and encrypt protected fields.
pub fn serialize_database(db: &Database, stream_key: &[u8]) -> Result<String, WvError> {
	if stream_key.len() != 64 {
		return Err(WvError::InvalidFormat("inner stream key must be 64 bytes".into()));
	}
	let d = sha512(stream_key);
	let key: [u8; 32] = d[..32].try_into().unwrap();
	let nonce: [u8; 12] = d[32..44].try_into().unwrap();
	let mut cipher = ChaCha20::new(&key.into(), &nonce.into());
	let mut stream_pos: u64 = 0;

	let mut out = String::new();
	out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
	out.push_str("<KeePassFile><Meta><Generator>");
	xml_escape_str(&mut out, &db.metadata.generator);
	out.push_str("</Generator></Meta><Root>");
	write_group(&mut out, &db.root, &mut cipher, &mut stream_pos)?;
	out.push_str("</Root></KeePassFile>");
	Ok(out)
}

fn write_group(
	out: &mut String,
	g: &Group,
	cipher: &mut ChaCha20,
	pos: &mut u64,
) -> Result<(), WvError> {
	out.push_str("<Group><UUID>");
	out.push_str(&base64::engine::general_purpose::STANDARD.encode(g.uuid.as_bytes()));
	out.push_str("</UUID><Name>");
	xml_escape_str(out, &g.name);
	out.push_str("</Name><Notes/>");
	for e in &g.entries {
		write_entry(out, e, cipher, pos)?;
	}
	for c in &g.children {
		write_group(out, c, cipher, pos)?;
	}
	out.push_str("</Group>");
	Ok(())
}

fn write_entry(
	out: &mut String,
	e: &Entry,
	cipher: &mut ChaCha20,
	pos: &mut u64,
) -> Result<(), WvError> {
	out.push_str("<Entry><UUID>");
	out.push_str(&base64::engine::general_purpose::STANDARD.encode(e.uuid.as_bytes()));
	out.push_str("</UUID><IconID>0</IconID>");
	let keys: Vec<_> = e.attrs.strings.keys().cloned().collect();
	for k in keys {
		let v = e.attrs.strings.get(&k).unwrap();
		let prot = k == "Password";
		out.push_str("<String><Key>");
		xml_escape_str(out, &k);
		out.push_str("</Key><Value");
		if prot {
			out.push_str(r#" Protected="True""#);
		}
		out.push('>');
		if prot {
			let mut buf = v.as_bytes().to_vec();
			let n = buf.len() as u64;
			cipher.seek(*pos);
			cipher.apply_keystream(&mut buf);
			*pos += n;
			let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
			xml_escape_str(out, &b64);
		} else {
			xml_escape_str(out, v);
		}
		out.push_str("</Value></String>");
	}
	out.push_str("</Entry>");
	Ok(())
}

fn parse_uuid_b64(s: &str) -> Result<Uuid, WvError> {
	let bytes = base64::engine::general_purpose::STANDARD
		.decode(s.trim().as_bytes())
		.map_err(|_| WvError::InvalidFormat("UUID base64".into()))?;
	if bytes.len() != 16 {
		return Err(WvError::InvalidFormat("UUID length".into()));
	}
	let mut arr = [0u8; 16];
	arr.copy_from_slice(&bytes);
	Ok(Uuid::from_bytes(arr))
}

fn xml_escape_str(out: &mut String, s: &str) {
	for ch in s.chars() {
		match ch {
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			'&' => out.push_str("&amp;"),
			'"' => out.push_str("&quot;"),
			_ => out.push(ch),
		}
	}
}

pub fn parse_database_xml(xml: &str) -> Result<Database, WvError> {
	let mut reader = Reader::from_str(xml);
	reader.trim_text(true);
	let mut buf = Vec::new();
	let mut meta = Metadata {
		generator: String::new(),
	};
	let mut group_stack: Vec<Group> = Vec::new();
	let mut root_out: Option<Group> = None;
	let mut current_entry: Option<Entry> = None;
	let mut string_key: Option<String> = None;
	let mut in_meta = false;
	let mut in_generator = false;
	let mut in_root = false;
	let mut in_entry = false;
	let mut in_string = false;
	let mut in_key = false;
	let mut in_value = false;
	let mut in_group_name = false;
	let mut in_group_uuid = false;
	let mut in_entry_uuid = false;

	loop {
		buf.clear();
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				if e.local_name().as_ref() == b"Meta" {
					in_meta = true;
				} else if e.local_name().as_ref() == b"Generator" && in_meta {
					in_generator = true;
				} else if e.local_name().as_ref() == b"Root" {
					in_root = true;
				} else if e.local_name().as_ref() == b"Group" && in_root {
					group_stack.push(Group::new(Uuid::nil()));
				} else if e.local_name().as_ref() == b"Entry" && !group_stack.is_empty() {
					in_entry = true;
					current_entry = Some(Entry::new(Uuid::nil()));
				} else if e.local_name().as_ref() == b"String" && in_entry {
					in_string = true;
				} else if e.local_name().as_ref() == b"Key" && in_string {
					in_key = true;
					string_key = Some(String::new());
				} else if e.local_name().as_ref() == b"Value" && in_string {
					in_value = true;
				} else if e.local_name().as_ref() == b"Name"
					&& !in_meta
					&& !in_entry
					&& !group_stack.is_empty()
				{
					in_group_name = true;
				} else if e.local_name().as_ref() == b"UUID"
					&& !group_stack.is_empty()
					&& in_entry
				{
					in_entry_uuid = true;
				} else if e.local_name().as_ref() == b"UUID"
					&& !group_stack.is_empty()
					&& !in_entry
				{
					in_group_uuid = true;
				}
			}
			Ok(Event::Text(t)) => {
				let txt = t
					.unescape()
					.map_err(|e| WvError::InvalidFormat(format!("xml text: {e}")))?;
				if in_generator {
					meta.generator = txt.to_string();
				} else if in_key {
					if let Some(ref mut k) = string_key {
						k.push_str(&txt);
					}
				} else if in_value {
					if let (Some(ref k), Some(ref mut ent)) = (&string_key, &mut current_entry) {
						ent.attrs.strings.insert(k.clone(), txt.to_string());
					}
				} else if in_group_name {
					if let Some(g) = group_stack.last_mut() {
						g.name = txt.to_string();
					}
				} else if in_group_uuid {
					if let Some(g) = group_stack.last_mut() {
						if let Ok(u) = parse_uuid_b64(&txt) {
							g.uuid = u;
						}
					}
				} else if in_entry_uuid {
					if let Some(ref mut ent) = current_entry {
						if let Ok(u) = parse_uuid_b64(&txt) {
							ent.uuid = u;
						}
					}
				}
			}
			Ok(Event::End(e)) => {
				if e.local_name().as_ref() == b"Meta" {
					in_meta = false;
				} else if e.local_name().as_ref() == b"Generator" {
					in_generator = false;
				} else if e.local_name().as_ref() == b"Root" {
					in_root = false;
				} else if e.local_name().as_ref() == b"Group" {
					let finished = match group_stack.pop() {
						Some(g) => g,
						None => continue,
					};
					if let Some(parent) = group_stack.last_mut() {
						parent.children.push(finished);
					} else {
						root_out = Some(finished);
					}
				} else if e.local_name().as_ref() == b"Entry" {
					if let (Some(ent), Some(g)) = (current_entry.take(), group_stack.last_mut()) {
						g.entries.push(ent);
					}
					in_entry = false;
				} else if e.local_name().as_ref() == b"String" {
					in_string = false;
					string_key = None;
				} else if e.local_name().as_ref() == b"Key" {
					in_key = false;
				} else if e.local_name().as_ref() == b"Value" {
					in_value = false;
				} else if e.local_name().as_ref() == b"Name" {
					in_group_name = false;
				} else if e.local_name().as_ref() == b"UUID" && in_entry {
					in_entry_uuid = false;
				} else if e.local_name().as_ref() == b"UUID" && !in_entry {
					in_group_uuid = false;
				}
			}
			Ok(Event::Eof) => break,
			Err(e) => return Err(WvError::InvalidFormat(format!("xml: {e}"))),
			_ => {}
		}
	}

	let root = root_out.unwrap_or_else(|| Group::new(Uuid::nil()));
	if meta.generator.is_empty() {
		meta.generator = "vaultwallet-core".into();
	}
	let placeholder_settings = VaultSettings {
		format_version_minor: 1,
		format_version_major: 4,
		cipher: CipherId::Aes256Cbc,
		compression_gzip: false,
		kdf: KdfParams::Argon2 {
			flavor: Argon2Flavor::D,
			salt: vec![0u8; 16],
			iterations: 2,
			memory_bytes: 8192,
			parallelism: 1,
			version: 0x13,
		},
	};
	Ok(Database {
		metadata: meta,
		root,
		settings: placeholder_settings,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn protected_stream_two_passwords_in_order() {
		let sk = [3u8; 64];
		let d = sha512(&sk);
		let key: [u8; 32] = d[..32].try_into().unwrap();
		let nonce: [u8; 12] = d[32..44].try_into().unwrap();
		let mut enc = ChaCha20::new(&key.into(), &nonce.into());
		let mut pos = 0u64;
		let mut enc_first = b"first".to_vec();
		enc.seek(pos);
		enc.apply_keystream(&mut enc_first);
		pos += enc_first.len() as u64;
		let mut enc_second = b"second".to_vec();
		enc.seek(pos);
		enc.apply_keystream(&mut enc_second);
		let b64a = base64::engine::general_purpose::STANDARD.encode(&enc_first);
		let b64b = base64::engine::general_purpose::STANDARD.encode(&enc_second);
		let xml = format!(
			r#"<?xml version="1.0"?><KeePassFile><Root><Group><Entry><String><Key>A</Key><Value Protected="True">{}</Value></String><String><Key>B</Key><Value Protected="True">{}</Value></String></Entry></Group></Root></KeePassFile>"#,
			b64a, b64b
		);
		let dec = decode_protected_xml(&xml, &sk).unwrap();
		assert!(dec.contains(">first<") && dec.contains(">second<"));
	}
}
