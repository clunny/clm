// A GitHub of this package's own, for the Worker's outbound requests. It is backed by a real
// bare git repository, so what the Worker reads is what `git` and `clm` wrote there, and what
// the Worker writes (blobs, trees, commits and refs, through the Git Data API) is a git
// object and a ref update that `clm` reads back. A request it does not answer fails the
// test that caused it, naming the method and path.
import { execFileSync } from 'node:child_process'

const GIT_ENV = {
  ...process.env,
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
  GIT_AUTHOR_NAME: 'GitHub',
  GIT_AUTHOR_EMAIL: 'noreply@github.com',
  GIT_COMMITTER_NAME: 'GitHub',
  GIT_COMMITTER_EMAIL: 'noreply@github.com',
}

const git = (dir: string, args: string[], input?: string | Buffer): Buffer =>
  execFileSync('git', args, { cwd: dir, env: GIT_ENV, input, maxBuffer: 1 << 28 })

const text = (dir: string, args: string[], input?: string | Buffer): string => git(dir, args, input).toString('utf8').trim()

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } })

const INSTALLATION_TOKEN = 'ghs_installation'

export class FakeGitHub {
  /** Requests nothing answered, as `METHOD path`. */
  readonly unanswered: string[] = []
  /** Every request answered, as `METHOD path`. */
  readonly answered: string[] = []
  private readonly token = `ghu_${Math.random().toString(16).slice(2)}`

  constructor(
    readonly owner: string,
    readonly repo: string,
    /** The bare repository holding the fake's state. */
    readonly dir: string,
    /** The login `GET /user` names. */
    readonly login: string,
  ) {}

  async handle(request: Request): Promise<Response> {
    const url = new URL(request.url)
    const key = `${request.method} ${url.pathname}${url.search}`
    try {
      const res = await this.route(request, url)
      if (res) {
        this.answered.push(key)
        return res
      }
    } catch (e) {
      this.unanswered.push(`${key} (${e})`)
      return json(500, { message: String(e) })
    }
    this.unanswered.push(key)
    return json(501, { message: `the fake GitHub does not answer ${key}` })
  }

  private async route(request: Request, url: URL): Promise<Response | null> {
    const { method } = request
    const path = url.pathname
    if (url.host === 'github.com') {
      if (method === 'POST' && path === '/login/oauth/access_token') {
        return json(200, { access_token: this.token, token_type: 'bearer' })
      }
      return null
    }
    if (url.host !== 'api.github.com') return null
    const auth = request.headers.get('authorization') ?? ''
    // The signed-in person's token, the App's JWT, or an installation token it exchanged for.
    if (!auth.endsWith(this.token) && !auth.startsWith('Bearer ey') && !auth.endsWith(INSTALLATION_TOKEN)) {
      return json(401, { message: 'Bad credentials' })
    }
    if (method === 'POST' && path === '/app/installations/1/access_tokens') {
      return json(201, { token: INSTALLATION_TOKEN, expires_at: new Date(Date.now() + 3_600_000).toISOString().replace(/\.\d+Z$/, 'Z') })
    }
    if (method === 'GET' && path === '/user') return json(200, { login: this.login })
    // The App's installation and the repository it covers: the one this fake serves.
    if (method === 'GET' && path === '/user/installations') return json(200, { installations: [{ id: 1 }] })
    if (method === 'GET' && path === '/user/installations/1/repositories') {
      const repositories = [{ owner: { login: this.owner }, name: this.repo, default_branch: 'main' }]
      return json(200, { total_count: 1, repositories })
    }
    if (method === 'POST' && path === '/graphql') return this.graphql(await request.json())
    const base = `/repos/${this.owner}/${this.repo}`
    const lower = path.toLowerCase()
    if (lower !== base.toLowerCase() && !lower.startsWith(`${base.toLowerCase()}/`)) return null
    const rest = path.slice(base.length)
    const decoded = decodeURIComponent(rest)
    if (method === 'GET') return this.read(decoded, url)
    const body = (await request.json()) as Record<string, any>
    return this.write(method, decoded, body)
  }

  private read(rest: string, url: URL): Response | null {
    const d = this.dir
    if (rest === '/installation') return json(200, { id: 1 })
    if (rest === '') return json(200, { full_name: `${this.owner}/${this.repo}`, default_branch: 'main' })
    const ref = /^\/git\/ref\/heads\/(.+)$/.exec(rest)
    if (ref) {
      const sha = this.sha(`refs/heads/${ref[1]}`)
      return sha ? json(200, { ref: `refs/heads/${ref[1]}`, object: { sha, type: 'commit' } }) : json(404, { message: 'Not Found' })
    }
    const matching = /^\/git\/matching-refs\/heads\/(.+)$/.exec(rest)
    if (matching) {
      const out = text(d, ['for-each-ref', '--format=%(refname) %(objectname)', `refs/heads/${matching[1]}`])
      const refs = out
        .split('\n')
        .filter(Boolean)
        .map((l) => l.split(' '))
        .map(([name, sha]) => ({ ref: name, object: { sha, type: 'commit' } }))
      return json(200, refs)
    }
    const compare = /^\/compare\/([0-9a-f]+)\.\.\.([0-9a-f]+)$/.exec(rest)
    if (compare) {
      const mergeBase = text(d, ['merge-base', compare[1], compare[2]])
      return json(200, { merge_base_commit: { sha: mergeBase } })
    }
    const tree = /^\/git\/trees\/([0-9a-f]+)$/.exec(rest)
    if (tree && url.searchParams.get('recursive') === '1') {
      const out = git(d, ['ls-tree', '-r', '-t', '-l', '-z', tree[1]]).toString('utf8')
      const entries = out
        .split('\0')
        .filter(Boolean)
        .map((line) => {
          const [meta, path] = line.split('\t')
          const [mode, type, sha, size] = meta.trim().split(/\s+/)
          return { path, mode, type, sha, ...(type === 'blob' ? { size: Number(size) } : {}) }
        })
      return json(200, { sha: tree[1], tree: entries, truncated: false })
    }
    const blob = /^\/git\/blobs\/([0-9a-f]+)$/.exec(rest)
    if (blob) {
      const content = git(d, ['cat-file', 'blob', blob[1]]).toString('base64')
      return json(200, { sha: blob[1], content, encoding: 'base64' })
    }
    // No open pull requests, and no events: a repository with no host's pages.
    if (rest === '/pulls' || rest === '/events') return json(200, [])
    return null
  }

  private write(method: string, rest: string, body: Record<string, any>): Response | null {
    const d = this.dir
    if (method === 'POST' && rest === '/git/blobs') {
      const data = body.encoding === 'base64' ? Buffer.from(body.content, 'base64') : Buffer.from(body.content, 'utf8')
      return json(201, { sha: text(d, ['hash-object', '-w', '--stdin'], data) })
    }
    if (method === 'POST' && rest === '/git/trees') {
      const lines = (body.tree as any[]).map((e) => `${e.mode} ${e.type} ${e.sha}\t${e.path}\n`).join('')
      return json(201, { sha: text(d, ['mktree'], lines) })
    }
    if (method === 'POST' && rest === '/git/commits') {
      const args = ['commit-tree', body.tree, '-m', body.message]
      for (const p of body.parents as string[]) args.push('-p', p)
      return json(201, { sha: text(d, args) })
    }
    if (method === 'POST' && rest === '/git/refs') {
      if (this.sha(body.ref)) return json(422, { message: 'Reference already exists' })
      git(d, ['update-ref', body.ref, body.sha])
      return json(201, { ref: body.ref, object: { sha: body.sha } })
    }
    const ref = /^\/git\/refs\/heads\/(.+)$/.exec(rest)
    if (method === 'PATCH' && ref) {
      const name = `refs/heads/${ref[1]}`
      const old = this.sha(name)
      if (!old) return json(422, { message: 'Reference does not exist' })
      // Only forward, as GitHub does without `force`.
      if (!body.force && !this.ancestor(old, body.sha)) return json(422, { message: 'Update is not a fast forward' })
      git(d, ['update-ref', name, body.sha])
      return json(200, { ref: name, object: { sha: body.sha } })
    }
    return null
  }

  private sha(ref: string): string | null {
    try {
      return text(this.dir, ['rev-parse', '--verify', '-q', ref])
    } catch {
      return null
    }
  }

  private ancestor(old: string, next: string): boolean {
    try {
      git(this.dir, ['merge-base', '--is-ancestor', old, next])
      return true
    } catch {
      return false
    }
  }

  /** The queries the Worker makes: files by `<rev>:<path>` expression, and refs and pulls. */
  private graphql(request: { query: string; variables: Record<string, any> }): Response {
    const { query, variables } = request
    const object = (expression: string) => {
      try {
        const sha = text(this.dir, ['rev-parse', '--verify', '-q', expression])
        const bytes = git(this.dir, ['cat-file', 'blob', sha])
        return { __typename: 'Blob', oid: sha, text: bytes.toString('utf8'), isBinary: bytes.includes(0), isTruncated: false }
      } catch {
        return null
      }
    }
    if (query.startsWith('query Specs(')) {
      // Which repositories hold `specs/SPEC.md` at `HEAD`: the one this fake serves does.
      const found = object('HEAD:specs/SPEC.md')
      const data: Record<string, unknown> = {}
      for (const name of Object.keys(variables)) {
        if (/^o\d+$/.test(name)) data[`r${name.slice(1)}`] = { object: found && { __typename: found.__typename } }
      }
      return json(200, { data })
    }
    let repository: Record<string, unknown>
    if (query.includes('$expression')) {
      const found = object(variables.expression)
      repository = { object: found && { __typename: found.__typename } }
    } else if (query.includes('refs(refPrefix')) {
      repository = { refs: { nodes: [], pageInfo: { hasNextPage: false, endCursor: null } } }
    } else if (query.includes('pullRequests(')) {
      repository = { pullRequests: { nodes: [] } }
    } else {
      repository = {}
      for (const name of Object.keys(variables)) {
        if (/^e\d+$/.test(name)) repository[`f${name.slice(1)}`] = object(variables[name])
      }
    }
    return json(200, { data: { repository } })
  }
}
