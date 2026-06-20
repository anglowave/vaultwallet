import { isTauri } from '@tauri-apps/api/core'
import {
	clear as tauriClear,
	readText as tauriReadText,
	writeText as tauriWriteText,
} from '@tauri-apps/plugin-clipboard-manager'

/**
 * Clipboard helper that prefers the native Tauri clipboard plugin.
 *
 * The browser `navigator.clipboard` API is unreliable inside the Tauri webview:
 * a write triggered from a `setTimeout` has no user activation, and WebView2
 * rejects empty-string writes — so timer-based clearing silently fails. The
 * native plugin works without a user gesture and can truly clear the clipboard.
 *
 * Falls back to `navigator.clipboard` for web-only dev (`bun run dev`).
 */
export function useSecureClipboard() {
	const native = import.meta.client && isTauri()

	async function copy(text: string): Promise<void> {
		if (native) {
			await tauriWriteText(text)
			return
		}
		await navigator.clipboard.writeText(text)
	}

	/**
	 * Clear the clipboard, but only if it still holds `expected`. This avoids
	 * wiping content the user copied from elsewhere after the original copy.
	 */
	async function clearIfMatches(expected: string): Promise<void> {
		if (native) {
			try {
				if ((await tauriReadText()) !== expected) return
			} catch {
				/* read may fail (e.g. non-text clipboard); clear anyway below */
			}
			await tauriClear()
			return
		}
		try {
			if ((await navigator.clipboard.readText()) !== expected) return
		} catch {
			/* read may be blocked; fall through to best-effort clear */
		}
		try {
			await navigator.clipboard.writeText('')
		} catch {
			/* some hosts reject empty clipboard writes */
		}
	}

	return { copy, clearIfMatches }
}
