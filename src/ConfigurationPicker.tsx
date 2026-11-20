import { ChevronRight } from 'lucide-react'
import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react'
import type { Config, Language } from './api'
import { configurationLabel } from './config-options'
import { valueFor } from './configuration-selection'
import type { Translator } from './i18n'

export function ConfigurationPicker({ value, configs, language, t, disabled = false, ariaLabel, onChange }: {
  value: string
  configs: Config[]
  language: Language
  t: Translator
  disabled?: boolean
  ariaLabel: string
  onChange: (value: string) => void
}) {
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(-1)
  const [up, setUp] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const trigger = useRef<HTMLButtonElement>(null)
  const listId = useId()

  const custom = configs.filter(config => !!config.customId)
  const builtIn = configs.filter(config => !config.customId)
  const options = [...custom, ...builtIn]
  const selectedIndex = options.findIndex(config => valueFor(config) === value)
  const selected = selectedIndex >= 0 ? options[selectedIndex] : undefined

  const optionId = (index: number) => listId + '-option-' + index

  function openMenu() {
    if (disabled || !options.length) return
    setActive(selectedIndex >= 0 ? selectedIndex : 0)
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
    onChange(valueFor(config))
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
  }, [open, active])

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (!open) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault()
        openMenu()
      }
      return
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive(index => Math.min(options.length - 1, index + 1))
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive(index => Math.max(0, index - 1))
    } else if (event.key === 'Home') {
      event.preventDefault()
      setActive(0)
    } else if (event.key === 'End') {
      event.preventDefault()
      setActive(options.length - 1)
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      choose(active)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      closeMenu(true)
    } else if (event.key === 'Tab') {
      setOpen(false)
    }
  }

  function option(config: Config, index: number) {
    return <div key={valueFor(config)} id={optionId(index)} role="option" aria-selected={index === selectedIndex}
      className={index === active ? 'config-option active' : 'config-option'}
      onMouseDown={event => event.preventDefault()}
      onMouseEnter={() => setActive(index)}
      onClick={() => choose(index)}>
      <span className="config-option-name">{configurationLabel(t, config, configs, language)}</span>
      {config.isDefault && <span className="chip">{t('defaultOption')}</span>}
    </div>
  }

  return <div className="config-picker" ref={root}>
    <button type="button" ref={trigger} className="config-trigger" role="combobox" aria-haspopup="listbox"
      aria-expanded={open} aria-controls={open ? listId : undefined} aria-label={ariaLabel}
      aria-activedescendant={open && active >= 0 ? optionId(active) : undefined}
      disabled={disabled || !options.length} onClick={() => { if (open) closeMenu(); else openMenu() }} onKeyDown={onKeyDown}>
      {selected
        ? <span className="config-trigger-label">{configurationLabel(t, selected, configs, language)}</span>
        : <span className="config-trigger-placeholder">{t('chooseConfig')}</span>}
      {selected?.isDefault && <span className="chip">{t('defaultOption')}</span>}
      <ChevronRight size={16} className={'disclosure' + (open ? ' expanded' : '')} aria-hidden="true" />
    </button>
    {open && <div className={'config-menu' + (up ? ' up' : '')} role="listbox" id={listId} aria-label={ariaLabel}>
      {custom.length > 0 && <div className="config-menu-group" role="presentation">{t('customConfigurations')}</div>}
      {custom.map((config, index) => option(config, index))}
      {builtIn.length > 0 && <div className="config-menu-group" role="presentation">{t('builtInConfigurations')}</div>}
      {builtIn.map((config, index) => option(config, custom.length + index))}
    </div>}
  </div>
}
