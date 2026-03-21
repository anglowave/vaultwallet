use std::fs;
use tempfile::tempdir;
use uuid::Uuid;
use vaultwallet_core::format::header::OuterHeader;
use vaultwallet_core::{
	open, save, Argon2Flavor, CipherId, CompositeKey, Database, Group, KdfParams, Metadata,
	VaultSettings,
};

fn tiny_db() -> Database {
	let mut root = Group::new(Uuid::nil());
	root.name = "R".into();
	Database {
		metadata: Metadata {
			generator: "t".into(),
		},
		root,
		settings: VaultSettings {
			format_version_minor: 1,
			format_version_major: 4,
			cipher: CipherId::Aes256Cbc,
			compression_gzip: false,
			kdf: KdfParams::Argon2 {
				flavor: Argon2Flavor::D,
				salt: vec![0u8; 32],
				iterations: 2,
				memory_bytes: 8192,
				parallelism: 1,
				version: 0x13,
			},
		},
	}
}

#[test]
fn flipped_header_hmac_byte_rejected() {
	let dir = tempdir().unwrap();
	let path = dir.path().join("t.wlvlt");
	let db = tiny_db();
	let key = CompositeKey::new().with_password("p").build().unwrap();
	save(&db, &path, key).unwrap();

	let mut raw = fs::read(&path).unwrap();
	let (_outer, hdr_end) = OuterHeader::parse(&raw).unwrap();
	// layout: header | sha256(header) | hmac(header) | blocks
	let hmac_off = hdr_end + 32;
	raw[hmac_off] ^= 0x80;
	fs::write(&path, &raw).unwrap();

	let key2 = CompositeKey::new().with_password("p").build().unwrap();
	let r = open(&path, key2);
	assert!(matches!(r, Err(vaultwallet_core::WvError::HmacMismatch)));
}
