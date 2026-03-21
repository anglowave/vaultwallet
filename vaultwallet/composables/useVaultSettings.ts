import { watch } from 'vue'
import {
	applyVaultAccentStyles,
	clearVaultAccentStyles,
} from '~/utils/accentPalette'

const STORAGE_RPC_KEY = 'vaultwallet:solanaRpcUrl'
const STORAGE_LOCK_KEY = 'vaultwallet:lockAfterInactiveSec'
const STORAGE_CLIP_KEY = 'vaultwallet:clearPrivateClipboardSec'
const STORAGE_ACCENT_KEY = 'vaultwallet:accentHex'

const MAX_LOCK_SEC = 24 * 60 * 60
const MAX_CLIP_SEC = 60 * 60

function readStoredInt(
	key: string,
	fallback: number,
	min: number,
	max: number,
): number {
	if (!import.meta.client) return fallback
	try {
		const raw = localStorage.getItem(key)
		if (raw === null || raw === '') return fallback
		const n = Number.parseInt(raw, 10)
		if (!Number.isFinite(n)) return fallback
		return Math.min(max, Math.max(min, n))
	} catch {
		return fallback
	}
}

function readStoredAccent(): string {
	if (!import.meta.client) return ''
	try {
		return localStorage.getItem(STORAGE_ACCENT_KEY)?.trim() ?? ''
	} catch {
		return ''
	}
}

let accentWatchStarted = false

export function useVaultSettings() {
	const runtimeConfig = useRuntimeConfig()

	const defaultSolanaRpcUrl = computed(() => {
		const u = runtimeConfig.public.solanaRpcUrl
		const s = typeof u === 'string' ? u.trim() : ''
		return s || 'https://api.mainnet-beta.solana.com'
	})

	const customSolanaRpc = useState<string>('vaultwallet-custom-rpc', () => {
		if (!import.meta.client) return ''
		try {
			return localStorage.getItem(STORAGE_RPC_KEY) ?? ''
		} catch {
			return ''
		}
	})

	const lockAfterInactiveSeconds = useState<number>(
		'vaultwallet-lock-inactive-sec',
		() => readStoredInt(STORAGE_LOCK_KEY, 0, 0, MAX_LOCK_SEC),
	)

	const clearPrivateClipboardAfterSeconds = useState<number>(
		'vaultwallet-clear-clipboard-sec',
		() => readStoredInt(STORAGE_CLIP_KEY, 0, 0, MAX_CLIP_SEC),
	)

	const accentColorHex = useState<string>(
		'vaultwallet-accent-hex',
		() => readStoredAccent(),
	)

	if (import.meta.client && !accentWatchStarted) {
		accentWatchStarted = true
		watch(
			accentColorHex,
			(h) => {
				const t = h?.trim() ?? ''
				const root = document.documentElement
				if (!t) clearVaultAccentStyles(root)
				else applyVaultAccentStyles(root, t)
			},
			{ immediate: true },
		)
	}

	function solRpcUrl(): string {
		const c = customSolanaRpc.value.trim()
		return c || defaultSolanaRpcUrl.value
	}

	function setCustomSolanaRpc(url: string) {
		const t = url.trim()
		customSolanaRpc.value = t
		if (!import.meta.client) return
		try {
			if (t) localStorage.setItem(STORAGE_RPC_KEY, t)
			else localStorage.removeItem(STORAGE_RPC_KEY)
		} catch {
			/* ignore */
		}
	}

	function setLockAfterInactiveSeconds(sec: number) {
		const n = Math.min(MAX_LOCK_SEC, Math.max(0, Math.floor(sec)))
		lockAfterInactiveSeconds.value = n
		if (!import.meta.client) return
		try {
			localStorage.setItem(STORAGE_LOCK_KEY, String(n))
		} catch {
			/* ignore */
		}
	}

	function setClearPrivateClipboardAfterSeconds(sec: number) {
		const n = Math.min(MAX_CLIP_SEC, Math.max(0, Math.floor(sec)))
		clearPrivateClipboardAfterSeconds.value = n
		if (!import.meta.client) return
		try {
			localStorage.setItem(STORAGE_CLIP_KEY, String(n))
		} catch {
			/* ignore */
		}
	}

	function setAccentColorHex(hex: string) {
		accentColorHex.value = hex.trim()
		if (!import.meta.client) return
		try {
			const t = hex.trim()
			if (t) localStorage.setItem(STORAGE_ACCENT_KEY, t)
			else localStorage.removeItem(STORAGE_ACCENT_KEY)
		} catch {
			/* ignore */
		}
	}

	return {
		defaultSolanaRpcUrl,
		customSolanaRpc,
		solRpcUrl,
		setCustomSolanaRpc,
		lockAfterInactiveSeconds,
		clearPrivateClipboardAfterSeconds,
		accentColorHex,
		setLockAfterInactiveSeconds,
		setClearPrivateClipboardAfterSeconds,
		setAccentColorHex,
	}
}
