// req:web.client-live/live-updates@468e272c
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { LiveSocket, type SocketLike } from './live';

class FakeSocket implements SocketLike {
  sent: string[] = [];
  closed = false;
  onopen: ((e: Event) => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onclose: ((e: CloseEvent) => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  constructor(readonly url: string) {}
  send(data: string) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
  }
  /** The server's side. */
  open() {
    this.onopen?.(new Event('open'));
  }
  message(data: string) {
    this.onmessage?.(new MessageEvent('message', { data }));
  }
  fail() {
    this.onerror?.(new Event('error'));
    this.onclose?.(new CloseEvent('close'));
  }
}

const URL_A = 'wss://clm-app.dev/api/repos/o/a/live';
const URL_B = 'wss://clm-app.dev/api/repos/o/b/live';

function setup() {
  const sockets: FakeSocket[] = [];
  const notices: (string | undefined)[] = [];
  const live = new LiveSocket(
    (url) => {
      const s = new FakeSocket(url);
      sockets.push(s);
      return s;
    },
    (d) => notices.push(d),
  );
  return { live, sockets, notices, last: () => sockets[sockets.length - 1] };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('the live socket', () => {
  it('holds one socket for the repo on screen, and moves it when the repo changes', () => {
    const t = setup();
    t.live.want(URL_A);
    t.live.want(URL_A);
    expect(t.sockets.map((s) => s.url)).toEqual([URL_A]);
    t.live.want(URL_B);
    expect(t.sockets[0].closed).toBe(true);
    expect(t.sockets.map((s) => s.url)).toEqual([URL_A, URL_B]);
  });

  it('hidden: the socket is closed while hidden, and a new one opened when shown', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().open();
    t.live.want(null);
    expect(t.sockets[0].closed).toBe(true);
    vi.advanceTimersByTime(180_000);
    expect(t.sockets).toHaveLength(1);
    t.live.want(URL_A);
    expect(t.sockets).toHaveLength(2);
  });

  it('a notice refreshes with its delivery; a pong is no notice', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().open();
    t.last().message('{"delivery":"d1"}');
    t.last().message('pong');
    t.last().message('not json');
    expect(t.notices).toEqual(['d1', undefined]);
  });

  it('a silent drop: no pong within 10 seconds of a ping closes the socket and opens another', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().open();
    vi.advanceTimersByTime(50_000);
    expect(t.sockets[0].sent).toEqual(['ping']);
    vi.advanceTimersByTime(10_000);
    expect(t.sockets[0].closed).toBe(true);
    vi.advanceTimersByTime(1_000);
    expect(t.sockets).toHaveLength(2);
  });

  it('a pong keeps the socket', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().open();
    for (let i = 0; i < 3; i++) {
      vi.advanceTimersByTime(50_000);
      t.last().message('pong');
    }
    vi.advanceTimersByTime(20_000);
    expect(t.sockets).toHaveLength(1);
    expect(t.sockets[0].closed).toBe(false);
    expect(t.sockets[0].sent).toEqual(['ping', 'ping', 'ping']);
  });

  it('retries after any close, from 1 second doubling to at most 60', () => {
    const t = setup();
    t.live.want(URL_A);
    const waits: number[] = [];
    for (let i = 0; i < 8; i++) {
      t.last().fail();
      const n = t.sockets.length;
      let waited = 0;
      while (t.sockets.length === n) {
        vi.advanceTimersByTime(1_000);
        waited += 1_000;
      }
      waits.push(waited / 1000);
    }
    expect(waits).toEqual([1, 2, 4, 8, 16, 32, 60, 60]);
  });

  it('the delay resets to 1 second once a socket stays open for a minute', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().fail();
    vi.advanceTimersByTime(1_000);
    t.last().fail();
    vi.advanceTimersByTime(2_000);
    t.last().open();
    // Answers its pings, and stays open a minute.
    vi.advanceTimersByTime(50_000);
    t.last().message('pong');
    vi.advanceTimersByTime(10_000);
    const n = t.sockets.length;
    t.last().fail();
    vi.advanceTimersByTime(1_000);
    expect(t.sockets.length).toBe(n + 1);
  });

  it('closing on purpose retries nothing', () => {
    const t = setup();
    t.live.want(URL_A);
    t.last().open();
    t.live.want(null);
    t.sockets[0].fail();
    vi.advanceTimersByTime(120_000);
    expect(t.sockets).toHaveLength(1);
  });
});
