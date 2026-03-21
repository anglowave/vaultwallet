<script setup lang="ts">
import {
	applyVaultAccentStyles,
	clearVaultAccentStyles,
} from '~/utils/accentPalette'

useSeoMeta({ title: 'Settings · VaultWallet' })

const toast = useToast()
const {
	customSolanaRpc,
	defaultSolanaRpcUrl,
	setCustomSolanaRpc,
	solRpcUrl,
	lockAfterInactiveSeconds,
	clearPrivateClipboardAfterSeconds,
	accentColorHex,
	setLockAfterInactiveSeconds,
	setClearPrivateClipboardAfterSeconds,
	setAccentColorHex,
} = useVaultSettings()

type SettingsSection = 'general' | 'appearance' | 'network'

const section = ref<SettingsSection>('general')

const sidebarNav = [
	{
		id: 'general' as const,
		label: 'General',
		icon: 'i-lucide-sliders-horizontal',
	},
	{
		id: 'appearance' as const,
		label: 'Appearance',
		icon: 'i-lucide-palette',
	},
	{
		id: 'network' as const,
		label: 'Network',
		icon: 'i-lucide-radio-tower',
	},
]

const rpcDraft = ref('')

watch(
	customSolanaRpc,
	(v) => {
		rpcDraft.value = v
	},
	{ immediate: true },
)

const effectiveRpc = computed(() => solRpcUrl())

/** UI in whole minutes; storage remains seconds for idle lock */
const lockMin = computed({
	get() {
		const s = lockAfterInactiveSeconds.value
		if (s <= 0) return 0
		return Math.round(s / 60)
	},
	set(v: number | null | undefined) {
		const m =
			typeof v === 'number' && Number.isFinite(v) ? Math.max(0, Math.floor(v)) : 0
		setLockAfterInactiveSeconds(m * 60)
	},
})

const clipSec = computed({
	get: () => clearPrivateClipboardAfterSeconds.value,
	set: (v: number | null | undefined) => {
		const n = typeof v === 'number' && Number.isFinite(v) ? v : 0
		setClearPrivateClipboardAfterSeconds(n)
	},
})

const ACCENT_PICKER_DEFAULT = '#2563EB'

const pickAccent = ref(ACCENT_PICKER_DEFAULT)
const skipAccentPreview = ref(true)

function isDefaultPickerHex(hex: string) {
	return hex.replace(/^#/, '').toLowerCase() === '2563eb'
}

watch(pickAccent, (v) => {
	if (skipAccentPreview.value || !import.meta.client) return
	if (!accentColorHex.value.trim() && isDefaultPickerHex(v)) {
		clearVaultAccentStyles(document.documentElement)
		return
	}
	applyVaultAccentStyles(document.documentElement, v)
})

watch(accentColorHex, (h) => {
	skipAccentPreview.value = true
	pickAccent.value = h.trim() || ACCENT_PICKER_DEFAULT
	nextTick(() => {
		skipAccentPreview.value = false
	})
}, { immediate: true })

onMounted(() => {
	nextTick(() => {
		skipAccentPreview.value = false
	})
})

onUnmounted(() => {
	if (!import.meta.client) return
	const h = accentColorHex.value.trim()
	if (h) applyVaultAccentStyles(document.documentElement, h)
	else clearVaultAccentStyles(document.documentElement)
})

function saveRpc() {
	setCustomSolanaRpc(rpcDraft.value)
	toast.add({
		title: 'RPC URL saved',
		description: 'Balance checks and new wallets use this endpoint.',
		color: 'success',
	})
}

function resetRpc() {
	rpcDraft.value = ''
	setCustomSolanaRpc('')
	toast.add({
		title: 'Using default RPC',
		description: defaultSolanaRpcUrl.value,
		color: 'neutral',
	})
}

function discardRpcDraft() {
	rpcDraft.value = customSolanaRpc.value
}

function applyAccent() {
	setAccentColorHex(pickAccent.value)
	toast.add({ title: 'Accent color saved', color: 'success' })
}

function resetAccent() {
	skipAccentPreview.value = true
	setAccentColorHex('')
	pickAccent.value = ACCENT_PICKER_DEFAULT
	clearVaultAccentStyles(document.documentElement)
	nextTick(() => {
		skipAccentPreview.value = false
	})
	toast.add({
		title: 'Using default accent',
		description: 'Black / white primary colors from the app theme.',
		color: 'neutral',
	})
}

/** Cards fill the panel width; padding matches Nuxt UI defaults more closely */
const settingsCardUi = {
	root: 'w-full',
	header: 'px-4 py-3 sm:px-5',
	body: 'space-y-3 px-4 py-4 sm:px-5 sm:py-4',
}
</script>

<template>
	<div class="bg-default flex min-h-0 flex-1 flex-col overflow-hidden">
		<UHeader
			class="border-b border-default min-h-11 shrink-0 py-1.5"
			:toggle="false"
			:ui="{
				container: 'max-w-none mx-0 w-full justify-between gap-3 px-3 sm:px-4',
				left: 'min-w-0 flex-1 justify-start',
				center: 'hidden',
				right: 'shrink-0 justify-end',
			}"
		>
			<template #left>
				<div class="flex min-w-0 items-center gap-2">
					<UButton
						to="/"
						color="neutral"
						variant="ghost"
						size="sm"
						icon="i-lucide-arrow-left"
					>
						Back
					</UButton>
					<span class="text-highlighted truncate text-sm font-semibold">
						Settings
					</span>
				</div>
			</template>

			<template #right>
				<UColorModeButton size="sm" />
			</template>
		</UHeader>

		<div class="flex min-h-0 flex-1">
			<aside
				class="border-default flex w-52 shrink-0 flex-col border-r bg-default"
			>
				<nav
					class="flex flex-col py-2"
					role="tablist"
					aria-label="Settings sections"
				>
					<button
						v-for="item in sidebarNav"
						:key="item.id"
						type="button"
						role="tab"
						:aria-selected="section === item.id"
						:class="[
							'relative flex w-full items-center gap-2.5 rounded-none py-2 pl-3 pr-2 text-left text-sm transition-colors',
							section === item.id
								? 'text-highlighted bg-elevated/60 font-medium before:absolute before:inset-y-1 before:left-0 before:w-0.5 before:rounded-full before:bg-primary'
								: 'text-muted hover:bg-elevated/35 hover:text-default',
						]"
						@click="section = item.id"
					>
						<UIcon
							:name="item.icon"
							class="size-4 shrink-0 opacity-80"
							:class="section === item.id ? 'text-primary' : ''"
						/>
						<span class="min-w-0 truncate">{{ item.label }}</span>
					</button>
				</nav>
			</aside>

			<UScrollArea class="min-h-0 min-w-0 flex-1">
				<div
					class="flex w-full max-w-none flex-col px-3 py-2 pb-8 text-left sm:px-5 sm:pr-6"
				>
					<header class="mb-4 max-w-md space-y-0.5 text-left">
						<h1 class="text-highlighted text-sm font-semibold tracking-tight">
							{{
								section === 'general'
									? 'General'
									: section === 'appearance'
										? 'Appearance'
										: 'Network'
							}}
						</h1>
						<p class="text-muted max-w-md text-xs leading-snug">
							{{
								section === 'general'
									? 'Timers and clipboard.'
									: section === 'appearance'
										? 'Theme and accent.'
										: 'RPC for balances and traces.'
							}}
						</p>
					</header>

					<div
						v-show="section === 'general'"
						class="flex w-full max-w-md flex-col gap-4 text-left"
					>
						<UCard :ui="settingsCardUi">
							<template #header>
								<h2 class="text-highlighted text-sm font-semibold">
									Auto-lock
								</h2>
							</template>
							<UFormField
								size="sm"
								label="Lock after inactivity"
								description="Whole minutes of no input. 0 = never while the vault is open."
							>
								<div class="flex flex-wrap items-center gap-2">
									<UInputNumber
										v-model="lockMin"
										size="sm"
										:min="0"
										:max="1440"
										:step="1"
										class="w-32"
									/>
									<span class="text-muted text-xs">min</span>
								</div>
							</UFormField>
						</UCard>

						<UCard :ui="settingsCardUi">
							<template #header>
								<h2 class="text-highlighted text-sm font-semibold">
									Clipboard
								</h2>
							</template>
							<UFormField
								size="sm"
								label="Clear private key from clipboard"
								description="Delay after table copy. 0 = off. Public keys unchanged."
							>
								<div class="flex flex-wrap items-center gap-2">
									<UInputNumber
										v-model="clipSec"
										size="sm"
										:min="0"
										:max="3600"
										:step="5"
										class="w-32"
									/>
									<span class="text-muted text-xs">sec</span>
								</div>
							</UFormField>
						</UCard>
					</div>

					<div
						v-show="section === 'appearance'"
						class="flex w-full max-w-md flex-col gap-4 text-left"
					>
						<UCard :ui="settingsCardUi">
							<template #header>
								<h2 class="text-highlighted text-sm font-semibold">
									Theme
								</h2>
							</template>
							<UFormField
								size="sm"
								label="Color mode"
								description="System, light, or dark."
							>
								<UColorModeSelect
									size="sm"
									class="w-full max-w-sm"
								/>
							</UFormField>
						</UCard>

						<UCard :ui="settingsCardUi">
							<template #header>
								<h2 class="text-highlighted text-sm font-semibold">
									Accent color
								</h2>
							</template>
							<div class="space-y-3">
								<p class="text-muted text-xs leading-snug">
									Preview in the picker; save to keep. Reset restores default
									primary.
								</p>
								<UColorPicker
									v-model="pickAccent"
									format="hex"
									size="sm"
									class="w-full max-w-sm"
								/>
								<div class="flex flex-wrap justify-start gap-1.5">
									<UButton
										size="xs"
										icon="i-lucide-check"
										@click="applyAccent"
									>
										Save accent
									</UButton>
									<UButton
										size="xs"
										color="neutral"
										variant="soft"
										icon="i-lucide-rotate-ccw"
										:disabled="!accentColorHex.trim()"
										@click="resetAccent"
									>
										Default accent
									</UButton>
								</div>
							</div>
						</UCard>
					</div>

					<div
						v-show="section === 'network'"
						class="flex w-full max-w-md flex-col gap-4 text-left"
					>
						<UCard :ui="settingsCardUi">
							<template #header>
								<h2 class="text-highlighted text-sm font-semibold">
									Solana RPC
								</h2>
							</template>

							<div class="space-y-3">
								<UAlert
									color="neutral"
									variant="subtle"
									class="text-xs"
									icon="i-lucide-info"
									title="Effective RPC"
									:description="effectiveRpc"
									:ui="{
										title: 'text-xs font-medium',
										description: 'text-xs break-all font-mono',
									}"
								/>

								<UFormField
									size="sm"
									label="Custom RPC URL"
									:hint="`Empty = default (${defaultSolanaRpcUrl}).`"
								>
									<UInput
										v-model="rpcDraft"
										size="sm"
										class="w-full font-mono text-xs"
										placeholder="https://…"
										icon="i-lucide-link"
										autocomplete="off"
									/>
								</UFormField>

								<div class="flex flex-wrap justify-start gap-1.5">
									<UButton
										size="xs"
										icon="i-lucide-save"
										:disabled="rpcDraft.trim() === customSolanaRpc.trim()"
										@click="saveRpc"
									>
										Save RPC
									</UButton>
									<UButton
										size="xs"
										color="neutral"
										variant="soft"
										icon="i-lucide-rotate-ccw"
										:disabled="
											!customSolanaRpc.trim() && !rpcDraft.trim()
										"
										@click="resetRpc"
									>
										Use default
									</UButton>
									<UButton
										v-if="rpcDraft !== customSolanaRpc"
										size="xs"
										color="neutral"
										variant="ghost"
										@click="discardRpcDraft"
									>
										Discard
									</UButton>
								</div>
							</div>
						</UCard>
					</div>
				</div>
			</UScrollArea>
		</div>
	</div>
</template>
