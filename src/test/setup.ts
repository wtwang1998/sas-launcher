import '@testing-library/jest-dom/vitest'
import { clearMocks } from '@tauri-apps/api/mocks'
import { cleanup, configure } from '@testing-library/react'
import { afterEach } from 'vitest'

// The default 1 s wait budget is too tight for a slower CI runner; the App
// polls every 2.8 s, so a single slow step can starve a findBy* query.
configure({ asyncUtilTimeout: 3_000 })

afterEach(() => {
  cleanup()
  clearMocks()
})
