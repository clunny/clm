import { describe, expect, it, vi } from 'vitest';
import { Notify, type NotifyEnv, type PushManagerLike, type SubscriptionLike } from './notify';

function subscription(endpoint: string): SubscriptionLike & { unsubscribe: ReturnType<typeof vi.fn<() => Promise<boolean>>> } {
  return {
    endpoint,
    toJSON: () => ({ endpoint, keys: { p256dh: 'p', auth: 'a' } }),
    unsubscribe: vi.fn<() => Promise<boolean>>(async () => true),
  };
}

interface Setup {
  key?: string | null;
  permission?: NotificationPermission;
  answer?: NotificationPermission;
  existing?: string | null;
  subscribed?: Record<string, boolean>;
  online?: boolean;
  stored?: Record<string, string>;
  pushManager?: boolean;
}

/** A browser and Worker as `s` says: the key, the permission, the browser's subscription,
 * and what `GET .../push` answers per repo base. */
function setup(s: Setup = {}) {
  let current: SubscriptionLike | null = s.existing ? subscription(s.existing) : null;
  const pm = {
    getSubscription: vi.fn(async () => current),
    subscribe: vi.fn(async (_o: { userVisibleOnly: boolean; applicationServerKey: string }) => (current = subscription('https://fcm.googleapis.com/fcm/send/new'))),
  } satisfies PushManagerLike;
  const store = new Map(Object.entries({ ...(s.stored ? { 'clm-push': JSON.stringify(s.stored) } : {}) }));
  const sent: { method: string; path: string; body: unknown }[] = [];
  const asked: string[] = [];
  const pushManager = vi.fn(async (): Promise<PushManagerLike | null> => (s.pushManager === false ? null : pm));
  const key = vi.fn(async (): Promise<string | null> => (s.key === undefined ? 'k' : s.key));
  const requestPermission = vi.fn(async (): Promise<NotificationPermission> => s.answer ?? 'granted');
  const env: NotifyEnv = {
    pushManager,
    key,
    permission: () => s.permission ?? 'granted',
    requestPermission,
    subscribed: async (base, endpoint) => {
      asked.push(`${base} ${endpoint}`);
      return s.subscribed?.[base] ?? false;
    },
    send: async (method, path, body) => {
      sent.push({ method, path, body });
      return new Response(null, { status: 200 });
    },
    apiBase: (owner, repo) => `/api/repos/${owner}/${repo}`,
    online: () => s.online ?? true,
    storage: { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => void store.set(k, v) },
  };
  const setCurrent = (sub: SubscriptionLike | null) => void (current = sub);
  return { env, pm, sent, asked, store, current: () => current, setCurrent, pushManager, key, requestPermission };
}

const box = (e: HTMLElement) => e.querySelector<HTMLInputElement>('input[type=checkbox]')!;
const shown = (e: HTMLElement) => !e.hidden && box(e) !== null;
/** Lets every pending promise and short timer run. */
const settle = () => new Promise((r) => setTimeout(r, 10));
const E1 = 'https://fcm.googleapis.com/fcm/send/1';

/** Turns the switch on or off as a person does. */
function toggle(e: HTMLElement) {
  const b = box(e);
  b.checked = !b.checked;
  b.dispatchEvent(new Event('change'));
}

describe('Notify me', () => {
  // req:web.client-live/notify-toggle@a43dc7fb
  it('turn on: subscribes with the key, posts the subscription once, and reads on', async () => {
    const t = setup({ key: 'k', subscribed: { '/api/repos/o/a': false } });
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).checked).toBe(false);
    expect(e.textContent).toContain('Notify me');
    toggle(e);
    await vi.waitFor(() => expect(t.sent).toHaveLength(1));
    expect(t.pm.subscribe).toHaveBeenCalledWith({ userVisibleOnly: true, applicationServerKey: 'k' });
    expect(t.sent[0]).toEqual({ method: 'POST', path: '/api/repos/o/a/push', body: t.current()!.toJSON() });
    await vi.waitFor(() => expect(box(e).disabled).toBe(false));
    expect(box(e).checked).toBe(true);
    expect(t.env.requestPermission).not.toHaveBeenCalled();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('no key: an env whose key is null shows no switch, and asks for neither again', async () => {
    const t = setup({ key: null });
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await settle();
    expect(e.hidden).toBe(true);
    await n.refresh();
    expect(e.hidden).toBe(true);
    expect(t.key).toHaveBeenCalledTimes(1);
    expect(t.pushManager).toHaveBeenCalledTimes(1);
    e.remove();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('no Web Push: an env whose push manager is null shows no switch, and asks for neither again', async () => {
    const t = setup({ pushManager: false });
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await settle();
    expect(e.hidden).toBe(true);
    await n.refresh();
    expect(e.hidden).toBe(true);
    expect(t.key).not.toHaveBeenCalled();
    expect(t.pushManager).toHaveBeenCalledTimes(1);
    e.remove();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('its state is what the push route answers for the browser endpoint', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/1', subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/1' } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
    expect(t.asked).toEqual(['/api/repos/o/a https://fcm.googleapis.com/fcm/send/1']);
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('off, and asks nothing, when the browser has no subscription', async () => {
    const t = setup({ existing: null });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).checked).toBe(false);
    expect(t.asked).toEqual([]);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('asks for permission when needed, and a refusal leaves it off and says so', async () => {
    const t = setup({ permission: 'default', answer: 'denied' });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/blocked/));
    expect(t.env.requestPermission).toHaveBeenCalled();
    expect(t.pm.subscribe).not.toHaveBeenCalled();
    expect(t.sent).toEqual([]);
    expect(box(e).checked).toBe(false);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  // req:web.client-live/push-renewal@d1d92077
  it('turn off: deletes the endpoint, and unsubscribes the browser only when no other repo is on', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': true, '/api/repos/o/b': true }, stored: { 'o/a': endpoint, 'o/b': endpoint } });
    const n = new Notify(t.env);
    const a = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(box(a)?.checked).toBe(true));
    toggle(a);
    await vi.waitFor(() => expect(t.sent).toHaveLength(1));
    expect(t.sent[0]).toEqual({ method: 'DELETE', path: '/api/repos/o/a/push', body: { endpoint } });
    await vi.waitFor(() => expect(box(a).disabled).toBe(false));
    expect(t.current()!.unsubscribe).not.toHaveBeenCalled();
    const sub = t.current()!;
    const b = n.switchFor('o', 'b');
    await vi.waitFor(() => expect(box(b)?.checked).toBe(true));
    toggle(b);
    await vi.waitFor(() => expect(sub.unsubscribe).toHaveBeenCalled());
    expect(t.sent.map((x) => `${x.method} ${x.path}`)).toEqual(['DELETE /api/repos/o/a/push', 'DELETE /api/repos/o/b/push']);
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('disabled offline, as drawn or updated', async () => {
    const t = setup({ online: false });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).disabled).toBe(true);

    // Apart: online as drawn, offline at the next update.
    let online = true;
    const u = setup();
    u.env.online = () => online;
    const n = new Notify(u.env);
    const f = n.switchFor('o', 'a');
    document.body.append(f);
    await vi.waitFor(() => expect(shown(f)).toBe(true));
    expect(box(f).disabled).toBe(false);
    online = false;
    await n.refresh();
    expect(box(f).disabled).toBe(true);
    f.remove();
  });

  // req:web.client-live/push-renewal@d1d92077
  it('a renewed browser subscription: the old endpoint is deleted and the new one posted for each repo that was on', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/renewed', stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/old', 'o/b': 'https://fcm.googleapis.com/fcm/send/renewed' } });
    const n = new Notify(t.env);
    const a = n.switchFor('o', 'a');
    const b = n.switchFor('o', 'b');
    await vi.waitFor(() => expect(shown(a) && shown(b)).toBe(true));
    await settle();
    expect(t.sent).toEqual([
      { method: 'POST', path: '/api/repos/o/a/push', body: t.current()!.toJSON() },
      { method: 'DELETE', path: '/api/repos/o/a/push', body: { endpoint: 'https://fcm.googleapis.com/fcm/send/old' } },
    ]);
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({ 'o/a': 'https://fcm.googleapis.com/fcm/send/renewed', 'o/b': 'https://fcm.googleapis.com/fcm/send/renewed' });
  });

  // req:web.client-live/push-renewal@d1d92077
  it('a renewal whose post fails keeps the old endpoint and is tried again', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/renewed', stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/old' } });
    let fail = true;
    const send = t.env.send;
    t.env.send = async (method, path, body) => (fail ? new Response(null, { status: 500 }) : send(method, path, body));
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({ 'o/a': 'https://fcm.googleapis.com/fcm/send/old' });
    fail = false;
    await n.refresh();
    expect(t.sent.map((x) => x.method)).toEqual(['POST', 'DELETE']);
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({ 'o/a': 'https://fcm.googleapis.com/fcm/send/renewed' });
    e.remove();

    // Apart: the POST answered 200 and the DELETE 500; the DELETE is never sent again.
    const u = setup({ existing: 'https://fcm.googleapis.com/fcm/send/renewed', stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/old' } });
    u.env.send = async (method, path, body) => {
      u.sent.push({ method, path, body });
      return new Response(null, { status: method === 'DELETE' ? 500 : 200 });
    };
    const m = new Notify(u.env);
    const f = m.switchFor('o', 'a');
    document.body.append(f);
    await vi.waitFor(() => expect(shown(f)).toBe(true));
    await settle();
    expect(JSON.parse(u.store.get('clm-push')!)).toEqual({ 'o/a': 'https://fcm.googleapis.com/fcm/send/renewed' });
    await m.refresh();
    expect(u.sent.map((x) => x.method)).toEqual(['POST', 'DELETE']);
    f.remove();
  });

  // req:web.client-live/push-renewal@d1d92077
  it('without storage, turning a repo off never unsubscribes the browser', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': true } });
    t.env.storage = {
      getItem: () => null,
      setItem: () => {
        throw new Error('blocked');
      },
    };
    const n = new Notify(t.env);
    const a = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(box(a)?.checked).toBe(true));
    toggle(a);
    await vi.waitFor(() => expect(t.sent).toHaveLength(1));
    toggle(a);
    await vi.waitFor(() => expect(t.sent).toHaveLength(2));
    toggle(a);
    await vi.waitFor(() => expect(t.sent).toHaveLength(3));
    expect(t.sent.map((x) => x.method)).toEqual(['DELETE', 'POST', 'DELETE']);
    await vi.waitFor(() => expect(box(a).disabled).toBe(false));
    expect(t.current()!.unsubscribe).not.toHaveBeenCalled();
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('tapping the message leaves the switch as it is', async () => {
    const t = setup({ permission: 'default', answer: 'denied' });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/blocked/));
    e.querySelector<HTMLElement>('.note')!.click();
    expect(box(e).checked).toBe(false);
    expect(t.env.requestPermission).toHaveBeenCalledTimes(1);
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('refresh asks the push route again, with fresh, for the switches still in the document', async () => {
    const t = setup({ existing: E1 });
    const asks: string[] = [];
    t.env.subscribed = async (base, endpoint, f) => {
      asks.push(`${base} ${endpoint} ${f}`);
      return base === '/api/repos/o/a' && f === 'd1';
    };
    const n = new Notify(t.env);
    const a = n.switchFor('o', 'a');
    const b = n.switchFor('o', 'b');
    document.body.append(a, b);
    await vi.waitFor(() => expect(shown(a) && shown(b)).toBe(true));
    expect(box(a).checked).toBe(false);
    expect([...asks].sort()).toEqual([`/api/repos/o/a ${E1} undefined`, `/api/repos/o/b ${E1} undefined`]);
    b.remove();
    await n.refresh('d1');
    expect(asks.slice(2)).toEqual([`/api/repos/o/a ${E1} d1`]);
    expect(box(a).checked).toBe(true);
    a.remove();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('redrawn with a known state: shows it at once, and keeps it when the ask fails', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': endpoint } });
    const n = new Notify(t.env);
    const first = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(box(first)?.checked).toBe(true));
    const hold: { release: (() => void) | null } = { release: null };
    t.env.subscribed = () =>
      new Promise<boolean>((_resolve, reject) => {
        hold.release = () => reject(new Error('down'));
      });
    const again = n.switchFor('o', 'a');
    expect(again.hidden).toBe(false);
    expect(box(again).checked).toBe(true);
    await vi.waitFor(() => expect(hold.release).not.toBeNull());
    hold.release!();
    await new Promise((r) => setTimeout(r, 10));
    expect(again.hidden).toBe(false);
    expect(box(again).checked).toBe(true);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  // req:web.client-live/push-renewal@d1d92077
  it('turn on over an existing browser subscription posts it, remembers it, and turning off then unsubscribes', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': false } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(t.sent).toHaveLength(1));
    expect(t.pm.subscribe).not.toHaveBeenCalled();
    expect(t.sent[0].body).toEqual(t.current()!.toJSON());
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({ 'o/a': endpoint });
    await vi.waitFor(() => expect(box(e).disabled).toBe(false));
    toggle(e);
    await vi.waitFor(() => expect(t.current()!.unsubscribe).toHaveBeenCalled());
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({});
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('a post the Worker refuses leaves the switch off and says so', async () => {
    const t = setup({ subscribed: { '/api/repos/o/a': false } });
    t.env.send = async () => new Response(null, { status: 403 });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/Not turned on \(403\)/));
    expect(box(e).checked).toBe(false);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('a delete the Worker refuses leaves the switch on and says so', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': endpoint } });
    t.env.send = async () => new Response(null, { status: 500 });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/Not turned off \(500\)/));
    expect(box(e).checked).toBe(true);
    expect(t.current()!.unsubscribe).not.toHaveBeenCalled();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it("the key can't be fetched: hidden, and shown after a refresh", async () => {
    const t = setup();
    let first = true;
    t.key.mockImplementation(async () => {
      if (first) {
        first = false;
        throw new Error('down');
      }
      return 'k';
    });
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await settle();
    expect(e.hidden).toBe(true);
    await n.refresh();
    expect(shown(e)).toBe(true);
    expect(t.key).toHaveBeenCalledTimes(2);
    expect(t.pushManager).toHaveBeenCalledTimes(2);
    e.remove();
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('nothing to go on: shown off', async () => {
    const t = setup({ existing: E1, subscribed: { '/api/repos/o/a': true } });
    t.pm.getSubscription.mockImplementation(async () => {
      throw new Error('no');
    });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).checked).toBe(false);
    expect(t.asked).toEqual([]);

    // Apart: the first ask for the browser subscription rejects.
    const u = setup({ existing: E1 });
    u.env.subscribed = async () => {
      throw new Error('down');
    };
    const f = new Notify(u.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(f)).toBe(true));
    expect(box(f).checked).toBe(false);
  });

  // req:web.client-live/notify-switch@7b2d0be3
  it('two switches at once share one ask', async () => {
    const t = setup({ existing: E1 });
    const asks: string[] = [];
    let answer: ((on: boolean) => void) | null = null;
    t.env.subscribed = (base, endpoint, f) => {
      asks.push(`${base} ${endpoint} ${f}`);
      return new Promise<boolean>((r) => (answer = r));
    };
    const n = new Notify(t.env);
    const a = n.switchFor('o', 'a');
    const b = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(answer).not.toBeNull());
    // Both switches have asked by now; the answer is still held.
    await settle();
    expect(asks).toEqual([`/api/repos/o/a ${E1} undefined`]);
    answer!(true);
    await vi.waitFor(() => expect(box(a).checked && box(b).checked).toBe(true));
    expect(asks).toHaveLength(1);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('busy: disabled while a write is in flight', async () => {
    // The push route answers off, so an update drawing over the write would show off.
    const t = setup();
    const sent: string[] = [];
    let answer: ((r: Response) => void) | null = null;
    t.env.send = (method, path) => {
      sent.push(`${method} ${path}`);
      return new Promise<Response>((r) => (answer = r));
    };
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    toggle(e);
    await vi.waitFor(() => expect(answer).not.toBeNull());
    expect(box(e).disabled).toBe(true);
    toggle(e);
    expect(box(e).checked).toBe(true);
    await n.refresh();
    await settle();
    expect(box(e).disabled).toBe(true);
    expect(box(e).checked).toBe(true);
    expect(sent).toEqual(['POST /api/repos/o/a/push']);
    answer!(new Response(null, { status: 200 }));
    await vi.waitFor(() => expect(box(e).disabled).toBe(false));
    expect(box(e).checked).toBe(true);
    expect(sent).toEqual(['POST /api/repos/o/a/push']);
    e.remove();
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('a turn-on that fails without an answer says to try again', async () => {
    for (const fail of ['send', 'subscribe', 'requestPermission'] as const) {
      const t = setup({ permission: fail === 'requestPermission' ? 'default' : 'granted' });
      if (fail === 'send')
        t.env.send = async () => {
          throw new TypeError('offline');
        };
      if (fail === 'subscribe')
        t.pm.subscribe.mockImplementation(async () => {
          throw new Error('no');
        });
      if (fail === 'requestPermission')
        t.requestPermission.mockImplementation(async () => {
          throw new Error('no');
        });
      const e = new Notify(t.env).switchFor('o', 'a');
      await vi.waitFor(() => expect(shown(e)).toBe(true));
      toggle(e);
      await vi.waitFor(() => expect(e.textContent, fail).toMatch(/Not turned on: try again\./));
      expect(box(e).checked, fail).toBe(false);
      expect(box(e).disabled, fail).toBe(false);
    }
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('a turn-off whose push manager rejects says to try again', async () => {
    const t = setup({ existing: E1, subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': E1 } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
    t.pm.getSubscription.mockImplementation(async () => {
      throw new Error('no');
    });
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/Not turned off: try again\./));
    expect(box(e).checked).toBe(true);
    expect(t.sent).toEqual([]);
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('off with no browser subscription deletes the remembered endpoint', async () => {
    for (const stored of [{ 'o/a': E1 }, undefined]) {
      const t = setup({ existing: E1, subscribed: { '/api/repos/o/a': true }, stored });
      const e = new Notify(t.env).switchFor('o', 'a');
      await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
      t.setCurrent(null);
      toggle(e);
      await vi.waitFor(() => expect(box(e).disabled).toBe(false));
      expect(box(e).checked).toBe(false);
      if (stored) {
        expect(t.sent).toEqual([{ method: 'DELETE', path: '/api/repos/o/a/push', body: { endpoint: E1 } }]);
        expect(JSON.parse(t.store.get('clm-push')!)).toEqual({});
      } else {
        expect(t.sent).toEqual([]);
      }
    }
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('an unsubscribe that fails still reads off', async () => {
    const t = setup({ existing: E1, subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': E1 } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
    const sub = t.current() as ReturnType<typeof subscription>;
    sub.unsubscribe.mockImplementation(async () => {
      throw new Error('no');
    });
    toggle(e);
    await vi.waitFor(() => expect(sub.unsubscribe).toHaveBeenCalled());
    await vi.waitFor(() => expect(box(e).disabled).toBe(false));
    expect(t.sent.map((x) => x.method)).toEqual(['DELETE']);
    expect(box(e).checked).toBe(false);
    expect(e.querySelector('.note')!.textContent).toBe('');
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({});
  });

  // req:web.client-live/push-renewal@d1d92077
  it('unreadable storage counts as nothing remembered', async () => {
    for (const raw of ['not json', '5', '["x"]']) {
      const t = setup({ existing: E1 });
      t.store.set('clm-push', raw);
      const e = new Notify(t.env).switchFor('o', 'a');
      await vi.waitFor(() => expect(shown(e)).toBe(true));
      await settle();
      expect(t.sent, raw).toEqual([]);
      toggle(e);
      await vi.waitFor(() => expect(box(e).disabled).toBe(false));
      expect(box(e).checked, raw).toBe(true);
      expect(t.sent.map((x) => `${x.method} ${x.path}`), raw).toEqual(['POST /api/repos/o/a/push']);
      expect(JSON.parse(t.store.get('clm-push')!), raw).toEqual({ 'o/a': E1 });
    }
  });

  // req:web.client-live/notify-toggle@a43dc7fb
  it('an ask answered after a turn-on draws nothing over it', async () => {
    const t = setup({ existing: E1 });
    let answer: ((on: boolean) => void) | null = null;
    t.env.subscribed = () => new Promise<boolean>((r) => (answer = r));
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await vi.waitFor(() => expect(answer).not.toBeNull());
    const held = answer!;
    // Shown, as a switch whose state was known would be, while its first ask is held.
    e.hidden = false;
    toggle(e);
    await vi.waitFor(() => expect(t.sent).toHaveLength(1));
    await vi.waitFor(() => expect(box(e).disabled).toBe(false));
    expect(box(e).checked).toBe(true);
    held(false);
    await settle();
    expect(box(e).checked).toBe(true);
    e.remove();
  });
});
