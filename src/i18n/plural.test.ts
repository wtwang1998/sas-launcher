import { expect, it } from 'vitest'
import { pluralKey, translate } from './index'

it('english_gets_a_singular_and_a_plural_form', () => {
  expect(translate('en')(pluralKey('en', 'config.usedByProjects', 1), { count: 1 })).toBe(
    'Used by 1 project',
  )
  expect(translate('en')(pluralKey('en', 'config.usedByProjects', 2), { count: 2 })).toBe(
    'Used by 2 projects',
  )
})

it('chinese_uses_the_same_sentence_for_every_count', () => {
  expect(translate('zh-Hans')(pluralKey('zh-Hans', 'config.usedByProjects', 1), { count: 1 })).toBe(
    '被 1 个项目使用',
  )
  expect(translate('zh-Hant')(pluralKey('zh-Hant', 'config.usedByProjects', 9), { count: 9 })).toBe(
    '被 9 個專案使用',
  )
})

it('a_count_above_one_never_picks_the_singular_form', () => {
  for (const count of [0, 2, 5, 11]) {
    expect(pluralKey('en', 'session.profileSeedValue', count)).toBe(
      'session.profileSeedValue.other',
    )
  }
  expect(pluralKey('en', 'session.profileSeedValue', 1)).toBe('session.profileSeedValue.one')
})
