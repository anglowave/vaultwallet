use tempfile::tempdir;
use uuid::Uuid;
use vaultwallet_core::{
	open, save, Argon2Flavor, CipherId, CompositeKey, Database, Entry, Group, KdfParams, Metadata,
	VaultSettings,
};

fn sample_db() -> Database {
	let mut root = Group::new(Uuid::new_v4());
	root.name = "Root".into();
	let mut e = Entry::new(Uuid::new_v4());
	e.attrs.strings.insert("Title".into(), "Hello".into());
	e.attrs.strings.insert("UserName".into(), "alice".into());
	e.attrs.strings.insert("Password".into(), "secret-pass".into());
	root.entries.push(e);
	Database {
		metadata: Metadata {
			generator: "vaultwallet-core".into(),
		},
		root,
		settings: VaultSettings {
			format_version_minor: 1,
			format_version_major: 4,
			cipher: CipherId::Aes256Cbc,
			compression_gzip: true,
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
fn round_trip_argon2d_aes_gzip() {
	let dir = tempdir().unwrap();
	let path = dir.path().join("t.wlvlt");
	let db = sample_db();
	let key = CompositeKey::new().with_password("pw").build().unwrap();
	save(&db, &path, key).unwrap();
	let key2 = CompositeKey::new().with_password("pw").build().unwrap();
	let db2 = open(&path, key2).unwrap();
	assert_eq!(db2.root.name, db.root.name);
	assert_eq!(db2.root.entries.len(), 1);
	let e = &db2.root.entries[0];
	assert_eq!(e.title(), Some("Hello"));
	assert_eq!(e.username(), Some("alice"));
	assert_eq!(e.password(), Some("secret-pass"));
}

#[test]
fn round_trip_argon2id_chacha() {
	let dir = tempdir().unwrap();
	let path = dir.path().join("c.wlvlt");
	let mut db = sample_db();
	db.settings.cipher = CipherId::ChaCha20;
	db.settings.kdf = KdfParams::Argon2 {
		flavor: Argon2Flavor::Id,
		salt: vec![1u8; 32],
		iterations: 2,
		memory_bytes: 8192,
		parallelism: 1,
		version: 0x13,
	};
	let key = CompositeKey::new().with_password("x").build().unwrap();
	save(&db, &path, key).unwrap();
	let key2 = CompositeKey::new().with_password("x").build().unwrap();
	let db2 = open(&path, key2).unwrap();
	assert_eq!(db2.root.entries[0].password(), Some("secret-pass"));
	assert_eq!(db2.settings.cipher, CipherId::ChaCha20);
}

#[test]
fn save_appends_wlvlt_extension() {
	let dir = tempdir().unwrap();
	let path = dir.path().join("noext");
	let db = sample_db();
	let key = CompositeKey::new().with_password("p").build().unwrap();
	save(&db, &path, key).unwrap();
	let w = path.with_extension("wlvlt");
	assert!(w.exists());
}
