import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}', 'scripts/**/*.test.mjs'],
    testTimeout: 15_000,
    // Vitest stubs CSS imports, so a test that reads the real stylesheet gets
    // an empty string; process style.css so `?raw` yields its source.
    css: { include: [/style\.css/] },
  },
})
