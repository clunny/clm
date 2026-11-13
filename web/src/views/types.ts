// The Worker's response shapes (crates/clm-web/src/{repos,snapshot,branches,review,implementation,taken,live,capabilities}.rs)
// and the `view` and `effort` nodes' models it carries (crates/clm-view/src/{spec,flight,blockers}.rs, crates/clm-effort/src/report.rs).

export interface Repo {
  owner: string;
  name: string;
  default_branch: string;
}

export type Coverage = { state: 'complete' } | { state: 'indexing'; done: number };

export interface RequirementView {
  id: string;
  text: string;
  /** Its current version: the first 8 hex digits of its hash (`versioned-markers`). */
  version: string;
  /** `current`, `behind` or `unbuilt`, as `clm coverage` reports; null while the index is incomplete. */
  state: string | null;
  /** `path:line` of each citing marker; null while the marker index is incomplete. */
  markers: string[] | null;
}

export interface NodeView {
  id: string;
  title: string;
  status: string;
  parent: string | null;
  depends_on: string[];
  contract: string;
  requirements: RequirementView[];
  assets: Record<string, string>;
}

export interface ChangeView {
  id: string;
  status: string;
  from: string | null;
  resolves: string[];
  nodes: string[];
  /** Requirements current of those its deltas add or modify; null while the index is incomplete. */
  current: number | null;
  total: number;
  complete: boolean | null;
}

export interface BacklogView {
  slug: string;
  kind: string;
  title: string;
  nodes: string[];
  origin: string | null;
  /** The requirements it blocks (`requirement-blocks`). */
  blocks: string[];
  /** The answers a `question` item offers; absent in a snapshot kept from before the Worker sent it, read as none. */
  choices?: string[];
  /** The item's Markdown body and its priority; absent in a snapshot kept from before the Worker sent them. */
  body?: string;
  priority?: string | null;
}

/** An open `answered` record of the implementation state (`answers`): the person's answer to
 * a backlog item, `mine` when its writer is the requester's `<login>/web`. */
export interface Answer {
  id: string;
  /** `<person>/<clone>`. */
  writer: string;
  at: string;
  item: string;
  answer: string;
  mine: boolean;
}

/** The `view` node's `spec` model: every node once, parents before children. */
export interface SpecNode {
  node: string;
  parent: string | null;
  title: string;
  status: string;
  owners: string[];
  depends_on: string[];
  /** Its requirements by coverage state (`check/pending-report`). */
  current: number;
  behind: number;
  unbuilt: number;
  /** Requirements a live change removed from it that a marker still cites. */
  obsolete: number;
  total: number;
  /** Over its size budget (`oversized-is-a-state`); absent when sizes weren't checked. */
  oversized?: boolean;
}

export interface SpecView {
  view: 'spec';
  nodes: SpecNode[];
}

export interface DecisionView {
  name: string;
  title: string;
  date: string;
  status: string;
}

export interface Snapshot {
  repo: string;
  sha: string;
  coverage: Coverage;
  /** Nesting, owners, status and requirements current, as `clm view spec` gives them. */
  spec: SpecView;
  nodes: NodeView[];
  changes: ChangeView[];
  backlog: BacklogView[];
  decisions: DecisionView[];
  violations: string[];
}

/** An entry of `GET /api/repos/{o}/{r}/branches`: a `spec/*` branch, with its open page. */
export interface SpecBranch {
  branch: string;
  head_sha: string;
  page: { number: number; url: string; title: string } | null;
}

export interface Texts {
  before: string | null;
  after: string | null;
}

export interface Element extends Texts {
  id?: string;
  path?: string;
}

export interface Changes {
  added: Element[];
  modified: Element[];
  removed: Element[];
}

export interface NodeChanges {
  id: string;
  status: string;
  frontmatter: Texts | null;
  contract: Texts | null;
  design: Texts | null;
  requirements: Changes;
  assets: Changes;
}

export interface ChangeInfo {
  id: string;
  status: string;
  from: string | null;
  resolves: string[];
  intent: string;
  intent_section: string | null;
}

export interface Record_ extends Texts {
  name?: string;
  slug?: string;
  status: string;
}

export interface Document extends Texts {
  path: string;
}

/** An `approved` record: its writer, `<person>/<clone>`, and its time. */
export interface Approval {
  writer: string;
  at: string;
}

/** `GET .../branches/{branch}/review`. */
export interface BranchReview {
  branch: string;
  /** The default branch. */
  base: string;
  base_sha: string;
  head_sha: string;
  /** Whether the branch needs a human's review (`gate/needs-review-of`), with or without a change. */
  review: boolean;
  change: ChangeInfo | null;
  nodes: NodeChanges[];
  decisions: Record_[];
  backlog: Record_[];
  documents: Document[];
  /** A `checked` record names the head's code key. */
  checked: boolean;
  page: { number: number; url: string } | null;
  /** Null when a tree listing was truncated: such a branch can't be approved. */
  fingerprint: string | null;
  /** The ledger's `approved` records for the fingerprint, oldest first. */
  approvals: BranchApproval[];
  /** The default branch's tip holds `docs/allowed_signers`: `clm land` counts only signed
   * approvals. Absent in a review kept from before the Worker sent it, read as false. */
  signed?: boolean;
  /** The `commented` records for the branch, oldest first (web/comment-on-review). Absent in
   * a review kept from before the Worker sent it, read as none. */
  comments?: ReviewComment[];
  /** The `changes_requested` records for the branch, oldest first (web/request-changes).
   * Absent as `comments` may be. */
  change_requests?: ChangeRequest[];
}

/** A `commented` record on a spec branch. */
export interface ReviewComment {
  id: string;
  /** `<person>/<clone>`. */
  writer: string;
  at: string;
  fingerprint: string;
  /** Null for the change as a whole, else a requirement ID. */
  target: string | null;
  text: string;
  /** Its fingerprint is the review's. */
  current: boolean;
}

/** A `changes_requested` record on a spec branch. */
export interface ChangeRequest {
  id: string;
  writer: string;
  at: string;
  fingerprint: string;
  text: string;
  current: boolean;
  /** Current, and not since approved by the same person: `clm land` holds the branch. */
  held: boolean;
}

/** An `approved` record: whether its line carries a signature, which `clm land` checks. */
export interface BranchApproval extends Approval {
  signed?: boolean;
}

export type Review = BranchReview;

/** An `impl/*` branch's open page. */
export interface ImplPage {
  number: number;
  url: string;
  /** Whether a `checked` record names its head's code key; absent while that isn't known. */
  checked?: boolean;
}

/** What holds backlog items (`propose-claims`). */
export interface Taken {
  /** `state` is `main` or `proposed`. */
  changes: { id: string; state: string; items: string[]; nodes: string[] }[];
  claims: { item: string; nodes: string[] }[];
}

/** A pending requirement of a node in the `flight` model (`clm pending`). */
export interface PendingRequirement {
  id: string;
  version: string | null;
  /** `unbuilt`, `behind` or `obsolete`. */
  state: string;
  cited: string[];
  markers: string[];
  priority: string;
  /** Slugs of the backlog items whose `blocks` name it. */
  blocked_by: string[];
  /** The other requirements it waits on, as `pending-report` gives them. */
  waits_on: string[];
}

export interface FlightClaim {
  /** Unix seconds. */
  lapses_at: number;
  pushed_hours_ago: number;
  lapses_in_hours: number;
}

/** What blocks requirements of a node: exactly one of `item` and `requirement` is set. */
export interface FlightBlock {
  /** `backlog/<slug>`. */
  item?: string;
  /** `<node>/<slug>`, a pending requirement of another node they need (`requirement-needs`). */
  requirement?: string;
  requirements: string[];
}

/** A node with pending work, with the state `queue_from_refs` gives it. A state this app
 * doesn't know still shows by its name. */
export interface FlightNode {
  node: string;
  priority: string;
  state: string;
  pending: PendingRequirement[];
  claim: FlightClaim | null;
  blocked_by: FlightBlock[];
}

export interface FlightChange {
  change: string;
  priority: string;
  current: number;
  total: number;
  obsolete: string[];
  complete: boolean;
}

export interface FlightGroup {
  status: string;
  changes: FlightChange[];
}

export interface FlightView {
  view: 'flight';
  totals: {
    pending_nodes: number;
    by_state: Record<string, number>;
    live_changes: number;
    requirements_pending: number;
  };
  nodes: FlightNode[];
  changes: FlightGroup[];
}

export interface Held {
  node: string;
  /** The nodes between the root blocker and this one. */
  via: string[];
}

export interface Root {
  /** A node ID or `backlog/<slug>`. */
  root: string;
  kind: 'node' | 'backlog' | string;
  state: string;
  waits_on_human: boolean;
  weight: number;
  holds: Held[];
}

export interface BlockersView {
  view: 'blockers';
  roots: Root[];
}

export interface Activity {
  /** `push`, `claim`, `land` or `approval`. */
  kind: string;
  /** Unix seconds. */
  at: number;
  actor: string;
  /** The branch pushed, landed or approved, or the item claimed. */
  ref: string;
}

/** `GET /api/repos/{o}/{r}/implementation`: the `flight` and `blockers` models computed with
 * the live state at `now`, the `impl/*` branches' pages, the activity feed, what holds
 * backlog items. */
export interface Implementation {
  sha: string;
  now: number;
  expiry_hours: number;
  coverage: Coverage;
  flight: FlightView;
  blockers: BlockersView;
  /** Each `impl/*` branch with an open page. */
  pages: Record<string, ImplPage>;
  activity: Activity[];
  taken: Taken;
  /** The open proposals, by `at` then `id` (web/propose-from-backlog). Absent in a state
   * kept from before the Worker sent it, read as none. */
  proposals?: Proposal[];
  /** The open answers to backlog items, by `at` then `id`. Absent in a state kept from before
   * the Worker sent it, read as none. */
  answers?: Answer[];
}

/** A `proposed` record no `withdrawn` record names and no backlog item has filed yet. */
export interface Proposal {
  id: string;
  /** `<person>/<clone>`. */
  writer: string;
  at: string;
  slug: string;
  title: string;
  body: string;
  nodes: string[];
  /** The writer is the requester's `<login>/web`. */
  mine: boolean;
}

/** Work, as estimated dollars, tokens and time. */
export interface Spend {
  dollars: number;
  /** Input, output and cache writes. */
  work_tokens: number;
  cache_read_tokens: number;
  active_seconds: number;
}

/** A workaround flag open on a capability. */
export interface Flag {
  slug: string;
  /** The requirement it flags, `<node>/<slug>`. */
  requirement: string;
  title: string;
  text: string;
  /** Whether a decision on this capability settled it. */
  settled: boolean;
  /** `YYYY-MM-DD`; null without history, as the Worker's report always is. */
  added: string | null;
}

export type Rating = 'high' | 'medium' | 'low';
export type DecisionKind = 'keep' | 'cut' | 'redesign' | 'accept';

export interface Decided {
  at: string;
  writer: string;
  value: Rating;
  decision: DecisionKind;
}

/** One capability's page (`clm_effort::Cost`). The numbers only history gives are null. */
export interface Cost {
  /** `capability/<slug>`. */
  id: string;
  slug: string;
  /** The first 8 hex digits of its entry's hash: what a decision names. */
  version: string;
  statement: string;
  why: string;
  rules: string[];
  nodes: string[];
  spend: Spend;
  changes: number;
  approvals: number;
  flags_settled: number;
  burdens: string[];
  commits: number | null;
  questions_answered: number | null;
  /** The items that held its rules up, and the days they were open in all. */
  blocking: [number, number] | null;
  size_tokens: number | null;
  flags: Flag[];
  /** Its rules that serve no other capability, and the others the rest serve, with how many. */
  overlap: { only: number; shared: [string, number][] };
  decisions: Decided[];
  /** At least one flag is unsettled. */
  flagged: boolean;
  /** Its entry's Markdown (`effort/entry-text`), shown in full on its page. */
  text: string;
}

/** `GET .../capabilities` (web.worker/capabilities). */
export interface Capabilities {
  report: {
    /** By dollars, highest first. */
    capabilities: Cost[];
    unattributed: Spend;
    architecture_only: Spend;
  };
  /** The default branch's tip holds `docs/allowed_signers`. */
  signed: boolean;
}

/** One rule that enforces an architectural constraint, and whether it is current: null
 * while the Worker's marker index is incomplete. */
export interface Enforcing {
  rule: string;
  current: boolean | null;
}

/** One architectural constraint's page (`clm_effort::Constraint`): a `Cost`'s fields that
 * hold without history, over its enforcing rules. */
export interface Constraint {
  /** `architecture/<slug>`. */
  id: string;
  slug: string;
  /** The first 8 hex digits of its entry's hash: what a decision names. */
  version: string;
  statement: string;
  enforced_by: Enforcing[];
  /** Its `#### Untested` text. */
  untested: string | null;
  rules: string[];
  nodes: string[];
  spend: Spend;
  changes: number;
  approvals: number;
  flags_settled: number;
  burdens: string[];
  flags: Flag[];
  overlap: { only: number; shared: [string, number][] };
  decisions: Decided[];
  /** At least one flag is unsettled. */
  flagged: boolean;
  /** Its entry's Markdown (`effort/entry-text`), shown in full on its page. */
  text: string;
}

/** `GET .../architecture` (web.worker/architecture). */
export interface Architecture {
  /** By dollars, highest first. */
  architecture: Constraint[];
  /** The default branch's tip holds `docs/allowed_signers`. */
  signed: boolean;
  /** The marker index is incomplete, so no enforcing rule's state is known. */
  indexing: boolean;
}
