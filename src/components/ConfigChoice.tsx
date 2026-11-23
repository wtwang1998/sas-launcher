import { useRef, type KeyboardEvent } from 'react'
import type { Config, ConfigRef, Language } from '../api'
import type { Translator } from '../i18n'
import { groupConfigs, optionSummary } from '../lib/config'
import { ConfigurationPicker } from './ConfigurationPicker'
import { Tooltip } from './Tooltip'

export type ConfigChoiceValue = ConfigRef | null
type Kind = 'default' | 'builtIn' | 'user'

type Card = { kind: Kind; label: string; hint: string; ready: boolean }

/**
 * Three-option radio-card group for the project configuration pointer:
 * follow-the-default, a built-in configuration, or one of the user configurations.
 * Only the checked card renders its sub-control.
 */
export function ConfigChoice({
  value,
  configs,
  defaultConfig,
  language,
  t,
  busy = false,
  onCreateConfiguration,
  onChange,
}: {
  value: ConfigChoiceValue
  configs: Config[]
  defaultConfig: Config | null
  language: Language
  t: Translator
  busy?: boolean
  onCreateConfiguration?: () => void
  onChange: (value: ConfigChoiceValue) => void
}) {
  const { users, builtIns } = groupConfigs(configs)
  const kind: Kind = value === null ? 'default' : value.kind
  const refs = useRef<(HTMLDivElement | null)[]>([])
  const summary = optionSummary(language, defaultConfig)
  const cards: Card[] = [
    {
      kind: 'default',
      label: t('followDefaultOption'),
      hint: summary
        ? t('followDefaultOptionHintWith', { options: summary })
        : t('followDefaultOptionHint'),
      ready: true,
    },
    {
      kind: 'builtIn',
      label: t('builtInOption'),
      hint: t('builtInOptionHint'),
      ready: builtIns.length > 0,
    },
    {
      kind: 'user',
      label: t('userOption'),
      hint: t('userOptionHint'),
      ready: users.length > 0 || !!onCreateConfiguration,
    },
  ]

  function select(next: Kind) {
    if (next === kind) return
    if (next === 'default') {
      onChange(null)
      return
    }
    if (next === 'builtIn') {
      const first = builtIns[0]
      if (first?.reference.kind === 'builtIn')
        onChange({ kind: 'builtIn', code: first.reference.code })
      return
    }
    const first = users[0]
    onChange(
      first?.reference.kind === 'user'
        ? { kind: 'user', id: first.reference.id }
        : { kind: 'user', id: '' },
    )
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
    <div className="radio-cards" role="radiogroup" aria-label={t('configSelection')}>
      {cards.map((card, index) => {
        const checked = kind === card.kind
        return (
          <div
            key={card.kind}
            role="radio"
            aria-checked={checked}
            aria-disabled={!card.ready || undefined}
            tabIndex={checked ? 0 : -1}
            className={'radio-card' + (checked ? ' checked' : '') + (card.ready ? '' : ' disabled')}
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
            {checked && card.kind === 'default' && (
              <p className="radio-hint">{defaultConfig ? card.hint : t('followDefaultMissing')}</p>
            )}
            {checked && card.kind === 'builtIn' && (
              <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                <ConfigurationPicker
                  value={value}
                  configs={builtIns}
                  language={language}
                  t={t}
                  disabled={busy}
                  ariaLabel={t('builtInOption')}
                  onChange={(config) => {
                    if (config.reference.kind === 'builtIn')
                      onChange({ kind: 'builtIn', code: config.reference.code })
                  }}
                />
              </div>
            )}
            {checked && card.kind === 'user' && (
              <div className="radio-control" onClick={(event) => event.stopPropagation()}>
                <ConfigurationPicker
                  value={value}
                  configs={users}
                  language={language}
                  t={t}
                  disabled={busy}
                  ariaLabel={t('userOption')}
                  onChange={(config) => {
                    if (config.reference.kind === 'user')
                      onChange({ kind: 'user', id: config.reference.id })
                  }}
                />
              </div>
            )}
            {card.kind === 'user' && onCreateConfiguration && (
              <div className="radio-extra" onClick={(event) => event.stopPropagation()}>
                <button
                  type="button"
                  className="link-button"
                  disabled={busy}
                  onClick={() => onCreateConfiguration()}
                >
                  {t('createConfiguration')}
                </button>
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
