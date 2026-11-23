import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { expect, it } from 'vitest'
import type { ConfigRef } from '../api'
import { translate } from '../i18n'
import { builtIn, userConfig } from '../test/fixtures'
import { ConfigChoice } from './ConfigChoice'

it('arrow_keys_move_selection_and_subcontrol_only_renders_when_checked', async () => {
  const user = userEvent.setup()
  const configs = [
    builtIn('u8', 'UTF-8', 'zh_CN'),
    userConfig('cfg-1', '我的配置', 'EUC-CN', 'zh_CN'),
  ]
  const t = translate('zh-Hans')

  function Wrapper() {
    const [value, setValue] = useState<ConfigRef | null>(null)
    return (
      <ConfigChoice
        value={value}
        configs={configs}
        defaultConfig={builtIn('zh', 'EUC-CN', 'zh_CN')}
        language="zh-Hans"
        t={t}
        onChange={setValue}
      />
    )
  }

  render(<Wrapper />)
  const radios = screen.getAllByRole('radio')
  expect(radios).toHaveLength(3)
  expect(radios[0]).toHaveAttribute('aria-checked', 'true')
  expect(within(radios[0]).queryByRole('combobox')).not.toBeInTheDocument()

  radios[0].focus()
  await user.keyboard('{ArrowDown}')
  expect(radios[0]).toHaveAttribute('aria-checked', 'false')
  expect(radios[1]).toHaveAttribute('aria-checked', 'true')
  expect(within(radios[1]).getByRole('combobox')).toBeInTheDocument()
  expect(within(radios[0]).queryByRole('combobox')).not.toBeInTheDocument()

  await user.keyboard('{ArrowDown}')
  expect(radios[2]).toHaveAttribute('aria-checked', 'true')
  expect(within(radios[2]).getByRole('combobox')).toBeInTheDocument()
})
