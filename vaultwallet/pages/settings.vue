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

const lockSec = computed({
	get: () => lockAfterInactiveSeconds.value,
	set: (v: number | null | undefined) => {
		const n = typeof v === 'number' && Number.isFinite(v) ? v : 0
		setLockAfterInactiveSeconds(n)
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
</script>

<template>
	<div class="bg-default flex min-h-0 flex-1 flex-col overflow-hidden">
		<UHeader class="border-b border-default" :toggle="false">
			<template #left>
				<UButton
					to="/"
					color="neutral"
					variant="ghost"
					icon="i-lucide-arrow-left"
				>
					Back
				</UButton>
			</template>

			<span class="text-highlighted truncate font-semibold">Settings</span>

			<template #right>
				<UColorModeButton />
			</template>
		</UHeader>

		<div class="flex min-h-0 flex-1">
			<aside
				class="border-default bg-muted/10 flex w-56 shrink-0 flex-col border-r"
			>
				<p class="text-muted px-3 py-3 text-xs font-medium uppercase tracking-wide">
					Menu
				</p>
				<nav class="flex flex-col gap-0.5 p-2 pt-0">
					<UButton
						v-for="item in sidebarNav"
						:key="item.id"
						block
						:color="section === item.id ? 'primary' : 'neutral'"
						:variant="section === item.id ? 'soft' : 'ghost'"
						class="justify-start gap-2"
						@click="section = item.id"
					>
						<UIcon :name="item.icon" class="size-4 shrink-0" />
						<span class="truncate">{{ item.label }}</span>
					</UButton>
				</nav>
			</aside>

			<UScrollArea class="min-h-0 min-w-0 flex-1">
				<div class="mx-auto max-w-2xl p-6 pb-12">
					<UPageHeader
						class="mb-8"
						:title="
							section === 'general'
								? 'General'
								: section === 'appearance'
									? 'Appearance'
									: 'Network'
						"
						:description="
							section === 'general'
								? 'Security timers and clipboard behavior.'
								: section === 'appearance'
									? 'Theme and primary accent color.'
									: 'Solana RPC endpoint for balances and traces.'
						"
					/>

					<div v-show="section === 'general'" class="space-y-6">
						<UCard>
							<template #header>
								<h2 class="text-highlighted text-base font-semibold">
									Auto-lock
								</h2>
							</template>
							<UFormField
								label="Lock after inactivity"
								description="0 = never lock automatically while the vault is open."
							>
								<div class="flex flex-wrap items-center gap-3">
									<UInputNumber
										v-model="lockSec"
										:min="0"
										:max="86400"
										:step="30"
										class="w-40"
									/>
									<span class="text-muted text-sm">seconds</span>
								</div>
							</UFormField>
						</UCard>

						<UCard>
							<template #header>
								<h2 class="text-highlighted text-base font-semibold">
									Clipboard
								</h2>
							</template>
							<UFormField
								label="Clear private key from clipboard"
								description="After copying a private key from the wallet table, clear the clipboard after this delay. 0 = do not clear. Public key copies are not cleared."
							>
								<div class="flex flex-wrap items-center gap-3">
									<UInputNumber
										v-model="clipSec"
										:min="0"
										:max="3600"
										:step="5"
										class="w-40"
									/>
									<span class="text-muted text-sm">seconds</span>
								</div>
							</UFormField>
						</UCard>
					</div>

					<div v-show="section === 'appearance'" class="space-y-6">
						<UCard>
							<template #header>
								<h2 class="text-highlighted text-base font-semibold">
									Theme
								</h2>
							</template>
							<UFormField
								label="Color mode"
								description="System, light, or dark."
							>
								<UColorModeSelect class="w-full max-w-xs" />
							</UFormField>
						</UCard>

						<UCard>
							<template #header>
								<h2 class="text-highlighted text-base font-semibold">
									Accent color
								</h2>
							</template>
							<div class="space-y-4">
								<p class="text-muted text-sm leading-relaxed">
									Adjust the picker to preview; save when you are happy.
									Reset restores the default black / white primary style.
								</p>
								<UColorPicker
									v-model="pickAccent"
									format="hex"
									class="max-w-sm"
								/>
								<div class="flex flex-wrap gap-2">
									<UButton icon="i-lucide-check" @click="applyAccent">
										Save accent
									</UButton>
									<UButton
										color="neutral"
										variant="soft"
										icon="i-lucide-rotate-ccw"
										:disabled="!accentColorHex.trim()"
										@click="resetAccent"
									>
										Use default accent
									</UButton>
								</div>
							</div>
						</UCard>
					</div>

					<div v-show="section === 'network'" class="space-y-6">
						<UCard>
							<template #header>
								<h2 class="text-highlighted text-base font-semibold">
									Solana RPC
								</h2>
							</template>

							<div class="space-y-4">
								<UAlert
									color="neutral"
									variant="subtle"
									icon="i-lucide-info"
									title="Effective RPC"
									:description="effectiveRpc"
								/>

								<UFormField
									label="Custom RPC URL"
									:hint="`Leave empty for built-in default (${defaultSolanaRpcUrl}).`"
								>
									<UInput
										v-model="rpcDraft"
										class="w-full font-mono text-sm"
										placeholder="https://…"
										icon="i-lucide-link"
										autocomplete="off"
									/>
								</UFormField>

								<div class="flex flex-wrap gap-2">
									<UButton
										icon="i-lucide-save"
										:disabled="rpcDraft.trim() === customSolanaRpc.trim()"
										@click="saveRpc"
									>
										Save RPC
									</UButton>
									<UButton
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
										color="neutral"
										variant="ghost"
										@click="discardRpcDraft"
									>
										Discard edits
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
