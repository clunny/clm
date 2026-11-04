import ts from 'typescript'
import { describe, expect, test } from 'vitest'

describe('manifest', () => {
  // req:plugin.panel/manifest-owner@4e7fbb29
  test("the owner's full name and the licence", () => {
    // `npm test` runs in `plugin-panel/`.
    const text = ts.sys.readFile(`${ts.sys.getCurrentDirectory()}/.claude-plugin/plugin.json`)
    const manifest = JSON.parse(text ?? '') as { author: { name: string }; license: string }
    expect(manifest.author.name).toBe('Christopher Lunny')
    expect(manifest.license).toBe('MIT')
  })
})
