<script setup lang="ts">
/**
 * Frameless Tauri title bar: drag region + window controls on the trailing edge.
 */
const isTauri = computed(
	() => import.meta.client && '__TAURI_INTERNALS__' in window,
)

async function getWin() {
	const { getCurrentWindow } = await import('@tauri-apps/api/window')
	return getCurrentWindow()
}

async function onClose() {
	if (!isTauri.value) return
	try {
		await (await getWin()).close()
	} catch {
		/* dev server */
	}
}

async function onMinimize() {
	if (!isTauri.value) return
	try {
		await (await getWin()).minimize()
	} catch {
		/* dev server */
	}
}

async function onZoom() {
	if (!isTauri.value) return
	try {
		await (await getWin()).toggleMaximize()
	} catch {
		/* dev server */
	}
}

async function onTitleBarDblClick() {
	if (!isTauri.value) return
	try {
		await (await getWin()).toggleMaximize()
	} catch {
		/* dev server */
	}
}
</script>

<template>
	<header
		v-if="isTauri"
		class="titlebar border-default bg-elevated/90 flex h-11 shrink-0 cursor-default items-stretch border-b backdrop-blur-md"
	>
		<div
			data-tauri-drag-region
			class="titlebar-drag text-muted flex min-w-0 flex-1 items-center justify-start gap-2 py-2 pl-3 text-sm font-medium select-none"
			@dblclick="onTitleBarDblClick"
		>
			<UIcon name="i-lucide-shield" class="text-primary size-4 shrink-0 opacity-90" />
			<span class="text-highlighted truncate">VaultWallet</span>
		</div>

		<div
			class="traffic-lights flex items-center gap-3 pl-3 pr-6"
			role="toolbar"
			aria-label="Window"
		>
			<button
				type="button"
				class="traffic traffic--minimize"
				aria-label="Minimize window"
				@mousedown.stop
				@click="onMinimize"
			>
				<svg class="traffic__glyph" viewBox="0 0 10 10" aria-hidden="true">
					<path
						d="M2 5 L8 5"
						fill="none"
						stroke="currentColor"
						stroke-width="1.2"
						stroke-linecap="round"
					/>
				</svg>
			</button>
			<button
				type="button"
				class="traffic traffic--zoom"
				aria-label="Zoom window"
				@mousedown.stop
				@click="onZoom"
			>
				<svg class="traffic__glyph traffic__glyph--sm" viewBox="0 0 10 10" aria-hidden="true">
					<path
						d="M3 6 L6 3 M3 3 L6 6"
						fill="none"
						stroke="currentColor"
						stroke-width="1.1"
						stroke-linecap="round"
					/>
				</svg>
			</button>
			<button
				type="button"
				class="traffic traffic--close"
				aria-label="Close window"
				@mousedown.stop
				@click="onClose"
			>
				<svg class="traffic__glyph" viewBox="0 0 10 10" aria-hidden="true">
					<path
						d="M2.5 2.5 L7.5 7.5 M7.5 2.5 L2.5 7.5"
						fill="none"
						stroke="currentColor"
						stroke-width="1.2"
						stroke-linecap="round"
					/>
				</svg>
			</button>
		</div>
	</header>
</template>

<style scoped>
.traffic {
	display: flex;
	width: 12px;
	height: 12px;
	align-items: center;
	justify-content: center;
	border: none;
	border-radius: 9999px;
	padding: 0;
	cursor: default;
	transition:
		box-shadow 0.12s ease,
		filter 0.12s ease,
		transform 0.08s ease;
}

.traffic:active {
	transform: scale(0.92);
}

.traffic__glyph {
	display: none;
	width: 8px;
	height: 8px;
	color: rgba(0, 0, 0, 0.45);
}

.traffic__glyph--sm {
	width: 7px;
	height: 7px;
}

.traffic:hover .traffic__glyph,
.traffic:focus-visible .traffic__glyph {
	display: block;
}

.traffic:focus-visible {
	outline: 2px solid var(--ui-color-primary-500, #396cd8);
	outline-offset: 2px;
}

.traffic--close {
	background: #ff5f57;
	box-shadow:
		0 0 0 0.5px rgba(0, 0, 0, 0.15) inset,
		0 1px 2px rgba(0, 0, 0, 0.12);
}

.traffic--close:hover {
	filter: brightness(1.06);
}

.traffic--minimize {
	background: #febc2e;
	box-shadow:
		0 0 0 0.5px rgba(0, 0, 0, 0.12) inset,
		0 1px 2px rgba(0, 0, 0, 0.1);
}

.traffic--minimize:hover {
	filter: brightness(1.06);
}

.traffic--zoom {
	background: #28c840;
	box-shadow:
		0 0 0 0.5px rgba(0, 0, 0, 0.12) inset,
		0 1px 2px rgba(0, 0, 0, 0.1);
}

.traffic--zoom:hover {
	filter: brightness(1.06);
}

@media (prefers-color-scheme: dark) {
	.traffic__glyph {
		color: rgba(0, 0, 0, 0.55);
	}
}
</style>
