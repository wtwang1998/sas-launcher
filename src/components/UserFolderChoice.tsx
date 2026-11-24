import { type KeyboardEvent, useId, useRef } from 'react'
import type { Config, Language, SasuserCandidate } from '../api'
import type { Translator } from '../i18n'
import { formatModified, type ProfileDraft, samePath } from '../lib/profile'
import { Tooltip } from './Tooltip'

type Kind = ProfileDraft['kind']

type Card = { kind: Kind; label: string; hint: string; ready: boolean }

/**
 * Three-option radio-card group for where a project's user folder comes from.
 * The layers sit outside the cards: they describe what a snapshot carries, not
 * which source it comes from.
 */
export function UserFolderChoice({
  value,
  candidates,
  configs,
  language,
  t,
  busy = false,
  onBrowse,
  onChange,
}: {
  value: ProfileDraft
  candidates: SasuserCandidate[]
  configs: Config[]
  language: Language
  t: Translator
  busy?: boolean
  onBrowse?: () => void
  onChange: (value: ProfileDraft) => void
}) {
  const hintId = useId()
  const snapshots = configs.flatMap((config) =>
    config.hasProfile && config.reference.kind === 'user'
      ? [{ id: config.reference.id, name: config.name }]
      : [],
  )
  const snapshotConfig = value.kind === 'configSnapshot' ? value.config : ''
  const draftPath = value.kind === 'sasUser' ? value.path : ''
  const chosen = candidates.find((candidate) => samePath(candidate.path, draftPath)) ?? null
  const kind = value.kind
  const refs = useRef<(HTMLDivElement | null)[]>([])
  const cards: Card[] = [
    {
      kind: 'sasUser',
      label: t('inheritUserFolderOption'),
      hint: t('inheritUserFolderHint'),
      ready: candidates.length > 0 || draftPath !== '',
    },
    {
      kind: 'configSnapshot',
      label: t('inheritConfigSnapshotOption'),
      hint: t('inheritConfigSnapshotHint'),
      ready: snapshots.length > 0,
    },
    {
      kind: 'fresh',
      label: t('cleanEnvironmentOption'),
      hint: t('cleanEnvironmentHint'),
      ready: true,
    },
  ]

  function select(next: Kind) {
    if (next === kind) return
    if (next === 'sasUser') {
      onChange({ ...value, kind: 'sasUser', path: candidates[0]?.path ?? '' })
      return
    }
    if (next === 'configSnapshot') {
      onChange({ ...value, kind: 'configSnapshot', config: snapshots[0]?.id ?? '' })
      return
    }
    onChange({ ...value, kind: 'fresh' })
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>, index: number) {
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault()
      select(cards[index].kind)
      return
    }
    const delta =
      event.key === 'ArrowDown' || event.key === 'ArrowRight'
        ? 1
        : event.key === 'ArrowUp' || event.key === 'ArrowLeft'
          ? -1
          : 0
    const target =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? cards.length - 1
          : delta
            ? (index + delta + cards.length) % cards.length
            : -1
    if (target < 0) return
    event.preventDefault()
    if (!cards[target].ready) return
    select(cards[target].kind)
    refs.current[target]?.focus()
  }

  return (
    <>
      <div className="radio-cards" role="radiogroup" aria-label={t('userFolderGroup')}>
        {cards.map((card, index) => {
          const checked = kind === card.kind
          return (
            <div
              key={card.kind}
              role="radio"
              aria-checked={checked}
              aria-disabled={!card.ready || undefined}
              tabIndex={checked ? 0 : -1}
              className={`radio-card${checked ? ' checked' : ''}${card.ready ? '' : ' disabled'}`}
              ref={(element) => {
                refs.current[index] = element
              }}
              onClick={() => {
                if (card.ready && !busy) select(card.kind)
              }}
              onKeyDown={(event) => onKeyDown(event, index)}
            >
              <div className="radio-head">
                <span className="radio-dot" aria-hidden="true" />
                <span className="radio-label">{card.label}</span>
                <Tooltip label={t('helpHint')} text={card.hint} />
              </div>
              {!card.ready && card.kind === 'configSnapshot' && (
                <p className="radio-hint">{t('inheritConfigSnapshotEmpty')}</p>
              )}
              {checked && card.kind === 'sasUser' && (
                <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                  {candidates.length > 0 ? (
                    <select
                      aria-label={t('inheritUserFolderOption')}
                      aria-describedby={hintId}
                      value={chosen?.path ?? candidates[0]?.path ?? ''}
                      disabled={busy}
                      onChange={(event) =>
                        onChange({ ...value, kind: 'sasUser', path: event.target.value })
                      }
                    >
                      {candidates.map((candidate) => (
                        <option key={candidate.path} value={candidate.path}>
                          {candidate.path}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <p className="radio-hint">{t('inheritUserFolderMissing')}</p>
                  )}
                </div>
              )}
              {checked && card.kind === 'sasUser' && chosen && (
                <p className="radio-hint" id={hintId}>
                  {t('inheritUserFolderSummary', {
                    count: chosen.files,
                    when: formatModified(language, chosen.modified),
                  })}
                </p>
              )}
              {checked && card.kind === 'configSnapshot' && (
                <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                  <select
                    aria-label={t('inheritConfigSnapshotOption')}
                    value={
                      snapshots.some((config) => config.id === snapshotConfig) ? snapshotConfig : ''
                    }
                    disabled={busy || snapshots.length === 0}
                    onChange={(event) =>
                      onChange({ ...value, kind: 'configSnapshot', config: event.target.value })
                    }
                  >
                    {snapshots.length === 0 && (
                      <option value="">{t('inheritConfigSnapshotEmpty')}</option>
                    )}
                    {snapshots.map((config) => (
                      <option key={config.id} value={config.id}>
                        {config.name}
                      </option>
                    ))}
                  </select>
                </div>
              )}
              {checked && card.kind !== 'fresh' && (
                <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                  <div className="profile-layers">
                    <p className="profile-layer">{t('profileLayersInterface')}</p>
                    <label className="profile-layer">
                      <input
                        type="checkbox"
                        checked={value.layers.environment}
                        disabled={busy}
                        onChange={(event) =>
                          onChange({
                            ...value,
                            layers: { ...value.layers, environment: event.target.checked },
                          })
                        }
                      />
                      {t('profileLayersEnvironment')}
                    </label>
                    <label className="profile-layer">
                      <input
                        type="checkbox"
                        checked={value.layers.templates}
                        disabled={busy}
                        onChange={(event) =>
                          onChange({
                            ...value,
                            layers: { ...value.layers, templates: event.target.checked },
                          })
                        }
                      />
                      {t('profileLayersTemplates')}
                    </label>
                  </div>
                </div>
              )}
            </div>
          )
        })}
      </div>
      {kind === 'sasUser' && onBrowse && (
        <div className="radio-extra">
          <button type="button" className="link-button" disabled={busy} onClick={onBrowse}>
            {t('browseFolder')}
          </button>
        </div>
      )}
    </>
  )
}
