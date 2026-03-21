# vaultwallet-core

Rust library for **VaultWallet** vault files: KDBX 4.1–compatible binary layout (encryption, key derivation, HMAC-block framing, inner XML). On disk the format matches standard KDBX4; only the recommended extension and product naming differ.

## File extension

- Use **`.wlvlt`** for normal open/save paths (`DB_EXTENSION`, `DB_EXTENSION_WITH_DOT`).
- `open` rejects paths that do not end with `.wlvlt` before reading bytes.
- `save` writes via a `.tmp` file and renames atomically; it adds `.wlvlt` when the path has no extension.

## License

This crate is licensed under **GPL-3.0** (see `Cargo.toml`). When redistributing, comply with GPL-3.0 obligations. If you need a non-GPL stack, treat this as a starting point and re-implement from the public KDBX specifications rather than from KeePassXC sources.

## API sketch

```rust
use vaultwallet_core::{
    open, save, import_kdbx, CompositeKey, Database, WlvltPath,
};

let key = CompositeKey::new()
    .with_password("secret")
    .build()?;

let db = open("vault.wlvlt", key.clone())?;
save(&db, "vault.wlvlt", key.clone())?;

// One-time import from any extension (e.g. legacy binary):
import_kdbx("old.bin", "vault.wlvlt", key)?;
```

## Status

Core paths implemented: Argon2d / Argon2id, AES-256-CBC, ChaCha20, gzip compression, header + block HMAC, ChaCha20 protected stream (SHA-512 of inner stream key), AES-KDF (two-block ECB), Twofish-CBC decryption. The XML/database model is intentionally smaller than full KeePassXC; extend `format/xml.rs` and `db/` for complete feature parity.
