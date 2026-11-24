import { describe, expect, test } from 'vitest'

import { PANE_ID, opensForSkill } from '../src/panel.ts'

describe('build-opens-panel', () => {
  const pane = (id: string) => ({ id, isPlaced: false })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('the clm plugin build skill opens a pane that is not listed, in a CLM repository', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [], inRepo: true })).toBe(true)
    expect(opensForSkill({ skill: 'build', pluginName: 'clm', panes: [], inRepo: true })).toBe(true)
    expect(opensForSkill({ skill: 'build', pluginName: 'clm', panes: [pane('other')], inRepo: true })).toBe(true)
  })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('a listed pane, placed or waiting, is left as it is', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [pane(PANE_ID)], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'clm:build', panes: [{ id: PANE_ID, isPlaced: true }], inRepo: true })).toBe(false)
  })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('outside a CLM repository nothing opens', () => {
    expect(opensForSkill({ skill: 'clm:build', panes: [], inRepo: false })).toBe(false)
  })

  // req:plugin.panel/build-opens-panel@d27a61fc
  test('another skill opens nothing', () => {
    expect(opensForSkill({ skill: 'clm:propose', panes: [], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'build', panes: [], inRepo: true })).toBe(false)
    expect(opensForSkill({ skill: 'build', pluginName: 'other', panes: [], inRepo: true })).toBe(false)
  })
})
