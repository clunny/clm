// Self-contained: Claude Code refuses a contract that imports. `src/agents.ts` takes the
// run types from here.

export type TaskStatus = 'pending' | 'in_progress' | 'completed'
export type Task = { id: string; content: string; status: TaskStatus }
export type RunStatus = 'running' | 'waiting' | 'done' | 'failed'

/** One subagent this session launched. */
export type AgentRun = {
  /** The started `agentId`. */
  id: string
  /** The `agentId` of the subagent that spawned it; absent when the main loop did. */
  parentId?: string
  type: string
  description: string
  model: string
  effort?: string
  /** The node it builds, from the first five lines of its prompt. */
  node?: string
  status: RunStatus
  /** A failed run whose `Dismiss` was pressed: no longer drawn, still counted. */
  dismissed?: boolean
  startedAt: number
  endedAt?: number
  round: number
  contextTokens: number
  tokens: number
  costUsd: number
  steps: number
  tasks: Task[]
}

export type PanelTab = 'spec' | 'flight' | 'blockers'

declare module 'claude-code' {
  interface PluginState {
    'clm-panel': {
      tab: Shaped<PanelTab>
      /** The node the inspector shows, by ID. */
      selected: Shaped<string | null>
      /** The Blockers tab's roots whose chains show, by name. */
      unfolded: Shaped<string[]>
      /** Whether the inspector shows the selected node's contract. */
      contract: Shaped<boolean>
      generation: number
      agents: AgentRun[]
      now: number
    }
  }
}
