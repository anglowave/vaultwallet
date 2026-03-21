import { invoke } from '@tauri-apps/api/core'
import type { VaultGroup, VaultEntry, VaultTree } from '~/types/vault'

export function useVaultTauri() {
	async function vaultCreate(
		path: string,
		password: string,
		kdfStrength: number,
	) {
		await invoke('vault_create', { path, password, kdfStrength })
	}

	async function vaultOpen(path: string, password: string): Promise<VaultTree> {
		return invoke<VaultTree>('vault_open', { path, password })
	}

	async function vaultAddGroup(
		path: string,
		password: string,
		parentGroupId: string,
		name: string,
	): Promise<VaultGroup> {
		return invoke<VaultGroup>('vault_add_group', {
			path,
			password,
			parentGroupId,
			name,
		})
	}

	async function vaultRenameGroup(
		path: string,
		password: string,
		groupId: string,
		name: string,
	) {
		await invoke('vault_rename_group', {
			path,
			password,
			groupId,
			name,
		})
	}

	async function vaultDeleteGroup(
		path: string,
		password: string,
		groupId: string,
	) {
		await invoke('vault_delete_group', { path, password, groupId })
	}

	async function vaultAddEntry(
		path: string,
		password: string,
		groupId: string,
		fields: Record<string, string>,
	): Promise<VaultEntry> {
		return invoke<VaultEntry>('vault_add_entry', {
			path,
			password,
			groupId,
			fields,
		})
	}

	async function vaultUpdateEntry(
		path: string,
		password: string,
		groupId: string,
		entryId: string,
		fields: Record<string, string>,
	) {
		await invoke('vault_update_entry', {
			path,
			password,
			groupId,
			entryId,
			fields,
		})
	}

	async function vaultDeleteEntry(
		path: string,
		password: string,
		groupId: string,
		entryId: string,
	) {
		await invoke('vault_delete_entry', {
			path,
			password,
			groupId,
			entryId,
		})
	}

	async function solanaPublicKeyFromPrivate(privateKey: string): Promise<string> {
		return invoke<string>('solana_public_key_from_private', { privateKey })
	}

	async function solanaFetchBalance(
		rpcUrl: string,
		address: string,
	): Promise<string> {
		return invoke<string>('solana_fetch_balance', { rpcUrl, address })
	}

	async function solanaTraceFunding(
		rpcUrl: string,
		wallet: string,
	): Promise<string | null> {
		return invoke<string | null>('solana_trace_funding', { rpcUrl, wallet })
	}

	return {
		vaultCreate,
		vaultOpen,
		vaultAddGroup,
		vaultRenameGroup,
		vaultDeleteGroup,
		vaultAddEntry,
		vaultUpdateEntry,
		vaultDeleteEntry,
		solanaPublicKeyFromPrivate,
		solanaFetchBalance,
		solanaTraceFunding,
	}
}
