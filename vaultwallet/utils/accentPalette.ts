/** Build Nuxt UI primary scale from a single accent hex (sRGB). */
export function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
	const m = /^#?([0-9a-fA-F]{6})$/.exec(hex.trim())
	if (!m) return null
	const n = parseInt(m[1], 16)
	return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 }
}

function rgbToHex({ r, g, b }: { r: number; g: number; b: number }): string {
	const h = (n: number) => n.toString(16).padStart(2, '0')
	return `#${h(r)}${h(g)}${h(b)}`.toUpperCase()
}

function mixRgb(
	a: { r: number; g: number; b: number },
	b: { r: number; g: number; b: number },
	t: number,
): { r: number; g: number; b: number } {
	return {
		r: Math.round(a.r + (b.r - a.r) * t),
		g: Math.round(a.g + (b.g - a.g) * t),
		b: Math.round(a.b + (b.b - a.b) * t),
	}
}

const PRIMARY_STEPS = [
	50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950,
] as const

/** Mix factors from white → accent → black across the scale */
const MIX_T: Record<(typeof PRIMARY_STEPS)[number], number> = {
	50: 0.92,
	100: 0.82,
	200: 0.68,
	300: 0.52,
	400: 0.32,
	500: 0,
	600: 0.18,
	700: 0.38,
	800: 0.58,
	900: 0.72,
	950: 0.84,
}

export function primaryPaletteFromAccent(hex: string): Record<string, string> | null {
	const accent = hexToRgb(hex)
	if (!accent) return null
	const white = { r: 255, g: 255, b: 255 }
	const black = { r: 0, g: 0, b: 0 }
	const out: Record<string, string> = {}
	for (const step of PRIMARY_STEPS) {
		const t = MIX_T[step]
		let rgb: { r: number; g: number; b: number }
		if (step < 500) {
			rgb = mixRgb(white, accent, 1 - t)
		} else if (step === 500) {
			rgb = accent
		} else {
			rgb = mixRgb(accent, black, t)
		}
		out[`--ui-color-primary-${step}`] = rgbToHex(rgb)
	}
	out['--ui-primary'] = out['--ui-color-primary-500']
	return out
}

const ACCENT_STYLE_KEYS = PRIMARY_STEPS.map((s) => `--ui-color-primary-${s}`).concat([
	'--ui-primary',
])

export function clearVaultAccentStyles(root: HTMLElement) {
	for (const k of ACCENT_STYLE_KEYS) {
		root.style.removeProperty(k)
	}
	root.removeAttribute('data-vw-accent-custom')
}

export function applyVaultAccentStyles(root: HTMLElement, hex: string) {
	const vars = primaryPaletteFromAccent(hex)
	if (!vars) {
		clearVaultAccentStyles(root)
		return
	}
	root.dataset.vwAccentCustom = '1'
	for (const [k, v] of Object.entries(vars)) {
		root.style.setProperty(k, v)
	}
}
