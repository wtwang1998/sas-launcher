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
