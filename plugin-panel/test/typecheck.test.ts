// The panel's type-check (`type-check`): `npm test` runs `tsc -p .` first, so this file proves
// the configuration it runs under and that the compiler, so configured, reports a type error.
// The files are read through the compiler's own `ts.sys`, since the tests type without Node's.
import ts from 'typescript'
import { describe, expect, test } from 'vitest'

// `npm test` runs in `plugin-panel/`.
const panel = `${ts.sys.getCurrentDirectory()}/`
const read = (path: string): string => {
  const text = ts.sys.readFile(panel + path)
  if (text === undefined) throw new Error(`no ${path}`)
  return text
}

/** `tsconfig.json`'s compiler options, as `tsc -p .` reads them. */
function options(): ts.CompilerOptions {
  const { config, error } = ts.readConfigFile(panel + 'tsconfig.json', ts.sys.readFile)
  if (error) throw new Error(ts.flattenDiagnosticMessageText(error.messageText, '\n'))
  return ts.parseJsonConfigFileContent(config, ts.sys, panel).options
}

describe('type-check', () => {
  // req:plugin.panel/type-check@16075ab7
  test('a type error fails: the compiler, with the tsconfig and its type root, names the file and its line', () => {
    const file = panel + 'src/erring.ts'
    const source = 'export const ok = 1\nexport const bad: Missing = 2\n'
    const opts = options()
    const host = ts.createCompilerHost(opts)
    const getSourceFile = host.getSourceFile
    host.getSourceFile = (name, version, ...rest) =>
      name === file ? ts.createSourceFile(name, source, version) : getSourceFile(name, version, ...rest)
    const fileExists = host.fileExists
    host.fileExists = name => name === file || fileExists(name)
    const program = ts.createProgram([file], opts, host)
    const errors = ts.getPreEmitDiagnostics(program).filter(d => d.file?.fileName === file)
    expect(errors).toHaveLength(1)
    const error = errors[0]!
    expect(ts.flattenDiagnosticMessageText(error.messageText, '\n')).toContain('Missing')
    expect(error.file!.getLineAndCharacterOfPosition(error.start!).line + 1).toBe(2)
    // Nothing else errs: the type root resolves the one type, 'claude-code', to the panel's declarations.
    expect(ts.getPreEmitDiagnostics(program).filter(d => d.file?.fileName !== file)).toEqual([])
  })

  // req:plugin.panel/type-check@16075ab7
  test('the suite type-checks first, under a tsconfig whose one type root is declarations', () => {
    const pkg = JSON.parse(read('package.json')) as {
      scripts: { test: string }
      devDependencies: Record<string, string>
    }
    const script = pkg.scripts.test
    expect(script).toMatch(/tsc -p \. && vitest run/)
    expect(script.indexOf('tsc -p .')).toBeLessThan(script.indexOf('vitest run'))
    expect(pkg.devDependencies).toHaveProperty('typescript')

    const { config } = ts.readConfigFile(panel + 'tsconfig.json', ts.sys.readFile) as {
      config: { include: string[]; compilerOptions: { typeRoots: string[]; types: string[] } }
    }
    expect([...config.include].sort()).toEqual(['hooks', 'src', 'test', 'types'])
    expect(config.compilerOptions.typeRoots).toEqual(['./declarations'])
    expect(config.compilerOptions.types).toEqual(['claude-code'])
    const opts = options()
    expect(opts).toMatchObject({
      target: ts.ScriptTarget.ES2023,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      strict: true,
      noUncheckedIndexedAccess: true,
      noEmit: true,
      skipLibCheck: true,
      jsx: ts.JsxEmit.React,
      jsxFactory: 'h',
      jsxFragmentFactory: 'Fragment',
      allowImportingTsExtensions: true,
    })
    expect(opts.lib).toEqual(['lib.es2023.d.ts'])
  })

  // req:plugin.panel/type-check@16075ab7
  test('the declarations are the panel\'s own, and nothing is vendored', () => {
    const first = read('declarations/claude-code/index.d.ts').split('\n')[0]
    expect(first).toBe("// The parts of Claude Code's plugin API this panel uses, written by hand.")
    expect(ts.sys.directoryExists(panel + 'vendor')).toBe(false)
  })
})
