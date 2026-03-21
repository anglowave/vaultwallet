import type { MaybeRefOrGetter } from 'vue'
import { onUnmounted, toValue, watch } from 'vue'
import { useEventListener, useThrottleFn } from '@vueuse/core'

const ACTIVITY_EVENTS: (keyof WindowEventMap)[] = [
	'mousedown',
	'mousemove',
	'keydown',
	'scroll',
	'touchstart',
	'wheel',
]

export function useVaultIdleLock(options: {
	enabled: MaybeRefOrGetter<boolean>
	timeoutSeconds: MaybeRefOrGetter<number>
	onIdle: () => void
}) {
	let timer: ReturnType<typeof setTimeout> | null = null

	function clearTimer() {
		if (timer !== null) {
			clearTimeout(timer)
			timer = null
		}
	}

	function schedule() {
		clearTimer()
		if (!import.meta.client) return
		if (!toValue(options.enabled)) return
		const sec = toValue(options.timeoutSeconds)
		if (!sec || sec <= 0) return
		timer = setTimeout(() => {
			timer = null
			options.onIdle()
		}, sec * 1000)
	}

	const bump = useThrottleFn(() => {
		schedule()
	}, 400)

	if (import.meta.client) {
		for (const ev of ACTIVITY_EVENTS) {
			useEventListener(window, ev, bump, { passive: true })
		}
	}

	watch(
		[
			() => toValue(options.enabled),
			() => toValue(options.timeoutSeconds),
		],
		() => {
			clearTimer()
			schedule()
		},
		{ immediate: true },
	)

	onUnmounted(() => {
		clearTimer()
	})
}
