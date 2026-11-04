// The repo's live socket (web.client-live/live-updates): while the page is visible and online, one
// WebSocket to the Worker's `/live` route for the repo on screen. Its `Live` object sends a
// notice, `{"delivery"}`, when the repo changes, and the app refreshes at once. A browser
// reports nothing about a failed handshake, so every close or failure is retried alike, after
// 1 second doubling to 60; a `ping` every 50 seconds that gets no `pong` within 10 finds a
// socket a phone's network dropped without a word.

/** What the app uses of a `WebSocket`. */
export interface SocketLike {
  send(data: string): void;
  close(): void;
  onopen: ((e: Event) => void) | null;
  onmessage: ((e: MessageEvent) => void) | null;
  onclose: ((e: CloseEvent) => void) | null;
  onerror: ((e: Event) => void) | null;
}

const PING_MS = 50_000;
const PONG_MS = 10_000;
/** How long a socket stays open before the retry delay starts again at its first. */
const STEADY_MS = 60_000;
const FIRST_DELAY_MS = 1_000;
const MAX_DELAY_MS = 60_000;

export class LiveSocket {
  private url: string | null = null;
  private ws: SocketLike | null = null;
  private delay = FIRST_DELAY_MS;
  private timers: ReturnType<typeof setTimeout>[] = [];
  private ping: ReturnType<typeof setInterval> | null = null;
  private pong: ReturnType<typeof setTimeout> | null = null;

  /** `open` makes a socket for a URL; `onNotice` gets each notice's delivery, or undefined
   * for a message that names none. */
  constructor(
    private open: (url: string) => SocketLike,
    private onNotice: (delivery: string | undefined) => void,
  ) {}

  /** The live URL of the repo on screen while the page is visible and online, else null:
   * a different URL closes the socket held and opens one for it. */
  want(url: string | null) {
    if (url === this.url) return;
    this.stop();
    this.url = url;
    this.delay = FIRST_DELAY_MS;
    if (url) this.connect();
  }

  private connect() {
    const ws = this.open(this.url!);
    this.ws = ws;
    ws.onopen = () => {
      this.timers.push(setTimeout(() => (this.delay = FIRST_DELAY_MS), STEADY_MS));
      this.ping = setInterval(() => {
        ws.send('ping');
        this.pong ??= setTimeout(() => this.drop(), PONG_MS);
      }, PING_MS);
    };
    ws.onmessage = (e) => {
      if (e.data === 'pong') {
        if (this.pong !== null) clearTimeout(this.pong);
        this.pong = null;
        return;
      }
      this.onNotice(delivery(e.data));
    };
    ws.onclose = ws.onerror = () => {
      if (this.ws === ws) this.drop();
    };
  }

  /** Closes the socket held and opens another after the current delay. */
  private drop() {
    this.stop();
    if (!this.url) return;
    const wait = this.delay;
    this.delay = Math.min(this.delay * 2, MAX_DELAY_MS);
    this.timers.push(setTimeout(() => this.connect(), wait));
  }

  private stop() {
    for (const t of this.timers) clearTimeout(t);
    this.timers = [];
    if (this.ping !== null) clearInterval(this.ping);
    if (this.pong !== null) clearTimeout(this.pong);
    this.ping = this.pong = null;
    const ws = this.ws;
    this.ws = null;
    if (!ws) return;
    ws.onopen = ws.onmessage = ws.onclose = ws.onerror = null;
    try {
      ws.close();
    } catch {
      // Already closed.
    }
  }
}

function delivery(data: unknown): string | undefined {
  try {
    const d = typeof data === 'string' ? (JSON.parse(data) as { delivery?: unknown }).delivery : undefined;
    return typeof d === 'string' ? d : undefined;
  } catch {
    return undefined;
  }
}
