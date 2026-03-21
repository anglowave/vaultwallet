<script setup lang="ts">
import type { VaultGroup } from '~/types/vault'

const props = defineProps<{
	node: VaultGroup
	selectedGroupId: string | null
	depth: number
	rootId: string
	editingFolderId: string | null
	editingFolderName: string
	loading?: boolean
}>()

const emit = defineEmits<{
	select: [id: string]
	contextRename: [payload: { id: string; name: string }]
	contextDelete: [id: string]
	'update:editingFolderName': [value: string]
	renameCommit: []
	renameCancel: []
}>()

const open = ref(true)

const isRootFolder = computed(() => props.node.id === props.rootId)

const isEditing = computed(
	() =>
		props.editingFolderId !== null && props.editingFolderId === props.node.id,
)

const folderMenuItems = computed(() => [
	[
		{
			label: 'Rename folder',
			icon: 'i-lucide-pencil',
			disabled: isRootFolder.value,
			onSelect: () =>
				emit('contextRename', {
					id: props.node.id,
					name: props.node.name,
				}),
		},
	],
	[
		{
			label: 'Delete folder',
			icon: 'i-lucide-trash-2',
			color: 'error' as const,
			disabled: isRootFolder.value,
			onSelect: () => emit('contextDelete', props.node.id),
		},
	],
])
</script>

<template>
	<div class="select-none">
		<UContextMenu :items="folderMenuItems">
			<div
				class="flex items-center gap-0.5 rounded-md"
				:style="{ paddingLeft: `${Math.min(props.depth, 8) * 10}px` }"
			>
				<UButton
					v-if="node.children.length"
					color="neutral"
					variant="ghost"
					size="xs"
					class="p-1"
					:icon="open ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
					@click.stop="open = !open"
				/>
				<span v-else class="inline-block w-7 shrink-0" />
				<UInput
					v-if="isEditing"
					:model-value="editingFolderName"
					:disabled="loading"
					color="neutral"
					variant="outline"
					size="sm"
					class="min-w-0 flex-1 font-medium"
					autofocus
					@update:model-value="emit('update:editingFolderName', $event)"
					@keydown.enter.prevent="emit('renameCommit')"
					@keydown.esc.prevent="emit('renameCancel')"
					@blur="emit('renameCommit')"
				/>
				<UButton
					v-else
					:color="selectedGroupId === node.id ? 'primary' : 'neutral'"
					variant="ghost"
					size="sm"
					class="min-w-0 flex-1 justify-start gap-2 font-medium"
					@click="emit('select', node.id)"
				>
					<UIcon name="i-lucide-folder" class="size-4 shrink-0 opacity-70" />
					<span class="truncate">{{ node.name || 'Group' }}</span>
				</UButton>
			</div>
		</UContextMenu>
		<div v-show="open && node.children.length">
			<VaultTreeNode
				v-for="c in node.children"
				:key="c.id"
				:node="c"
				:root-id="rootId"
				:selected-group-id="selectedGroupId"
				:depth="depth + 1"
				:editing-folder-id="editingFolderId"
				:editing-folder-name="editingFolderName"
				:loading="loading"
				@select="emit('select', $event)"
				@context-rename="emit('contextRename', $event)"
				@context-delete="emit('contextDelete', $event)"
				@update:editing-folder-name="emit('update:editingFolderName', $event)"
				@rename-commit="emit('renameCommit')"
				@rename-cancel="emit('renameCancel')"
			/>
		</div>
	</div>
</template>
