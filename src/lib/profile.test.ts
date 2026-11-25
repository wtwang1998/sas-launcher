import { expect, it } from 'vitest'
import type { Project, ProjectProfile } from '../api'
import {
  DEFAULT_LAYERS,
  formatModified,
  profileChoiceFor,
  profileDraftFrom,
  profileReady,
} from './profile'

const stored = (overrides: Partial<ProjectProfile> = {}): ProjectProfile => ({
  origin: { kind: 'sasUser', path: 'C:\\Users\\me\\Documents\\My SAS Files\\9.4' },
  layers: DEFAULT_LAYERS,
  files: 2,
  missing: 0,
  ...overrides,
})

const project = (profile: ProjectProfile | null): Project => ({
  id: 'p1',
  name: 'A',
  note: null,
  config: null,
  profile,
  startupFile: null,
  startupExposesPath: false,
  startupNeedsMigration: false,
})

it('an untouched preference carries the layer the Explorer options live in', () => {
  expect(profileDraftFrom(null).layers).toEqual({
    interface: true,
    environment: true,
    templates: false,
  })
})

it('an unchanged preference is sent as keep so the snapshot survives a rename', () => {
  const current = project(stored())
  const draft = profileDraftFrom(current.profile)
  expect(profileChoiceFor(current, draft)).toEqual({ kind: 'keep' })
})

it('the same folder written with another case or separator is still unchanged', () => {
  const current = project(stored())
  const draft = profileDraftFrom(current.profile)
  if (draft.kind !== 'sasUser') throw new Error('expected a user folder draft')
  const renamed = { ...draft, path: 'c:/users/me/documents/my sas files/9.4/' }
  expect(profileChoiceFor(current, renamed)).toEqual({ kind: 'keep' })
})

it('switching a layer replaces the snapshot', () => {
  const current = project(stored())
  const draft = profileDraftFrom(current.profile)
  const changed = { ...draft, layers: { ...draft.layers, templates: true } }
  expect(profileChoiceFor(current, changed)).toEqual(changed)
})

it('a new project always sends its choice', () => {
  const draft = profileDraftFrom(stored())
  expect(profileChoiceFor(null, draft)).toEqual(draft)
})

it('a project without a stored preference is upgraded on save', () => {
  const draft = profileDraftFrom(null)
  expect(profileChoiceFor(project(null), draft)).toEqual(draft)
})

it('a clean environment stores the interface layer only, whatever the form showed', () => {
  const draft = {
    kind: 'fresh' as const,
    layers: { interface: true, environment: true, templates: true },
  }
  expect(profileChoiceFor(null, draft)).toEqual({ kind: 'fresh', layers: DEFAULT_LAYERS })
})

it('a user folder source needs a path, a snapshot source needs a config', () => {
  expect(profileReady({ kind: 'sasUser', path: '  ', layers: DEFAULT_LAYERS })).toBe(false)
  expect(profileReady({ kind: 'sasUser', path: 'C:\\x', layers: DEFAULT_LAYERS })).toBe(true)
  expect(profileReady({ kind: 'configSnapshot', config: '', layers: DEFAULT_LAYERS })).toBe(false)
  expect(profileReady({ kind: 'configSnapshot', config: 'cfg-1', layers: DEFAULT_LAYERS })).toBe(
    true,
  )
  expect(profileReady({ kind: 'fresh', layers: DEFAULT_LAYERS })).toBe(true)
})

it('an unknown origin falls back to the first card with the stored layers', () => {
  expect(
    profileDraftFrom({
      origin: null,
      layers: { ...DEFAULT_LAYERS, templates: true },
      files: 0,
      missing: 0,
    }),
  ).toEqual({
    kind: 'sasUser',
    path: '',
    layers: { ...DEFAULT_LAYERS, templates: true },
  })
})

it('a missing timestamp renders as nothing rather than 1970', () => {
  expect(formatModified('zh-Hans', 0)).toBe('')
  expect(formatModified('en', 1_700_000_000_000)).not.toBe('')
})
