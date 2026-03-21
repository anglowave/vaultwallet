// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
	ssr: false,
	compatibilityDate: '2025-05-15',
	telemetry: false,
	devtools: { enabled: true },
	modules: ['@nuxt/ui'],
	css: ['~/assets/css/main.css'],
	colorMode: {
		preference: 'dark',
		fallback: 'dark',
		classSuffix: '',
	},
	devServer: {
		port: 1420,
	},
	vite: {
		clearScreen: false,
		envPrefix: ['VITE_', 'TAURI_'],
		server: {
			strictPort: true,
			watch: { ignored: ['**/src-tauri/**'] },
		},
	},
	ignore: ['**/src-tauri/**'],
	app: {
		head: {
			title: 'VaultWallet',
			meta: [
				{ name: 'viewport', content: 'width=device-width, initial-scale=1' },
			],
			link: [
				{ rel: 'preconnect', href: 'https://fonts.googleapis.com' },
				{
					rel: 'preconnect',
					href: 'https://fonts.gstatic.com',
					crossorigin: '',
				},
				{
					rel: 'stylesheet',
					href: 'https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100..900;1,100..900&display=swap',
				},
			],
		},
	},
	runtimeConfig: {
		public: {
			solanaRpcUrl:
				process.env.NUXT_PUBLIC_SOLANA_RPC_URL ||
				'https://api.mainnet-beta.solana.com',
		},
	},
})
