<script setup lang="ts">
import {
	applyVaultAccentStyles,
	clearVaultAccentStyles,
} from '~/utils/accentPalette'

const open = defineModel<boolean>('open', { default: false })

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

const ACCENT_PICKER_DEFAULT = '#2563EB'

const pickAccent = ref(ACCENT_PICKER_DEFAULT)
const skipAccentPreview = ref(true)
const accentPopoverOpen = ref(false)

function isDefaultPickerHex(hex: string) {
	return hex.replace(/^#/, '').toLowerCase() === '2563eb'
}

watch(pickAccent, (v) => {
	if (skipAccentPreview.value || !import.meta.client) return
	const root = document.documentElement
	if (!accentColorHex.value.trim() && isDefaultPickerHex(v)) {
		clearVaultAccentStyles(root)
		return
	}
	applyVaultAccentStyles(root, v)
})

watch(
	accentColorHex,
	(h) => {
		skipAccentPreview.value = true
		pickAccent.value = h.trim() || ACCENT_PICKER_DEFAULT
		nextTick(() => {
			skipAccentPreview.value = false
		})
	},
	{ immediate: true },
)

watch(accentPopoverOpen, (isOpen) => {
	if (!import.meta.client) return
	if (isOpen) {
		skipAccentPreview.value = true
		pickAccent.value = accentColorHex.value.trim() || ACCENT_PICKER_DEFAULT
		nextTick(() => {
			skipAccentPreview.value = false
		})
	} else {
		skipAccentPreview.value = true
		const h = accentColorHex.value.trim()
		const root = document.documentElement
		if (h) applyVaultAccentStyles(root, h)
		else clearVaultAccentStyles(root)
		pickAccent.value = h || ACCENT_PICKER_DEFAULT
		nextTick(() => {
			skipAccentPreview.value = false
		})
	}
})

function applyAccent(close?: () => void) {
	setAccentColorHex(pickAccent.value)
	toast.add({ title: 'Accent color saved', color: 'success' })
	close?.()
}

function resetAccent(close?: () => void) {
	skipAccentPreview.value = true
	setAccentColorHex('')
	pickAccent.value = ACCENT_PICKER_DEFAULT
	clearVaultAccentStyles(document.documentElement)
	nextTick(() => {
		skipAccentPreview.value = false
	})
	toast.add({
		title: 'Using default accent',
		description: 'Black / white primary from the app theme.',
		color: 'neutral',
	})
	close?.()
}

watch(open, (isOpen) => {
	if (!isOpen) accentPopoverOpen.value = false
})

const settingsTab = ref<'general' | 'appearance' | 'network'>('general')

const tabItems = [
	{
		label: 'General',
		value: 'general' as const,
		icon: 'i-lucide-sliders-horizontal',
		slot: 'general',
	},
	{
		label: 'Appearance',
		value: 'appearance' as const,
		icon: 'i-lucide-palette',
		slot: 'appearance',
	},
	{
		label: 'Network',
		value: 'network' as const,
		icon: 'i-lucide-radio-tower',
		slot: 'network',
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

const settingsCardUi = {
	root: 'w-full',
	header: 'px-4 py-3 sm:px-5',
	body: 'space-y-3 px-4 py-4 sm:px-5 sm:py-4',
}
</script>

<template>
	<UModal
		v-model:open="open"
		title="Settings"
		description="VaultWallet preferences"
		scrollable
		class="sm:max-w-lg"
	>
		<template #body>
			<UTabs
				v-model="settingsTab"
				:items="tabItems"
				variant="link"
				class="w-full"
				:ui="{ content: 'pt-3' }"
			>
				<template #general>
					<div class="flex w-full flex-col gap-4 text-left">
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
				</template>

				<template #appearance>
					<div class="flex w-full flex-col gap-4 text-left">
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
							<UFormField
								size="sm"
								label="Primary buttons and focus"
								description="Click the swatch to open the picker. Save to keep, or Default for theme black / white."
							>
								<UPopover v-model:open="accentPopoverOpen">
									<button
										type="button"
										class="ring-default focus-visible:ring-primary h-10 w-10 shrink-0 rounded-md border border-default shadow-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
										:class="
											!accentColorHex.trim()
												? 'bg-[linear-gradient(135deg,#1a1a1a_50%,#fafafa_50%)]'
												: ''
										"
										:style="
											accentColorHex.trim()
												? { backgroundColor: accentColorHex }
												: undefined
										"
										:aria-label="
											accentColorHex.trim()
												? `Accent color ${accentColorHex}`
												: 'Theme default accent, click to choose a color'
										"
									/>
									<template #content="{ close }">
										<div class="flex w-[min(100vw-2rem,18rem)] flex-col gap-3 p-3">
											<UColorPicker
												v-model="pickAccent"
												format="hex"
												size="sm"
												class="w-full"
											/>
											<div class="flex flex-wrap justify-end gap-1.5">
												<UButton
													size="xs"
													color="neutral"
													variant="soft"
													icon="i-lucide-rotate-ccw"
													:disabled="!accentColorHex.trim()"
													@click="resetAccent(close)"
												>
													Default
												</UButton>
												<UButton
													size="xs"
													icon="i-lucide-check"
													@click="applyAccent(close)"
												>
													Save
												</UButton>
											</div>
										</div>
									</template>
								</UPopover>
							</UFormField>
						</UCard>
					</div>
				</template>

				<template #network>
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
				</template>
			</UTabs>
		</template>
	</UModal>
</template>
