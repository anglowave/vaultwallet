use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
const MIN_INCOMING_SOL: f64 = 0.01;
const MAX_SIGNATURE_PAGES: usize = 80;
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

static EXCHANGE_MAP: OnceLock<Result<HashMap<String, (String, String)>, String>> = OnceLock::new();

#[derive(Deserialize)]
struct ExchangeFile {
	exchanges: Vec<ExchangeEntry>,
}

#[derive(Deserialize)]
struct ExchangeEntry {
	name: String,
	icon: String,
	addresses: Vec<String>,
}

fn load_exchange_map() -> Result<HashMap<String, (String, String)>, String> {
	const JSON: &str =
		include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../config/exchange-wallets.json"));
	let file: ExchangeFile =
		serde_json::from_str(JSON).map_err(|e| format!("exchange-wallets.json: {e}"))?;
	let mut map = HashMap::new();
	for ex in file.exchanges {
		for addr in ex.addresses {
			map.insert(addr, (ex.name.clone(), ex.icon.clone()));
		}
	}
	Ok(map)
}

fn exchange_directory() -> Result<&'static HashMap<String, (String, String)>, String> {
	match EXCHANGE_MAP.get_or_init(|| load_exchange_map()) {
		Ok(m) => Ok(m),
		Err(e) => Err(e.clone()),
	}
}

fn parse_secret_bytes(raw: &str) -> Result<Vec<u8>, String> {
	let s = raw.trim();
	if s.is_empty() {
		return Err("empty private key".into());
	}
	if s.starts_with('[') {
		let arr: Vec<Value> = serde_json::from_str(s).map_err(|e| e.to_string())?;
		if arr.len() != 64 {
			return Err("JSON key must contain 64 numbers".into());
		}
		let mut out = Vec::with_capacity(64);
		for v in arr {
			let n = v
				.as_u64()
				.ok_or_else(|| "JSON key must be byte values 0–255".to_string())?;
			if n > 255 {
				return Err("JSON key byte out of range".into());
			}
			out.push(n as u8);
		}
		Ok(out)
	} else {
		bs58::decode(s)
			.into_vec()
			.map_err(|e| format!("invalid base58: {e}"))
	}
}

fn signing_key_from_input(raw: &str) -> Result<SigningKey, String> {
	let bytes = parse_secret_bytes(raw)?;
	match bytes.len() {
		64 => {
			let a: [u8; 64] = bytes
				.try_into()
				.map_err(|_| "expected 64-byte secret key".to_string())?;
			SigningKey::from_keypair_bytes(&a).map_err(|e| e.to_string())
		}
		32 => {
			let a: [u8; 32] = bytes
				.try_into()
				.map_err(|_| "expected 32-byte seed".to_string())?;
			Ok(SigningKey::from_bytes(&a))
		}
		_ => Err(format!(
			"unsupported key length {} (use 32-byte seed or 64-byte secret)",
			bytes.len()
		)),
	}
}

pub fn public_key_from_private(raw: &str) -> Result<String, String> {
	let sk = signing_key_from_input(raw)?;
	let vk = sk.verifying_key();
	Ok(bs58::encode(vk.as_bytes()).into_string())
}

fn format_balance_lamports(lamports: u64) -> String {
	let whole = lamports / 1_000_000_000;
	let frac = lamports % 1_000_000_000;
	let hundredths = frac / 10_000_000;
	format!("{whole}.{hundredths:02} sol")
}

async fn rpc_request(client: &reqwest::Client, url: &str, method: &str, params: Value) -> Result<Value, String> {
	let body = json!({
		"jsonrpc": "2.0",
		"id": 1,
		"method": method,
		"params": params,
	});
	let res = client
		.post(url)
		.json(&body)
		.send()
		.await
		.map_err(|e| e.to_string())?;
	let json: Value = res.json().await.map_err(|e| e.to_string())?;
	if let Some(err) = json.get("error") {
		return Err(err.to_string());
	}
	json
		.get("result")
		.cloned()
		.ok_or_else(|| "RPC response missing result".to_string())
}

fn json_u64_lamports(v: &Value) -> Option<u64> {
	if let Some(n) = v.as_u64() {
		return Some(n);
	}
	if let Some(n) = v.as_i64() {
		return u64::try_from(n).ok();
	}
	v.as_object()
		.and_then(|o| o.get("value"))
		.and_then(|x| x.as_u64().or_else(|| x.as_i64().and_then(|i| u64::try_from(i).ok())))
}

pub async fn fetch_balance_display(rpc_url: &str, address: &str) -> Result<String, String> {
	let url = rpc_url.trim();
	if url.is_empty() {
		return Err("RPC URL is empty".into());
	}
	let client = reqwest::Client::new();
	let params = json!([address, { "commitment": "confirmed" }]);
	let v = rpc_request(&client, url, "getBalance", params).await?;
	let lamports = json_u64_lamports(&v).ok_or_else(|| {
		format!("unexpected getBalance result (expected integer lamports): {v}")
	})?;
	Ok(format_balance_lamports(lamports))
}

fn account_pubkey_at(keys: &[Value], index: usize) -> Option<String> {
	let acc = keys.get(index)?;
	if let Some(s) = acc.as_str() {
		return Some(s.to_string());
	}
	acc.get("pubkey")
		.and_then(|p| p.as_str())
		.map(std::string::ToString::to_string)
}

fn system_transfer_source(instructions: &[Value], wallet: &str) -> Option<String> {
	for inst in instructions {
		let program_ok = inst.get("program").and_then(|p| p.as_str()) == Some("system")
			|| inst.get("programId").and_then(|p| p.as_str()) == Some(SYSTEM_PROGRAM);
		if !program_ok {
			continue;
		}
		let parsed = inst.get("parsed")?;
		if parsed.get("type").and_then(|t| t.as_str()) != Some("transfer") {
			continue;
		}
		let info = parsed.get("info")?;
		if info.get("destination").and_then(|d| d.as_str()) != Some(wallet) {
			continue;
		}
		let source = info.get("source").and_then(|s| s.as_str())?;
		return Some(source.to_string());
	}
	None
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FundingPayload {
	exchange: Option<String>,
	icon: Option<String>,
	amount_sol: f64,
	signature: String,
	date_utc: Option<String>,
	sender: String,
}

pub async fn trace_funding(rpc_url: &str, wallet: &str) -> Result<Option<String>, String> {
	let url = rpc_url.trim();
	if url.is_empty() {
		return Err("RPC URL is empty".into());
	}
	let map = exchange_directory()?;
	let client = reqwest::Client::new();
	let mut all_sigs: Vec<String> = Vec::new();
	let mut before: Option<String> = None;

	for _page in 0..MAX_SIGNATURE_PAGES {
		let mut cfg = json!({ "limit": 1000 });
		if let Some(ref b) = before {
			cfg["before"] = json!(b);
		}
		let result = rpc_request(
			&client,
			url,
			"getSignaturesForAddress",
			json!([wallet, cfg]),
		)
		.await?;
		let arr = result
			.as_array()
			.ok_or_else(|| "getSignaturesForAddress: expected array".to_string())?;
		if arr.is_empty() {
			break;
		}
		let last_sig = arr
			.last()
			.and_then(|o| o.get("signature"))
			.and_then(|s| s.as_str())
			.ok_or_else(|| "signature entry missing".to_string())?;
		before = Some(last_sig.to_string());
		for row in arr {
			if let Some(sig) = row.get("signature").and_then(|s| s.as_str()) {
				all_sigs.push(sig.to_string());
			}
		}
		tokio::time::sleep(Duration::from_millis(200)).await;
	}

	all_sigs.reverse();

	for signature in all_sigs {
		let tx = rpc_request(
			&client,
			url,
			"getTransaction",
			json!([
				&signature,
				{
					"encoding": "jsonParsed",
					"maxSupportedTransactionVersion": 0
				}
			]),
		)
		.await
		.ok();

		let Some(tx) = tx else {
			tokio::time::sleep(Duration::from_millis(50)).await;
			continue;
		};
		if tx.is_null() {
			tokio::time::sleep(Duration::from_millis(50)).await;
			continue;
		}

		let meta = match tx.get("meta").filter(|m| !m.is_null()) {
			Some(m) => m,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let pre = match meta.get("preBalances").and_then(|x| x.as_array()) {
			Some(a) => a,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let post = match meta.get("postBalances").and_then(|x| x.as_array()) {
			Some(a) => a,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let transaction = match tx.get("transaction").and_then(|t| t.as_object()) {
			Some(t) => t,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let message = match transaction.get("message").and_then(|m| m.as_object()) {
			Some(m) => m,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let accounts = match message.get("accountKeys").and_then(|a| a.as_array()) {
			Some(a) => a,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};
		let instructions = match message.get("instructions").and_then(|i| i.as_array()) {
			Some(i) => i,
			None => {
				tokio::time::sleep(Duration::from_millis(50)).await;
				continue;
			}
		};

		if accounts.len() != pre.len() || pre.len() != post.len() {
			tokio::time::sleep(Duration::from_millis(50)).await;
			continue;
		}

		for i in 0..accounts.len() {
			let pk = match account_pubkey_at(accounts, i) {
				Some(p) => p,
				None => continue,
			};
			if pk != wallet {
				continue;
			}
			let pre_bal = pre[i].as_u64().unwrap_or(0);
			let post_bal = post[i].as_u64().unwrap_or(0);
			if post_bal <= pre_bal {
				continue;
			}
			let lamports = post_bal - pre_bal;
			let sol = lamports as f64 / LAMPORTS_PER_SOL;
			if sol < MIN_INCOMING_SOL {
				continue;
			}
			let Some(source) = system_transfer_source(instructions, wallet) else {
				continue;
			};

			let date_utc = tx.get("blockTime").and_then(|t| t.as_i64()).and_then(|secs| {
				chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
			});

			let (exchange, icon) = map
				.get(&source)
				.map(|(n, ic)| (Some(n.clone()), Some(ic.clone())))
				.unwrap_or((None, None));

			let payload = FundingPayload {
				exchange,
				icon,
				amount_sol: sol,
				signature: signature.clone(),
				date_utc,
				sender: source,
			};
			return Ok(Some(
				serde_json::to_string(&payload).map_err(|e| e.to_string())?,
			));
		}

		tokio::time::sleep(Duration::from_millis(50)).await;
	}

	Ok(None)
}
