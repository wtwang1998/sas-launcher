import { ChevronDown } from 'lucide-react'
import { type KeyboardEvent, useCallback, useEffect, useId, useRef, useState } from 'react'
import { type Config, type ConfigRef, type Language, refKey, sameRef } from '../api'
import type { Translator } from '../i18n'
import { configLabel, groupConfigs } from '../lib/config'
import { Chips } from './Chips'

/**
 * Custom listbox for configurations. Keeps the v0.1 keyboard model
 * (combobox trigger + listbox + roving active option) but renders every entry
 * with the same name/chips shape as the configuration list.
 */
export function ConfigurationPicker({
  value,
  configs,
  language,
  t,
  disabled = false,
  ariaLabel,
  onChange,
}: {
  value: ConfigRef | null
  configs: Config[]
  language: Language
  t: Translator
  disabled?: boolean
  ariaLabel: string
  onChange: (config: Config) => void
}) {
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(-1)
  const [up, setUp] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const trigger = useRef<HTMLButtonElement>(null)
  const listId = useId()

  const { users, builtIns } = groupConfigs(configs)
  const options = [...users, ...builtIns]
  const selectedIndex = options.findIndex((config) => sameRef(config.reference, value))
  const selected = selectedIndex >= 0 ? options[selectedIndex] : undefined

  const optionId = useCallback((index: number) => `${listId}-option-${index}`, [listId])

  function openMenu() {
    if (disabled || !options.length) return
    setActive(Math.max(selectedIndex, 0))
    const box = trigger.current?.getBoundingClientRect()
    setUp(!!box && window.innerHeight - box.bottom < 320 && box.top > 320)
    setOpen(true)
  }

  function closeMenu(focusTrigger = false) {
    setOpen(false)
    if (focusTrigger) trigger.current?.focus()
  }

  function choose(index: number) {
    const config = options[index]
    if (!config) return
    onChange(config)
    closeMenu(true)
  }

  useEffect(() => {
    if (!open) return
    const leave = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false)
    }
    window.addEventListener('mousedown', leave)
    return () => window.removeEventListener('mousedown', leave)
  }, [open])

  useEffect(() => {
    if (disabled) setOpen(false)
  }, [disabled])

  useEffect(() => {
    if (!open || active < 0) return
    document.getElementById(optionId(active))?.scrollIntoView({ block: 'nearest' })
  }, [open, active, optionId])

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (!open) {
      if (
        event.key === 'ArrowDown' ||
        event.key === 'ArrowUp' ||
        event.key === 'Enter' ||
        event.key === ' '
      ) {
        // The picker lives inside a radio card; do not let arrows move the radio group.
        event.preventDefault()
        event.stopPropagation()
        openMenu()
      }
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      event.stopPropagation()
      setActive((index) => Math.min(options.length - 1, index + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      event.stopPropagation()
      setActive((index) => Math.max(0, index - 1))
    } else if (event.key === 'Home') {
      event.preventDefault()
      event.stopPropagation()
      setActive(0)
    } else if (event.key === 'End') {
      event.preventDefault()
      event.stopPropagation()
      setActive(options.length - 1)
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      event.stopPropagation()
      choose(active)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      closeMenu(true)
    } else if (event.key === 'Tab') {
      setOpen(false)
    }
  }

  function option(config: Config, index: number) {
    return (
      <div
        key={refKey(config.reference)}
        id={optionId(index)}
        role="option"
        aria-selected={index === selectedIndex}
        className={index === active ? 'config-option active' : 'config-option'}
        onMouseDown={(event) => event.preventDefault()}
        onMouseEnter={() => setActive(index)}
        onClick={() => choose(index)}
      >
        <span className="config-option-name">{configLabel(t, config)}</span>
        <Chips config={config} language={language} t={t} />
      </div>
    )
  }

  return (
    <div className="config-picker" ref={root}>
      <button
        type="button"
        ref={trigger}
        className="config-trigger"
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-label={ariaLabel}
        aria-activedescendant={open && active >= 0 ? optionId(active) : undefined}
        disabled={disabled || !options.length}
        onClick={(event) => {
          event.stopPropagation()
          if (open) closeMenu()
          else openMenu()
        }}
        onKeyDown={onKeyDown}
      >
        {selected ? (
          <>
            <span className="config-trigger-label">{configLabel(t, selected)}</span>
            <Chips config={selected} language={language} t={t} />
          </>
        ) : (
          <span className="config-trigger-placeholder">
            {t('configChoice.configurationPickerEmpty')}
          </span>
        )}
        <ChevronDown
          size={16}
          className={`disclosure${open ? ' expanded' : ''}`}
          aria-hidden="true"
        />
      </button>
      {open && (
        <div
          className={`config-menu${up ? ' up' : ''}`}
          role="listbox"
          id={listId}
          aria-label={ariaLabel}
        >
          {users.length > 0 && (
            <div className="config-menu-group" role="presentation">
              {t('configurations.userConfigurations')}
            </div>
          )}
          {users.map((config, index) => option(config, index))}
          {builtIns.length > 0 && (
            <div className="config-menu-group" role="presentation">
              {t('configurations.builtInConfigurations')}
            </div>
          )}
          {builtIns.map((config, index) => option(config, users.length + index))}
        </div>
      )}
    </div>
  )
}
