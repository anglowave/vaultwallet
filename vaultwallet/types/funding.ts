/** Shape of `config/exchange-wallets.json` */
export interface ExchangeWalletsFile {
	version: number
	exchanges: ExchangeConfigEntry[]
}

export interface ExchangeConfigEntry {
	name: string
	/** Public path served from `/public` (e.g. `/exchanges/coinbase.svg`) */
	icon: string
	addresses: string[]
}

/** Stored JSON in vault `Funding` field */
export interface WalletFundingPayload {
	exchange: string | null
	icon: string | null
	amountSol: number
	signature: string
	dateUtc: string | null
	sender: string
}
