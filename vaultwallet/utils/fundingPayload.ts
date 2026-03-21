import type { WalletFundingPayload } from '~/types/funding'
import { truncateMiddle } from '~/types/vault'

/** Exchange name, or shortened funder address when not in the exchange directory. */
function fundingSourceDisplayName(p: WalletFundingPayload): string {
	if (p.exchange) return p.exchange
	const s = p.sender.trim()
	if (s) return truncateMiddle(s, 4)
	return 'Unknown'
}

export function parseFundingField(raw: string): WalletFundingPayload | null {
	const s = raw.trim()
	if (!s || s === '—') return null
	try {
		const o = JSON.parse(s) as WalletFundingPayload
		if (
			typeof o.amountSol === 'number' &&
			typeof o.signature === 'string' &&
			typeof o.sender === 'string'
		) {
			return {
				exchange: typeof o.exchange === 'string' ? o.exchange : null,
				icon: typeof o.icon === 'string' ? o.icon : null,
				amountSol: o.amountSol,
				signature: o.signature,
				dateUtc: typeof o.dateUtc === 'string' ? o.dateUtc : null,
				sender: o.sender,
			}
		}
	} catch {
		/* legacy plain text */
	}
	return null
}

/** Short relative time, e.g. `1d ago`, `3h ago` (null if unparseable). */
export function formatFundingTimeAgo(
	dateUtc: string | null | undefined,
): string | null {
	const s = dateUtc?.trim()
	if (!s) return null
	const t = Date.parse(s)
	if (Number.isNaN(t)) return null
	let diffSec = Math.floor((Date.now() - t) / 1000)
	if (diffSec < 0) diffSec = 0
	if (diffSec < 45) return 'just now'
	const diffMin = Math.floor(diffSec / 60)
	if (diffMin < 60) return `${diffMin}m ago`
	const diffHr = Math.floor(diffSec / 3600)
	if (diffHr < 24) return `${diffHr}h ago`
	const diffDay = Math.floor(diffSec / 86400)
	if (diffDay < 7) return `${diffDay}d ago`
	const diffWk = Math.floor(diffDay / 7)
	if (diffWk < 5) return `${diffWk}w ago`
	const diffMo = Math.floor(diffDay / 30)
	if (diffMo < 12) return `${diffMo}mo ago`
	const diffYr = Math.floor(diffDay / 365)
	return `${Math.max(1, diffYr)}y ago`
}

export function fundingTableLabel(p: WalletFundingPayload): string {
	const amt = p.amountSol.toFixed(4)
	const ago = formatFundingTimeAgo(p.dateUtc)
	const tail = ago ? ` · ${ago}` : ''
	return `${fundingSourceDisplayName(p)} · ${amt} SOL${tail}`
}

/** Exchange + amount only (no relative time) — for multi-line summaries. */
export function fundingSummaryHeadline(p: WalletFundingPayload): string {
	const amt = p.amountSol.toFixed(4)
	return `${fundingSourceDisplayName(p)} · ${amt} SOL`
}
