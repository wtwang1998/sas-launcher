import { type KeyboardEvent, useId, useRef } from 'react'
import type { Config, SasuserCandidate } from '../api'
import { carriedCount, type ProfileDraft, samePath } from '../lib/profile'
import { formatModified } from '../lib/format'
import { Tooltip } from './Tooltip'
import { pluralKey } from '../i18n'
import { useI18n } from '../i18n/provider'

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
  busy = false,
  onBrowse,
  onChange,
}: {
  value: ProfileDraft
  candidates: SasuserCandidate[]
  configs: Config[]
  busy?: boolean
  onBrowse?: () => void
  onChange: (value: ProfileDraft) => void
}) {
  const { language, t } = useI18n()
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
      label: t('profile.inheritUserFolderOption'),
      hint: t('profile.inheritUserFolderHint'),
      ready: candidates.length > 0 || draftPath !== '' || !!onBrowse,
    },
    {
      kind: 'configSnapshot',
      label: t('profile.inheritConfigSnapshotOption'),
      hint: t('profile.inheritConfigSnapshotHint'),
      ready: snapshots.length > 0,
    },
    {
      kind: 'fresh',
      label: t('profile.noPreferenceOption'),
      hint: t('profile.noPreferenceHint'),
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
      <div className="radio-cards" role="radiogroup" aria-label={t('profile.userFolderGroup')}>
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
                <Tooltip label={t('common.helpHint')} text={card.hint} />
              </div>
              {!card.ready && card.kind === 'configSnapshot' && (
                <p className="radio-hint">{t('profile.inheritConfigSnapshotEmpty')}</p>
              )}
              {checked && card.kind === 'sasUser' && (
                <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                  {candidates.length > 0 ? (
                    <select
                      aria-label={t('profile.inheritUserFolderOption')}
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
                    <p className="radio-hint">{t('profile.inheritUserFolderMissing')}</p>
                  )}
                  {onBrowse && (
                    <div className="profile-browse">
                      <button
                        type="button"
                        className="link-button"
                        disabled={busy}
                        onClick={onBrowse}
                      >
                        {t('dialog.browseFolder')}
                      </button>
                    </div>
                  )}
                </div>
              )}
              {checked && card.kind === 'sasUser' && chosen && (
                <p className="radio-hint" id={hintId}>
                  {carriedCount(chosen.present, value.layers) === 0
                    ? t('profile.inheritUserFolderEmpty')
                    : t(
                        pluralKey(
                          language,
                          'profile.inheritUserFolderSummary',
                          carriedCount(chosen.present, value.layers),
                        ),
                        {
                          count: carriedCount(chosen.present, value.layers),
                          when: formatModified(language, chosen.modified),
                        },
                      )}
                </p>
              )}
              {checked && card.kind === 'configSnapshot' && (
                <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                  <select
                    aria-label={t('profile.inheritConfigSnapshotOption')}
                    value={
                      snapshots.some((config) => config.id === snapshotConfig) ? snapshotConfig : ''
                    }
                    disabled={busy || snapshots.length === 0}
                    onChange={(event) =>
                      onChange({ ...value, kind: 'configSnapshot', config: event.target.value })
                    }
                  >
                    {snapshots.length === 0 && (
                      <option value="">{t('profile.inheritConfigSnapshotEmpty')}</option>
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
                    <p className="profile-layer">{t('profile.profileLayersCarryOver')}</p>
                    <p className="profile-layer profile-also">{t('profile.profileLayersAlso')}</p>
                    <div className="profile-layer">
                      <label className="profile-option">
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
                        {t('profile.profileLayersEnvironment')}
                      </label>
                      <Tooltip
                        label={t('common.helpHint')}
                        text={t('profile.profileLayersEnvironmentHint')}
                      />
                    </div>
                    <label className="profile-option">
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
                      {t('profile.profileLayersTemplates')}
                    </label>
                  </div>
                </div>
              )}
            </div>
          )
        })}
      </div>
    </>
  )
}
