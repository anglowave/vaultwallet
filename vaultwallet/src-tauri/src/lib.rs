mod solana;

use rand::RngCore;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;
use vaultwallet_core::{
	open, save, Argon2Flavor, CipherId, CompositeKey, Database, Entry, Group, KdfParams,
	Metadata, VaultSettings, WvError,
};

#[derive(Serialize)]
struct MetadataOut {
	generator: String,
}

#[derive(Serialize)]
struct EntryOut {
	id: String,
	fields: HashMap<String, String>,
}

#[derive(Serialize)]
struct GroupOut {
	id: String,
	name: String,
	entries: Vec<EntryOut>,
	children: Vec<GroupOut>,
}

#[derive(Serialize)]
struct VaultTreeOut {
	metadata: MetadataOut,
	root: GroupOut,
}

fn wv_err(e: WvError) -> String {
	e.to_string()
}

fn composite_key(password: &str) -> Result<CompositeKey, String> {
	CompositeKey::new()
		.with_password(password)
		.build()
		.map_err(wv_err)
}

/// `kdf_strength` 0 = quick … 3 = slowest / hardest for attackers to brute-force.
fn new_database(kdf_strength: u32) -> Database {
	let level = kdf_strength.min(3);
	let mut salt = vec![0u8; 32];
	rand::thread_rng().fill_bytes(&mut salt);
	let (memory_bytes, iterations, parallelism) = match level {
		0 => (2_u64 * 1024 * 1024, 4_u32, 1_u32),
		1 => (8_u64 * 1024 * 1024, 7_u32, 2_u32),
		2 => (16_u64 * 1024 * 1024, 10_u32, 2_u32),
		_ => (32_u64 * 1024 * 1024, 14_u32, 2_u32),
	};
	let mut root = Group::new(Uuid::new_v4());
	root.name = "Root".into();
	Database {
		metadata: Metadata {
			generator: "VaultWallet".into(),
		},
		root,
		settings: VaultSettings {
			format_version_minor: 1,
			format_version_major: 4,
			cipher: CipherId::Aes256Cbc,
			compression_gzip: true,
			kdf: KdfParams::Argon2 {
				flavor: Argon2Flavor::Id,
				salt,
				iterations,
				memory_bytes,
				parallelism,
				version: 0x13,
			},
		},
	}
}

fn normalize_wlvlt_path(path: &str) -> Result<std::path::PathBuf, String> {
	let p = Path::new(path.trim());
	if p.as_os_str().is_empty() {
		return Err("path is empty".into());
	}
	let lossy = p.as_os_str().to_string_lossy().to_ascii_lowercase();
	let out = if lossy.ends_with(".wlvlt") {
		p.to_path_buf()
	} else {
		p.with_extension("wlvlt")
	};
	Ok(out)
}

fn parse_uuid(s: &str) -> Result<Uuid, String> {
	Uuid::parse_str(s.trim()).map_err(|e| e.to_string())
}

fn group_to_out(g: &Group) -> GroupOut {
	GroupOut {
		id: g.uuid.to_string(),
		name: g.name.clone(),
		entries: g
			.entries
			.iter()
			.map(|e| EntryOut {
				id: e.uuid.to_string(),
				fields: e.attrs.strings.clone(),
			})
			.collect(),
		children: g.children.iter().map(group_to_out).collect(),
	}
}

fn find_group_mut<'a>(root: &'a mut Group, id: &Uuid) -> Option<&'a mut Group> {
	if &root.uuid == id {
		return Some(root);
	}
	for c in &mut root.children {
		if let Some(g) = find_group_mut(c, id) {
			return Some(g);
		}
	}
	None
}

fn remove_group_recursive(root: &mut Group, id: &Uuid) -> bool {
	if let Some(i) = root.children.iter().position(|c| &c.uuid == id) {
		root.children.remove(i);
		return true;
	}
	for c in &mut root.children {
		if remove_group_recursive(c, id) {
			return true;
		}
	}
	false
}

#[tauri::command]
fn vault_create(path: String, password: String, kdf_strength: u32) -> Result<(), String> {
	let dest = normalize_wlvlt_path(&path)?;
	if dest.exists() {
		return Err(format!("file already exists: {}", dest.display()));
	}
	let key = composite_key(&password)?;
	let db = new_database(kdf_strength);
	save(&db, &dest, key).map_err(wv_err)
}

#[tauri::command]
fn vault_open(path: String, password: String) -> Result<VaultTreeOut, String> {
	let dest = normalize_wlvlt_path(&path)?;
	let key = composite_key(&password)?;
	let db = open(&dest, key).map_err(wv_err)?;
	Ok(VaultTreeOut {
		metadata: MetadataOut {
			generator: db.metadata.generator.clone(),
		},
		root: group_to_out(&db.root),
	})
}

#[tauri::command]
fn vault_add_group(
	path: String,
	password: String,
	parent_group_id: String,
	name: String,
) -> Result<GroupOut, String> {
	let dest = normalize_wlvlt_path(&path)?;
	let pid = parse_uuid(&parent_group_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	let parent = find_group_mut(&mut db.root, &pid).ok_or_else(|| "parent group not found".to_string())?;
	let mut g = Group::new(Uuid::new_v4());
	g.name = name;
	parent.children.push(g);
	let added = parent.children.last().unwrap();
	let out = group_to_out(added);
	save(&db, &dest, key).map_err(wv_err)?;
	Ok(out)
}

#[tauri::command]
fn vault_rename_group(
	path: String,
	password: String,
	group_id: String,
	name: String,
) -> Result<(), String> {
	let dest = normalize_wlvlt_path(&path)?;
	let gid = parse_uuid(&group_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	let g = find_group_mut(&mut db.root, &gid).ok_or_else(|| "group not found".to_string())?;
	g.name = name;
	save(&db, &dest, key).map_err(wv_err)
}

#[tauri::command]
fn vault_delete_group(path: String, password: String, group_id: String) -> Result<(), String> {
	let dest = normalize_wlvlt_path(&path)?;
	let gid = parse_uuid(&group_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	if db.root.uuid == gid {
		return Err("cannot delete root group".into());
	}
	if !remove_group_recursive(&mut db.root, &gid) {
		return Err("group not found".into());
	}
	save(&db, &dest, key).map_err(wv_err)
}

#[tauri::command]
fn vault_add_entry(
	path: String,
	password: String,
	group_id: String,
	fields: HashMap<String, String>,
) -> Result<EntryOut, String> {
	let dest = normalize_wlvlt_path(&path)?;
	let gid = parse_uuid(&group_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	let g = find_group_mut(&mut db.root, &gid).ok_or_else(|| "group not found".to_string())?;
	let mut e = Entry::new(Uuid::new_v4());
	e.attrs.strings = fields;
	g.entries.push(e);
	let added = g.entries.last().unwrap();
	let out = EntryOut {
		id: added.uuid.to_string(),
		fields: added.attrs.strings.clone(),
	};
	save(&db, &dest, key).map_err(wv_err)?;
	Ok(out)
}

#[tauri::command]
fn vault_update_entry(
	path: String,
	password: String,
	group_id: String,
	entry_id: String,
	fields: HashMap<String, String>,
) -> Result<(), String> {
	let dest = normalize_wlvlt_path(&path)?;
	let gid = parse_uuid(&group_id)?;
	let eid = parse_uuid(&entry_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	let g = find_group_mut(&mut db.root, &gid).ok_or_else(|| "group not found".to_string())?;
	let e = g
		.entries
		.iter_mut()
		.find(|x| x.uuid == eid)
		.ok_or_else(|| "entry not found".to_string())?;
	e.attrs.strings = fields;
	save(&db, &dest, key).map_err(wv_err)
}

#[tauri::command]
fn vault_delete_entry(
	path: String,
	password: String,
	group_id: String,
	entry_id: String,
) -> Result<(), String> {
	let dest = normalize_wlvlt_path(&path)?;
	let gid = parse_uuid(&group_id)?;
	let eid = parse_uuid(&entry_id)?;
	let key = composite_key(&password)?;
	let mut db = open(&dest, key.clone()).map_err(wv_err)?;
	let g = find_group_mut(&mut db.root, &gid).ok_or_else(|| "group not found".to_string())?;
	let i = g
		.entries
		.iter()
		.position(|x| x.uuid == eid)
		.ok_or_else(|| "entry not found".to_string())?;
	g.entries.remove(i);
	save(&db, &dest, key).map_err(wv_err)
}

#[tauri::command]
fn solana_public_key_from_private(private_key: String) -> Result<String, String> {
	solana::public_key_from_private(&private_key)
}

#[tauri::command]
async fn solana_fetch_balance(rpc_url: String, address: String) -> Result<String, String> {
	solana::fetch_balance_display(&rpc_url, &address).await
}

#[tauri::command]
async fn solana_trace_funding(rpc_url: String, wallet: String) -> Result<Option<String>, String> {
	solana::trace_funding(&rpc_url, &wallet).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_dialog::init())
		.invoke_handler(tauri::generate_handler![
			vault_create,
			vault_open,
			vault_add_group,
			vault_rename_group,
			vault_delete_group,
			vault_add_entry,
			vault_update_entry,
			vault_delete_entry,
			solana_public_key_from_private,
			solana_fetch_balance,
			solana_trace_funding,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
