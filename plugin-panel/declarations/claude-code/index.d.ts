// The parts of Claude Code's plugin API this panel uses, written by hand.
//
// Only what `hooks/register.tsx` and the types it imports use is declared here, shaped as the
// `plugin-authoring` skill's reference describes it. When the panel starts using something
// this file lacks, the same change adds it.

declare module 'claude-code' {
  /** Each plugin's atoms by key; a plugin augments it with its own (`types/index.d.ts`). */
  export interface PluginState {}

  /** A value kept under a shape tag: an atom given `{ shape }` reads and takes `T`. */
  export type Shaped<T> = { readonly shaped: T }
  type ShapedValue<V> = V extends Shaped<infer T> ? T : never

  const atomValue: unique symbol
  /** A named value with its initial, read with `read` and changed with `update`. */
  export type Atom<T> = { readonly [atomValue]: (value: T) => T }

  type StateName<P extends string, K extends string> = { readonly plugin: P; readonly key: K }
  type StateValue<P extends keyof PluginState, K extends keyof PluginState[P]> = PluginState[P][K]

  export const atom: {
    <P extends keyof PluginState & string, K extends keyof PluginState[P] & string>(
      ref: StateName<P, K>,
      initial: StateValue<P, K>,
    ): Atom<StateValue<P, K>>
    <P extends keyof PluginState & string, K extends keyof PluginState[P] & string>(
      ref: StateName<P, K>,
      initial: ShapedValue<StateValue<P, K>>,
      options: { shape: string },
    ): Atom<ShapedValue<StateValue<P, K>>>
  }

  /** The value of an atom, its initial while absent. */
  export function read<T>($: EngineInterface, source: Atom<T>): Promise<T>
  /** Sets an atom to what `change` makes of its value. */
  export function update<T>($: EngineInterface, target: Atom<T>, change: (value: T) => T): Promise<T>

  export type Surface = 'terminal' | 'desktop' | 'vscode' | 'mobile'

  export type RenderElement = { readonly element: unknown }
  export type RenderNode = RenderElement | string
  export type RenderChildren = RenderNode | number | boolean | null | undefined | readonly RenderChildren[]
  type ElementChildren = { children?: RenderChildren }
  export type ElementConstructor<P> = (props: P & ElementChildren) => RenderElement

  export type BoxProps = { key?: string; flexDirection?: 'row' | 'column'; gap?: number }
  export type TextProps = {
    color?: string
    bold?: boolean
    dimColor?: boolean
    wrap?: 'wrap' | 'truncate-end'
  }
  export type ButtonProps = {
    key?: string
    label?: string
    plain?: true
    role?: 'dismiss'
    variant?: 'primary' | 'secondary'
    onPress?: () => void
  }
  export type CodeProps = { source: string }
  export type SvgProps = { source: string; alt: string }

  type Table = {
    Box: ElementConstructor<BoxProps>
    Text: ElementConstructor<TextProps>
    Button: ElementConstructor<ButtonProps>
    Code: ElementConstructor<CodeProps>
  }
  /** Each surface's element table; the terminal's alone has no `Svg`. */
  export interface Elements {
    terminal: Table
    desktop: Table & { Svg: ElementConstructor<SvgProps> }
    vscode: Table & { Svg: ElementConstructor<SvgProps> }
    mobile: Table & { Svg: ElementConstructor<SvgProps> }
  }

  export type Process = { exitCode: number; stdout: string; stderr: string }
  export type Usage = {
    input_tokens: number
    output_tokens: number
    cache_read_input_tokens: number
    cache_creation_input_tokens: number
    model?: string
  }

  export interface EngineInterface {
    plugin: { root: string }
    process: { run(argv: readonly string[], init?: { cwd?: string }): Promise<Process> }
    fs: {
      stat(path: string): Promise<{ kind: 'file' | 'dir' | 'other' }>
      list(path?: string): Promise<readonly { name: string; kind: string }[]>
    }
    session: { cwd(): Promise<string> }
    clock: {
      now(): Promise<number>
      every(ms: number, run: () => void): { cancel(): void }
    }
    agent: { list(): Promise<{ id: string; status: string }[]> }
    ui: {
      panes(): Promise<{ id: string; isPlaced: boolean }[]>
      open(pane: { id: string; title: string }): Promise<unknown>
      resolve<E extends { surface: Surface }>(e: E): Elements[E['surface']]
    }
  }

  type Next<E, R> = (e: E) => Promise<R>
  type Hook<E, R> = ($: EngineInterface, e: E, next: Next<E, R>) => Promise<R>

  /** An agent's `tool.call`: the tool, with the input its task tools take. */
  export type ToolCall = {
    agentId?: string
    tool: string
    todos?: { content?: string; status?: string }[]
    taskId?: string
    subject?: string
    status?: string
  }
  export type ToolResult = { deny?: unknown; isError?: boolean; result?: unknown }
  export type TurnComplete = { agentId?: string; reason: string; usage?: Usage }
  export type TurnStep = { agentId?: string; model: string; effort?: unknown }
  export type StepResult = { usage?: Usage | null }
  export type Spawn = { prompt: string; description: string; subagentType: string; parentAgentId?: string }
  export type Started = { agentId?: string; model?: string; deny?: unknown }

  export interface On {
    (event: 'session.start', hook: Hook<{}, unknown>): void
    (event: 'session.end', hook: Hook<{ reason: string }, unknown>): void
    (event: 'agent.spawn', hook: Hook<Spawn, Started>): void
    (
      event: 'turn.step',
      hook: (
        $: EngineInterface,
        e: TurnStep,
        next: (e: TurnStep) => AsyncGenerator<never, StepResult, undefined>,
      ) => AsyncGenerator<never, StepResult, undefined>,
    ): void
    (event: 'tool.call', hook: Hook<ToolCall, ToolResult>): void
    (
      event: 'command.run',
      match: { command: string },
      hook: ($: EngineInterface, e: { args: string }) => Promise<{ text: string }>,
    ): void
    (
      event: 'ui.render',
      match: { component: 'CommandOutput'; props: { command: string } },
      hook: Hook<{ surface: Surface; props: { isErrored?: boolean; text: string } }, RenderNode>,
    ): void
    (
      event: 'ui.render',
      match: { component: 'Pane'; requestId: string },
      hook: ($: EngineInterface, e: { surface: Surface; props: { bodyColumns: number } }) => Promise<RenderNode>,
    ): void
    (event: 'ui.close', hook: Hook<{ id: string }, unknown>): void
    (event: 'skill.prompt', hook: Hook<{ skill: string; pluginName?: string; text: string }, { text: string }>): void
    (event: 'turn.start', hook: Hook<{ agentId?: string }, unknown>): void
    (event: 'turn.complete', hook: Hook<TurnComplete, unknown>): void
  }

  export type Register = (on: On) => unknown

  global {
    /** The JSX factory and fragment the engine compiles against. */
    const h: (
      tag: string | ((props: never) => RenderNode | null | undefined),
      props: Record<string, unknown> | null | undefined,
      ...children: unknown[]
    ) => RenderNode | null | undefined
    const Fragment: (props: { children?: RenderNode[] }) => RenderElement

    namespace JSX {
      type Element = RenderElement
      type ElementType = (props: never) => RenderNode | null | undefined
      interface IntrinsicElements {}
      interface ElementChildrenAttribute {
        children: unknown
      }
      interface IntrinsicAttributes {
        key?: string
      }
    }
  }
}
