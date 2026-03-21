export interface VaultEntry {
	id: string
	fields: Record<string, string>
}

export interface VaultGroup {
	id: string
	name: string
	entries: VaultEntry[]
	children: VaultGroup[]
}

export interface VaultTree {
	metadata: { generator: string }
	root: VaultGroup
}

/** Passed to Tauri when creating a vault with advanced crypto options */
export interface VaultCreateCrypto {
	cipher: 'aes256cbc' | 'chacha20' | 'twofishcbc'
	argon2Flavor: 'id' | 'd'
	iterations: number
	memoryMib: number
	parallelism: number
}

/** Encrypted vault string fields for each Solana wallet entry */
export const WALLET_FIELD_KEYS = [
	'Title',
	'PublicKey',
	'PrivateKey',
	'Balance',
	'Funding',
] as const

export type WalletFieldKey = (typeof WALLET_FIELD_KEYS)[number]

export function entryDisplayName(entry: VaultEntry): string {
	const t = entry.fields.Title?.trim()
	if (t) return t
	const pk = entry.fields.PublicKey?.trim()
	if (pk) return truncateMiddle(pk, 8)
	return 'Untitled wallet'
}

export function truncateMiddle(s: string, edge: number): string {
	if (s.length <= edge * 2 + 1) return s
	return `${s.slice(0, edge)}…${s.slice(-edge)}`
}

export function findGroup(root: VaultGroup, id: string): VaultGroup | null {
	if (root.id === id) return root
	for (const c of root.children) {
		const f = findGroup(c, id)
		if (f) return f
	}
	return null
}
