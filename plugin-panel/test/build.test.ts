import { describe, expect, test } from 'vitest'

import { PANE_ID, opensForSkill } from '../src/panel.ts'

describe('build-opens-panel', () => {
  // req:plugin.panel/build-opens-panel@4b2e0c99
  const pane = (id: string) => ({ id, isPlaced: false })

  test('the clm plugin build skill opens a pane that is not listed, in a CLM repository', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [], inRepo: true })).toBe(true)
    expect(opensForSkill({ skill: 'build', pluginName: 'clm', panes: [], inRepo: true })).toBe(true)
    expect(opensForSkill({ skill: 'build', pluginName: 'clm', panes: [pane('other')], inRepo: true })).toBe(true)
  })

  test('a listed pane, placed or waiting, is left as it is', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [pane(PANE_ID)], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'clm:build', panes: [{ id: PANE_ID, isPlaced: true }], inRepo: true })).toBe(false)
  })

  test('outside a CLM repository nothing opens', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [], inRepo: false })).toBe(false)
  })

  test('another skill opens nothing', () => {
    expect(opensForSkill({ skill: 'clm:propose', panes: [], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'build', panes: [], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'build', pluginName: 'other', panes: [], inRepo: true })).toBe(false)
  })
})
