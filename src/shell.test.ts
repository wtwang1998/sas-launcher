/// <reference types="vite/client" />
import { describe, expect, it } from 'vitest'
import rawConfig from '../src-tauri/tauri.conf.json'
import stylesheet from './style.css?raw'

// jsdom has no layout engine, so it cannot see a scrollbar appear. These tests
// read the files that decide whether Windows ever shows one instead, and lock
// the two decisions that keep them out of sight:
//   1. the window asks WebView2 for the native overlay scrollbars, and
//   2. nothing in the stylesheet opts back into the classic, always-visible
//      ones, and no hidden element can push the document sideways.
// They are regression locks, not layout tests; the layout itself is verified
// on a real window.

type ShellConfig = { app: { windows: { scrollBarStyle?: string }[] } }
const config = rawConfig as ShellConfig

const rule = (selector: string) => {
  const start = stylesheet.indexOf(`${selector} {`)
  expect(start, `stylesheet is missing the ${selector} rule`).toBeGreaterThanOrEqual(0)
  return stylesheet.slice(start, stylesheet.indexOf('}', start))
}

describe('window scrollbars', () => {
  it('asks WebView2 for the native overlay scrollbars', () => {
    expect(config.app.windows[0]?.scrollBarStyle).toBe('fluentOverlay')
  })

  // scrollbar-width/scrollbar-color and the ::-webkit-scrollbar family all
  // force Chromium back to classic bars, which is what the overlay style
  // exists to replace, so the stylesheet must stay out of them.
  it('never styles scrollbars in CSS', () => {
    expect(stylesheet).not.toMatch(/::-webkit-scrollbar/)
    expect(stylesheet).not.toMatch(/scrollbar-width\s*:/)
    expect(stylesheet).not.toMatch(/scrollbar-color\s*:/)
  })

  it('clips sideways overflow at the document root', () => {
    expect(rule('html')).toMatch(/overflow-x:\s*clip/)
  })

  // WebView2 paints the native bars from the page's color scheme, so an
  // undeclared scheme means a dark bar on a light page. The dark theme work
  // owns the value; this only guards that one is declared at all.
  it('declares a color scheme for the native bars to follow', () => {
    expect(rule(':root')).toMatch(/color-scheme:\s*[a-z]/)
  })
})

describe('the hidden ⓘ bubble', () => {
  // The bubble is 320px wide and anchored to an icon that sits at the right
  // edge of a radio row. While it was hidden with `visibility`, it still took
  // part in layout and pushed the document 86px past the viewport, which is
  // what kept both scrollbars on screen.
  it('stays out of layout while hidden and only shows on demand', () => {
    const bubble = rule('.tooltip-bubble')
    expect(bubble).toMatch(/display:\s*none/)
    expect(bubble).not.toMatch(/visibility/)
    expect(stylesheet).toMatch(/\.tooltip:hover \.tooltip-bubble[^{]*\{[^}]*display:\s*block/)
  })

  it('anchors to the right edge so it cannot leave the viewport', () => {
    expect(rule('.tooltip-bubble')).toMatch(/right:\s*0/)
  })

  it('has no retired copy of the same defect', () => {
    expect(stylesheet).not.toMatch(/setup-tip/)
  })
})

// jsdom cannot see colour either, but contrast is arithmetic on the tokens, so
// it can be locked here. These are the pairs the stylesheet paints text with.
const tokens = new Map<string, { light: string; dark: string }>()
for (const [, name, light, dark] of stylesheet.matchAll(
  /--([a-z0-9-]+):\s*light-dark\((#[0-9a-f]{6}),\s*(#[0-9a-f]{6})\)/g,
)) {
  tokens.set(name ?? '', { light: light ?? '', dark: dark ?? '' })
}

const token = (name: string, theme: 'light' | 'dark'): string => {
  const pair = tokens.get(name)
  expect(pair, `--${name} is not a light/dark token pair`).toBeDefined()
  return (theme === 'light' ? pair?.light : pair?.dark) ?? ''
}

const luminance = (hex: string): number => {
  const channel = (index: number) => {
    const value = Number.parseInt(hex.slice(index, index + 2), 16) / 255
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

const contrast = (front: string, back: string): number => {
  const a = luminance(front)
  const b = luminance(back)
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

// 4.5:1 is WCAG AA for text below 24px, or below 18.66px when bold; every pair
// here is body, label or chip text. A pair that cannot hold the ratio has to
// lose its tint, not its place in the list: the point is that someone decided.
const readablePairs: [string, string][] = [
  ['text', 'paper'],
  ['text', 'paper-soft'],
  ['muted', 'paper'],
  ['muted', 'paper-soft'],
  ['nav-text', 'sidebar'],
  ['button-text', 'paper'],
  ['chip-text', 'chip-bg'],
  ['placeholder', 'paper'],
  ['selected-text', 'selected'],
  ['brand-text', 'brand'],
  ['on-accent', 'accent'],
  ['danger-text', 'danger-bg'],
  ['warn-text', 'warn-chip'],
  ['ok-text', 'ok-bg'],
  ['info-text', 'info-bg'],
]

describe('text clears its contrast on the surface it sits on', () => {
  it.each(readablePairs)('%s on %s stays readable in both themes', (front, back) => {
    for (const theme of ['light', 'dark'] as const) {
      expect(
        contrast(token(front, theme), token(back, theme)),
        `--${front} on --${back} (${theme})`,
      ).toBeGreaterThanOrEqual(4.5)
    }
  })
})

// The CSP allows style-src 'self' only, so a style attribute renders in dev and
// silently disappears in the packaged build. No linter in this repo looks for
// it, and the rule only holds while nobody writes one.
describe('the stylesheet stays the only place styles are written', () => {
  it('never sets a style attribute', () => {
    const offenders = Object.entries(
      import.meta.glob<string>('./**/*.tsx', { query: '?raw', eager: true, import: 'default' }),
    )
      .filter(([path]) => !path.endsWith('.test.tsx'))
      .filter(([, source]) => source.includes(' style={'))
      .map(([path]) => path)
    expect(offenders).toEqual([])
  })
})
