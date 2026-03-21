<script setup lang="ts">
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import {
	entryDisplayName,
	findGroup,
	truncateMiddle,
	type VaultTree,
} from '~/types/vault'
import { fundingTableLabel, parseFundingField } from '~/utils/fundingPayload'

const toast = useToast()
const tauri = useVaultTauri()
const runtimeConfig = useRuntimeConfig()

function solRpcUrl(): string {
	const u = runtimeConfig.public.solanaRpcUrl
	const s = typeof u === 'string' ? u.trim() : ''
	return s || 'https://api.mainnet-beta.solana.com'
}

const phase = ref<'gate' | 'vault'>('gate')
const gateTab = ref<'open' | 'create'>('open')

const vaultPath = ref('')
const password = ref('')
const createPasswordConfirm = ref('')
const createKdfStrength = ref(1)
const sessionPassword = ref('')

const KDF_PRESETS = [
	{ title: 'Quick', detail: '~16 MB' },
	{ title: 'Balanced', detail: '~64 MB' },
	{ title: 'Strong', detail: '~128 MB' },
	{ title: 'Maximum', detail: '~256 MB' },
] as const

const activeKdfPreset = computed(
	() => KDF_PRESETS[createKdfStrength.value] ?? KDF_PRESETS[1],
)

const loading = ref(false)
const balancesRefreshing = ref(false)
const tree = ref<VaultTree | null>(null)
const selectedGroupId = ref<string | null>(null)
let folderBalanceSeq = 0

const editModalOpen = ref(false)
const editEntryId = ref<string | null>(null)
const editEntryTitle = ref('')
const editOriginalTitle = ref('')

const editTitleDirty = computed(
	() => editEntryTitle.value.trim() !== editOriginalTitle.value.trim(),
)

const createWalletOpen = ref(false)
const createWalletName = ref('')
const createWalletPrivate = ref('')
const createWalletBalance = ref('')
const createWalletBalanceLoading = ref(false)
const createDerivedPublicKey = ref('')
let createDeriveTimer: ReturnType<typeof setTimeout> | null = null
let createDeriveSeq = 0


const renameOpen = ref(false)
const renameValue = ref('')
const renameTargetId = ref<string | null>(null)

const deleteGroupModal = ref(false)
const deleteEntryModal = ref(false)
const deletePendingEntryId = ref<string | null>(null)

const selectedGroup = computed(() => {
	if (!tree.value || !selectedGroupId.value) return null
	return findGroup(tree.value.root, selectedGroupId.value)
})

const groupEntries = computed(() => {
	const g = selectedGroup.value
	return g?.entries ?? []
})

const entryTableRows = computed(() =>
	groupEntries.value.map((e) => {
		const pk = e.fields.PublicKey?.trim() || ''
		const rawFunding = e.fields.Funding?.trim() || ''
		const parsed = parseFundingField(rawFunding)
		return {
			id: e.id,
			title: e.fields.Title?.trim() || '—',
			publicKeyShort: pk ? truncateMiddle(pk, 6) : '—',
			privateMask: e.fields.PrivateKey?.length ? '••••••••' : '—',
			balance: e.fields.Balance?.trim() || '—',
			fundingLabel: parsed
				? fundingTableLabel(parsed)
				: rawFunding || '—',
			fundingIcon: parsed?.icon ?? null,
		}
	}),
)

const entryTableColumns = [
	{ accessorKey: 'title', header: 'Name' },
	{ accessorKey: 'publicKeyShort', header: 'Public key' },
	{ accessorKey: 'privateMask', header: 'Private key' },
	{ accessorKey: 'balance', header: 'Balance' },
	{ accessorKey: 'fundingLabel', header: 'Wallet funding' },
	{
		id: 'actions',
		header: 'Actions',
		enableSorting: false,
		accessorFn: (row: { id: string }) => row.id,
	},
]

const deletePendingTitle = computed(() => {
	const id = deletePendingEntryId.value
	if (!id) return ''
	const e = groupEntries.value.find((x) => x.id === id)
	return e ? entryDisplayName(e) : ''
})

function openEditEntry(entryId: string) {
	const ent = groupEntries.value.find((e) => e.id === entryId)
	if (!ent) return
	editEntryId.value = entryId
	const t = ent.fields.Title ?? ''
	editEntryTitle.value = t
	editOriginalTitle.value = t
	editModalOpen.value = true
}

function openDeleteEntry(entryId: string) {
	deletePendingEntryId.value = entryId
	deleteEntryModal.value = true
}

function entryRowId(row: { id: string }) {
	return row.id
}

function cancelDeleteEntry() {
	deleteEntryModal.value = false
	deletePendingEntryId.value = null
}

function closeEditModal() {
	editModalOpen.value = false
}

watch(editModalOpen, (open) => {
	if (!open) {
		editEntryId.value = null
		editEntryTitle.value = ''
		editOriginalTitle.value = ''
	}
})

watch(createWalletOpen, (open) => {
	if (!open) {
		createDeriveSeq++
		createWalletName.value = ''
		createWalletPrivate.value = ''
		createWalletBalance.value = ''
		createWalletBalanceLoading.value = false
		createDerivedPublicKey.value = ''
		if (createDeriveTimer) {
			clearTimeout(createDeriveTimer)
			createDeriveTimer = null
		}
	}
})

function scheduleCreateWalletDerive(runId: number) {
	if (createDeriveTimer) clearTimeout(createDeriveTimer)
	createDeriveTimer = setTimeout(async () => {
		createDeriveTimer = null
		if (!createWalletOpen.value || runId !== createDeriveSeq) return
		const secret = createWalletPrivate.value.trim()
		if (!secret) {
			if (runId !== createDeriveSeq) return
			createDerivedPublicKey.value = ''
			createWalletBalance.value = ''
			createWalletBalanceLoading.value = false
			return
		}
		createWalletBalanceLoading.value = true
		let pub: string
		try {
			pub = await tauri.solanaPublicKeyFromPrivate(secret)
		} catch {
			if (runId !== createDeriveSeq) return
			createDerivedPublicKey.value = ''
			createWalletBalance.value = ''
			createWalletBalanceLoading.value = false
			return
		}
		if (runId !== createDeriveSeq) return
		createDerivedPublicKey.value = pub
		try {
			createWalletBalance.value = await tauri.solanaFetchBalance(solRpcUrl(), pub)
		} catch {
			if (runId !== createDeriveSeq) return
			createWalletBalance.value = 'Unavailable'
		} finally {
			if (runId === createDeriveSeq) {
				createWalletBalanceLoading.value = false
			}
		}
	}, 450)
}

watch(createWalletPrivate, () => {
	if (!createWalletOpen.value) return
	createDeriveSeq++
	const runId = createDeriveSeq
	createWalletBalance.value = ''
	createDerivedPublicKey.value = ''
	scheduleCreateWalletDerive(runId)
})

function showError(e: unknown) {
	const msg = e instanceof Error ? e.message : String(e)
	toast.add({
		title: 'Something went wrong',
		description: msg,
		color: 'error',
	})
}

async function pickVaultFile() {
	try {
		const p = await openDialog({
			filters: [{ name: 'VaultWallet', extensions: ['wlvlt'] }],
			multiple: false,
		})
		if (typeof p === 'string') vaultPath.value = p
	} catch (e) {
		showError(e)
	}
}

async function handleOpenVault() {
	if (!vaultPath.value.trim() || !password.value) {
		toast.add({ title: 'Path and password required', color: 'warning' })
		return
	}
	loading.value = true
	try {
		const t = await tauri.vaultOpen(vaultPath.value.trim(), password.value)
		tree.value = t
		sessionPassword.value = password.value
		password.value = ''
		selectedGroupId.value = t.root.id
		phase.value = 'vault'
		toast.add({ title: 'Vault unlocked', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

async function handleCreateVault() {
	if (!vaultPath.value.trim() || !password.value) {
		toast.add({ title: 'Path and password required', color: 'warning' })
		return
	}
	if (password.value !== createPasswordConfirm.value) {
		toast.add({
			title: 'Passwords do not match',
			description: 'Re-enter the same password in both fields.',
			color: 'warning',
		})
		return
	}
	loading.value = true
	try {
		await tauri.vaultCreate(
			vaultPath.value.trim(),
			password.value,
			createKdfStrength.value,
		)
		const t = await tauri.vaultOpen(vaultPath.value.trim(), password.value)
		tree.value = t
		sessionPassword.value = password.value
		password.value = ''
		createPasswordConfirm.value = ''
		selectedGroupId.value = t.root.id
		phase.value = 'vault'
		toast.add({ title: 'New vault ready', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

async function reloadTree() {
	if (!tree.value) return
	const t = await tauri.vaultOpen(
		vaultPath.value.trim(),
		sessionPassword.value,
	)
	tree.value = t
	const selG = selectedGroupId.value
	if (selG && !findGroup(t.root, selG)) {
		selectedGroupId.value = t.root.id
	}
}

watch(selectedGroupId, async (gid) => {
	const seq = ++folderBalanceSeq
	balancesRefreshing.value = false

	if (
		phase.value !== 'vault' ||
		!gid ||
		!tree.value ||
		!vaultPath.value.trim() ||
		!sessionPassword.value
	) {
		return
	}

	const g = findGroup(tree.value.root, gid)
	if (!g?.entries.length) return
	if (!g.entries.some((e) => e.fields.PublicKey?.trim())) return

	const rpc = solRpcUrl()
	balancesRefreshing.value = true
	try {
		for (const e of g.entries) {
			if (seq !== folderBalanceSeq) return
			const pk = e.fields.PublicKey?.trim()
			if (!pk) continue
			try {
				const bal = await tauri.solanaFetchBalance(rpc, pk)
				if (seq !== folderBalanceSeq) return
				await tauri.vaultUpdateEntry(
					vaultPath.value.trim(),
					sessionPassword.value,
					gid,
					e.id,
					{ ...e.fields, Balance: bal, PublicKey: pk },
				)
			} catch {
				// keep existing Balance on RPC / vault errors
			}
		}
		if (seq !== folderBalanceSeq) return
		await reloadTree()
	} finally {
		if (seq === folderBalanceSeq) balancesRefreshing.value = false
	}
})

function lockVault() {
	tree.value = null
	sessionPassword.value = ''
	selectedGroupId.value = null
	editModalOpen.value = false
	deletePendingEntryId.value = null
	deleteEntryModal.value = false
	phase.value = 'gate'
	createPasswordConfirm.value = ''
	createKdfStrength.value = 1
	toast.add({ title: 'Vault locked', color: 'neutral' })
}

watch(gateTab, (t) => {
	if (t === 'open') createPasswordConfirm.value = ''
})

async function addSubgroup() {
	const g = selectedGroup.value
	if (!g) return
	loading.value = true
	try {
		const name = `New folder ${g.children.length + 1}`
		await tauri.vaultAddGroup(
			vaultPath.value.trim(),
			sessionPassword.value,
			g.id,
			name,
		)
		await reloadTree()
		toast.add({ title: 'Folder added', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

function openRenameGroup(id: string, current: string) {
	renameTargetId.value = id
	renameValue.value = current
	renameOpen.value = true
}

function onFolderContextRename(p: { id: string; name: string }) {
	selectedGroupId.value = p.id
	openRenameGroup(p.id, p.name)
}

function onFolderContextDelete(id: string) {
	selectedGroupId.value = id
	deleteGroupModal.value = true
}

async function submitRenameGroup() {
	if (!renameTargetId.value) return
	loading.value = true
	try {
		await tauri.vaultRenameGroup(
			vaultPath.value.trim(),
			sessionPassword.value,
			renameTargetId.value,
			renameValue.value.trim() || 'Group',
		)
		renameOpen.value = false
		await reloadTree()
		toast.add({ title: 'Folder renamed', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

async function confirmDeleteGroup() {
	const id = selectedGroupId.value
	if (!id || !tree.value || tree.value.root.id === id) return
	loading.value = true
	try {
		await tauri.vaultDeleteGroup(
			vaultPath.value.trim(),
			sessionPassword.value,
			id,
		)
		deleteGroupModal.value = false
		editModalOpen.value = false
		selectedGroupId.value = tree.value.root.id
		await reloadTree()
		toast.add({ title: 'Folder removed', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

function openCreateWalletModal() {
	createWalletOpen.value = true
}

async function submitCreateWallet() {
	const g = selectedGroup.value
	if (!g) return
	const name = createWalletName.value.trim()
	const secret = createWalletPrivate.value.trim()
	if (!name) {
		toast.add({ title: 'Name is required', color: 'warning' })
		return
	}
	let pub: string
	try {
		pub = await tauri.solanaPublicKeyFromPrivate(secret)
	} catch {
		toast.add({
			title: 'Invalid private key',
			description: 'Use base58 or a 64-number JSON array.',
			color: 'warning',
		})
		return
	}
	loading.value = true
	try {
		const rpcUrl = solRpcUrl()
		let balance = createWalletBalance.value.trim()
		if (!balance || balance === 'Unavailable') {
			try {
				balance = await tauri.solanaFetchBalance(rpcUrl, pub)
			} catch {
				balance = 'Unavailable'
			}
		}
		let fundingStr = ''
		try {
			const trace = await tauri.solanaTraceFunding(rpcUrl, pub)
			if (trace) fundingStr = trace
		} catch {
			fundingStr = ''
		}
		const fields: Record<string, string> = {
			Title: name,
			PublicKey: pub,
			PrivateKey: secret,
			Balance: balance,
			Funding: fundingStr,
		}
		await tauri.vaultAddEntry(
			vaultPath.value.trim(),
			sessionPassword.value,
			g.id,
			fields,
		)
		createWalletOpen.value = false
		await reloadTree()
		toast.add({ title: 'Wallet saved', color: 'success' })
	} catch (e) {
		showError(e)
	} finally {
		loading.value = false
	}
}

async function saveEntry() {
	const g = selectedGroup.value
	const id = editEntryId.value
	if (!g || !id) return
	const ent = groupEntries.value.find((e) => e.id === id)
	if (!ent) return
	const name = editEntryTitle.value.trim()
	if (!name) {
		toast.add({ title: 'Name is required', color: 'warning' })
		return
	}
	loading.value = true
	try {
		const fields = { ...ent.fields, Title: name }
		await tauri.vaultUpdateEntry(
			vaultPath.value.trim(),
			sessionPassword.value,
			g.id,
			id,
			fields,
		)
		await reloadTree()
		toast.add({ title: 'Saved', color: 'success' })
		closeEditModal()
	} catch (err) {
		showError(err)
	} finally {
		loading.value = false
	}
}

async function confirmDeleteEntry() {
	const g = selectedGroup.value
	const id = deletePendingEntryId.value
	if (!g || !id) return
	loading.value = true
	try {
		await tauri.vaultDeleteEntry(
			vaultPath.value.trim(),
			sessionPassword.value,
			g.id,
			id,
		)
		deleteEntryModal.value = false
		if (editEntryId.value === id) editModalOpen.value = false
		deletePendingEntryId.value = null
		await reloadTree()
		toast.add({ title: 'Entry removed', color: 'success' })
	} catch (err) {
		showError(err)
	} finally {
		loading.value = false
	}
}

</script>

<template>
	<div class="bg-default flex h-full min-h-0 flex-col overflow-hidden">
		<!-- Gate: open / create -->
		<div
			v-if="phase === 'gate'"
			class="flex flex-1 items-center justify-center overflow-auto p-6"
		>
			<UCard class="w-full max-w-md overflow-hidden shadow-lg">
				<template #header>
					<div class="flex items-center gap-2.5">
						<div
							class="flex size-9 items-center justify-center rounded-lg bg-primary/15"
						>
							<UIcon name="i-lucide-shield" class="text-primary size-5" />
						</div>
						<div class="min-w-0">
							<h1 class="text-highlighted text-base font-semibold">
								VaultWallet
							</h1>
							<p class="text-muted text-xs">
								{{
									gateTab === 'create'
										? 'New .wlvlt — path, password, Argon2id cost.'
										: 'Open your .wlvlt (local only).'
								}}
							</p>
						</div>
					</div>
				</template>

				<UTabs
					v-model="gateTab"
					:items="[
						{ label: 'Open', value: 'open', icon: 'i-lucide-folder-open' },
						{ label: 'New vault', value: 'create', icon: 'i-lucide-sparkles' },
					]"
					class="mb-3"
				/>

				<div :class="gateTab === 'create' ? 'space-y-3' : 'space-y-4'">
					<UFormField
						:label="gateTab === 'create' ? 'Save as' : 'Vault file (.wlvlt)'"
					>
						<div class="flex gap-2">
							<UInput
								v-model="vaultPath"
								placeholder="C:\path\to\vault.wlvlt"
								class="flex-1"
								icon="i-lucide-file-lock"
							/>
							<UButton
								color="neutral"
								variant="soft"
								icon="i-lucide-folder-open"
								@click="pickVaultFile"
							>
								Browse
							</UButton>
						</div>
					</UFormField>

					<div
						v-if="gateTab === 'create'"
						class="grid grid-cols-2 gap-3"
					>
						<UFormField label="Password" class="min-w-0">
							<UInput
								v-model="password"
								type="password"
								autocomplete="new-password"
								class="w-full"
								icon="i-lucide-key-round"
							/>
						</UFormField>
						<UFormField label="Confirm" class="min-w-0">
							<UInput
								v-model="createPasswordConfirm"
								type="password"
								autocomplete="new-password"
								class="w-full"
								icon="i-lucide-key-round"
							/>
						</UFormField>
					</div>
					<UFormField v-else label="Password">
						<UInput
							v-model="password"
							type="password"
							autocomplete="current-password"
							icon="i-lucide-key-round"
						/>
					</UFormField>

					<div
						v-if="gateTab === 'create'"
						class="border-default bg-muted/15 space-y-2 rounded-lg border p-3"
					>
						<div class="flex items-center justify-between gap-2">
							<p class="text-highlighted text-xs font-medium">
								Argon2id · unlock cost
							</p>
							<UBadge
								color="primary"
								variant="subtle"
								class="shrink-0 tabular-nums text-[10px]"
							>
								{{ createKdfStrength + 1 }}/4
							</UBadge>
						</div>
						<p class="text-muted text-[11px] leading-snug">
							Higher = slower unlock, harder to brute-force.
						</p>
						<div class="px-0.5">
							<USlider
								v-model="createKdfStrength"
								:min="0"
								:max="3"
								:step="1"
								color="primary"
								:tooltip="{ content: { side: 'top' } }"
							/>
							<div class="text-muted mt-1 flex justify-between text-[10px]">
								<span>Fast</span>
								<span>Strong</span>
							</div>
						</div>
						<p class="text-muted font-mono text-[10px] leading-tight">
							{{ activeKdfPreset.title }} · {{ activeKdfPreset.detail }}
						</p>
					</div>

					<UButton
						v-if="gateTab === 'open'"
						block
						:loading="loading"
						icon="i-lucide-unlock"
						@click="handleOpenVault"
					>
						Unlock
					</UButton>
					<UButton
						v-else
						block
						:loading="loading"
						icon="i-lucide-shield-check"
						@click="handleCreateVault"
					>
						Create vault
					</UButton>
				</div>
			</UCard>
		</div>

		<!-- Main workspace -->
		<div v-else class="flex min-h-0 flex-1 flex-col overflow-hidden">
			<UHeader class="border-b border-default" :toggle="false">
				<template #left>
					<UBadge
						color="neutral"
						variant="subtle"
						class="max-w-[min(100%,28rem)] truncate font-mono text-xs"
					>
						{{ vaultPath }}
					</UBadge>
				</template>

				<template #right>
					<div class="flex items-center gap-2">
						<UColorModeButton />
						<UButton
							color="neutral"
							variant="soft"
							icon="i-lucide-lock"
							@click="lockVault"
						>
							Lock
						</UButton>
					</div>
				</template>
			</UHeader>

			<div class="flex min-h-0 flex-1 overflow-hidden">
				<!-- Groups -->
				<aside
					class="flex w-52 shrink-0 flex-col border-r border-default bg-default"
				>
					<div class="border-b border-default p-2">
						<p class="text-muted mb-2 text-xs font-medium uppercase tracking-wide">
							Folders
						</p>
						<UButton
							block
							size="sm"
							color="neutral"
							variant="soft"
							icon="i-lucide-folder-plus"
							:disabled="loading"
							@click="addSubgroup"
						>
							New folder
						</UButton>
					</div>
					<UScrollArea class="flex-1 p-1.5">
						<VaultTreeNode
							v-if="tree"
							:node="tree.root"
							:root-id="tree.root.id"
							:selected-group-id="selectedGroupId"
							:depth="0"
							@select="selectedGroupId = $event"
							@context-rename="onFolderContextRename"
							@context-delete="onFolderContextDelete"
						/>
					</UScrollArea>
				</aside>

				<!-- Entries table -->
				<main
					class="bg-default flex min-w-0 flex-1 flex-col overflow-hidden"
				>
					<div
						class="border-default flex shrink-0 flex-wrap items-center justify-between gap-3 border-b px-4 py-3"
					>
						<div>
							<h2 class="text-highlighted text-base font-semibold">
								{{ selectedGroup?.name || 'Wallets' }}
							</h2>
							<p class="text-muted text-xs">
								{{ groupEntries.length }}
								{{ groupEntries.length === 1 ? 'wallet' : 'wallets' }} in this folder
							</p>
						</div>
						<UButton
							icon="i-lucide-plus"
							:disabled="!selectedGroup || loading"
							@click="openCreateWalletModal"
						>
							Add wallet
						</UButton>
					</div>

					<UScrollArea class="min-h-0 flex-1 p-4">
						<UCard v-if="selectedGroup" class="overflow-hidden">
							<UTable
								:data="entryTableRows"
								:columns="entryTableColumns"
								:loading="loading || balancesRefreshing"
								empty="No wallets in this folder. Use Add wallet to create one."
								class="w-full shrink-0"
								:get-row-id="entryRowId"
							>
								<template #balance-cell="{ row }">
									<span class="font-mono text-sm tabular-nums">{{
										row.original.balance
									}}</span>
								</template>
								<template #fundingLabel-cell="{ row }">
									<div class="flex min-w-0 items-center gap-2">
										<img
											v-if="row.original.fundingIcon"
											:src="row.original.fundingIcon"
											alt=""
											class="size-5 shrink-0 rounded object-contain"
										/>
										<UIcon
											v-else
											name="i-lucide-building-2"
											class="text-muted size-4 shrink-0"
										/>
										<span class="truncate text-sm">{{
											row.original.fundingLabel
										}}</span>
									</div>
								</template>
								<template #actions-cell="{ row }">
									<div class="flex justify-end gap-1">
										<UButton
											color="neutral"
											variant="ghost"
											size="xs"
											icon="i-lucide-pencil"
											aria-label="Rename wallet"
											@click.stop="openEditEntry(row.original.id)"
										/>
										<UButton
											color="error"
											variant="ghost"
											size="xs"
											icon="i-lucide-trash-2"
											aria-label="Delete wallet"
											@click.stop="openDeleteEntry(row.original.id)"
										/>
									</div>
								</template>
							</UTable>
						</UCard>
					</UScrollArea>
				</main>
			</div>
		</div>

		<UModal v-model:open="createWalletOpen">
			<template #content>
				<UCard class="w-full max-w-lg">
					<template #header>
						<div>
							<h3 class="text-highlighted text-lg font-semibold">Add wallet</h3>
							<p class="text-muted mt-1 text-sm">
								Enter a label and private key. The public key and balance are
								derived via Solana RPC.
							</p>
						</div>
					</template>

					<div class="space-y-4">
						<UFormField label="Name" required>
							<UInput
								v-model="createWalletName"
								placeholder="My trading wallet"
								class="w-full"
								autofocus
							/>
						</UFormField>
						<UFormField
							label="Private key"
							required
							hint="Base58 secret key or Phantom-style JSON byte array"
						>
							<UInput
								v-model="createWalletPrivate"
								type="password"
								class="w-full font-mono text-sm"
								autocomplete="off"
							/>
						</UFormField>
						<UFormField label="Public key">
							<UInput
								:model-value="createDerivedPublicKey || '—'"
								readonly
								disabled
								class="w-full font-mono text-sm"
							/>
						</UFormField>
						<UFormField label="Balance">
							<div class="flex items-center gap-2">
								<span
									v-if="createWalletBalanceLoading"
									class="text-muted text-sm"
								>Fetching…</span>
								<span
									v-else
									class="font-mono text-sm tabular-nums"
								>{{ createWalletBalance || '—' }}</span>
							</div>
						</UFormField>
					</div>

					<template #footer>
						<div class="flex w-full flex-wrap justify-end gap-2">
							<UButton
								color="neutral"
								variant="ghost"
								@click="createWalletOpen = false"
							>
								Cancel
							</UButton>
							<UButton
								:loading="loading"
								icon="i-lucide-plus"
								@click="submitCreateWallet"
							>
								Save wallet
							</UButton>
						</div>
					</template>
				</UCard>
			</template>
		</UModal>

		<UModal v-model:open="editModalOpen">
			<template #content>
				<UCard class="w-full max-w-md">
					<template #header>
						<div>
							<h3 class="text-highlighted text-lg font-semibold">Rename wallet</h3>
							<p class="text-muted mt-1 text-sm">
								Only the display name can be changed. Keys, balance, and funding
								stay as stored in the vault.
							</p>
						</div>
					</template>

					<UFormField label="Name" required>
						<UInput
							v-model="editEntryTitle"
							class="w-full"
							placeholder="My trading wallet"
							autofocus
							@keydown.enter="saveEntry"
						/>
					</UFormField>

					<template #footer>
						<div class="flex w-full flex-wrap justify-end gap-2">
							<UButton color="neutral" variant="ghost" @click="closeEditModal">
								Cancel
							</UButton>
							<UButton
								:disabled="!editTitleDirty || loading"
								:loading="loading"
								icon="i-lucide-save"
								@click="saveEntry"
							>
								Save
							</UButton>
						</div>
					</template>
				</UCard>
			</template>
		</UModal>

		<UModal v-model:open="renameOpen">
			<template #content>
				<UCard>
					<template #header>
						<h3 class="text-highlighted font-semibold">Rename folder</h3>
					</template>
					<UFormField label="Name">
						<UInput v-model="renameValue" autofocus @keydown.enter="submitRenameGroup" />
					</UFormField>
					<template #footer>
						<div class="flex justify-end gap-2">
							<UButton color="neutral" variant="ghost" @click="renameOpen = false">
								Cancel
							</UButton>
							<UButton :loading="loading" @click="submitRenameGroup">Save</UButton>
						</div>
					</template>
				</UCard>
			</template>
		</UModal>

		<UModal v-model:open="deleteGroupModal">
			<template #content>
				<UCard>
					<template #header>
						<h3 class="text-highlighted font-semibold">Delete folder?</h3>
					</template>
					<p class="text-muted text-sm">
						This removes the folder and everything inside it. This cannot be undone.
					</p>
					<template #footer>
						<div class="flex justify-end gap-2">
							<UButton color="neutral" variant="ghost" @click="deleteGroupModal = false">
								Cancel
							</UButton>
							<UButton color="error" :loading="loading" @click="confirmDeleteGroup">
								Delete
							</UButton>
						</div>
					</template>
				</UCard>
			</template>
		</UModal>

		<UModal v-model:open="deleteEntryModal">
			<template #content>
				<UCard>
					<template #header>
						<h3 class="text-highlighted font-semibold">Delete wallet?</h3>
					</template>
					<p class="text-muted text-sm leading-relaxed">
						Permanently remove
						<strong class="text-highlighted">{{ deletePendingTitle }}</strong>
						from this vault? This cannot be undone.
					</p>
					<template #footer>
						<div class="flex justify-end gap-2">
							<UButton color="neutral" variant="ghost" @click="cancelDeleteEntry">
								Cancel
							</UButton>
							<UButton color="error" :loading="loading" @click="confirmDeleteEntry">
								Delete
							</UButton>
						</div>
					</template>
				</UCard>
			</template>
		</UModal>
	</div>
</template>
