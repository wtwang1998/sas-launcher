import type {
  Language,
  ProfileChoice,
  ProfileLayers,
  ProfileOrigin,
  Project,
  ProjectProfile,
} from '../api'

/** The interface layer alone: the only one that is safe to carry by default. */
export const DEFAULT_LAYERS: ProfileLayers = {
  interface: true,
  environment: false,
  templates: false,
}

/** What the new-project form holds. Always a concrete choice, never "keep". */
export type ProfileDraft =
  | { kind: 'sasUser'; path: string; layers: ProfileLayers }
  | { kind: 'configSnapshot'; config: string; layers: ProfileLayers }
  | { kind: 'fresh'; layers: ProfileLayers }

export const emptyProfileDraft: ProfileDraft = {
  kind: 'sasUser',
  path: '',
  layers: DEFAULT_LAYERS,
}

/** The draft that reproduces what a project stored, so an edit starts in place. */
export function profileDraftFrom(profile: ProjectProfile | null): ProfileDraft {
  const layers = profile?.layers ?? DEFAULT_LAYERS
  const origin = profile?.origin
  if (origin?.kind === 'sasUser') return { kind: 'sasUser', path: origin.path, layers }
  if (origin?.kind === 'configSnapshot')
    return { kind: 'configSnapshot', config: origin.config, layers }
  if (origin?.kind === 'fresh') return { kind: 'fresh', layers }
  return { ...emptyProfileDraft, layers }
}

/**
 * An untouched preference must reach the backend as `keep`: re-sending the same
 * source would capture a new snapshot and silently overwrite the project's one.
 */
export function profileChoiceFor(project: Project | null, draft: ProfileDraft): ProfileChoice {
  const stored = project?.profile ?? null
  if (stored && sameOrigin(stored.origin, draft) && sameLayers(stored.layers, draft.layers)) {
    return { kind: 'keep' }
  }
  return draft
}

/** The submit button waits for a source that can actually be copied. */
export function profileReady(draft: ProfileDraft): boolean {
  if (draft.kind === 'sasUser') return draft.path.trim() !== ''
  if (draft.kind === 'configSnapshot') return draft.config.trim() !== ''
  return true
}

/** `将继承 4 个文件 … 修改于 …` needs the same wording on every platform. */
export function formatModified(language: Language, millis: number): string {
  if (!millis) return ''
  return new Date(millis).toLocaleString(language === 'zh-Hans' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function sameOrigin(origin: ProfileOrigin | null, draft: ProfileDraft): boolean {
  if (!origin || origin.kind !== draft.kind) return false
  if (origin.kind === 'sasUser' && draft.kind === 'sasUser') {
    return samePath(origin.path, draft.path)
  }
  if (origin.kind === 'configSnapshot' && draft.kind === 'configSnapshot') {
    return origin.config === draft.config
  }
  return true
}

/** SAS writes the same folder with either separator and in any case. */
export function samePath(left: string, right: string): boolean {
  const normalize = (value: string) =>
    value
      .replace(/[\\/]+/g, '\\')
      .replace(/\\+$/, '')
      .toLowerCase()
  return normalize(left) === normalize(right)
}

function sameLayers(left: ProfileLayers, right: ProfileLayers): boolean {
  return (
    left.interface === right.interface &&
    left.environment === right.environment &&
    left.templates === right.templates
  )
}
