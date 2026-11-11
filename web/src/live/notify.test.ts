// req:web.client-live/notify-switch@bc04aecc
// req:web.client-live/push-renewal@d02c4647
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
  const env: NotifyEnv = {
    pushManager: async () => (s.pushManager === false ? null : pm),
    key: async () => (s.key === undefined ? 'k' : s.key),
    permission: () => s.permission ?? 'granted',
    requestPermission: vi.fn(async () => s.answer ?? 'granted'),
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
  return { env, pm, sent, asked, store, current: () => current };
}

const box = (e: HTMLElement) => e.querySelector<HTMLInputElement>('input[type=checkbox]')!;
const shown = (e: HTMLElement) => !e.hidden && box(e) !== null;

/** Turns the switch on or off as a person does. */
function toggle(e: HTMLElement) {
  const b = box(e);
  b.checked = !b.checked;
  b.dispatchEvent(new Event('change'));
}

describe('Notify me', () => {
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

  it('no key: a Worker answering 404 for the key shows no switch', async () => {
    const t = setup({ key: null });
    const e = new Notify(t.env).switchFor('o', 'a');
    await new Promise((r) => setTimeout(r, 10));
    expect(e.hidden).toBe(true);
  });

  it('no Web Push or no service worker: no switch', async () => {
    const t = setup({ pushManager: false });
    const e = new Notify(t.env).switchFor('o', 'a');
    await new Promise((r) => setTimeout(r, 10));
    expect(e.hidden).toBe(true);
  });

  it('its state is what the push route answers for the browser endpoint', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/1', subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/1' } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(box(e)?.checked).toBe(true));
    expect(t.asked).toEqual(['/api/repos/o/a https://fcm.googleapis.com/fcm/send/1']);
  });

  it('off, and asks nothing, when the browser has no subscription', async () => {
    const t = setup({ existing: null });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).checked).toBe(false);
    expect(t.asked).toEqual([]);
  });

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

  it('disabled offline', async () => {
    const t = setup({ online: false });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).disabled).toBe(true);
  });

  it('a renewed browser subscription: the old endpoint is deleted and the new one posted for each repo that was on', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/renewed', stored: { 'o/a': 'https://fcm.googleapis.com/fcm/send/old', 'o/b': 'https://fcm.googleapis.com/fcm/send/renewed' } });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(t.sent).toHaveLength(2));
    expect(t.sent).toEqual([
      { method: 'POST', path: '/api/repos/o/a/push', body: t.current()!.toJSON() },
      { method: 'DELETE', path: '/api/repos/o/a/push', body: { endpoint: 'https://fcm.googleapis.com/fcm/send/old' } },
    ]);
    expect(JSON.parse(t.store.get('clm-push')!)).toEqual({ 'o/a': 'https://fcm.googleapis.com/fcm/send/renewed', 'o/b': 'https://fcm.googleapis.com/fcm/send/renewed' });
    await vi.waitFor(() => expect(shown(e)).toBe(true));
  });

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
  });

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

  it('refresh asks the push route again, with fresh, for the switches on screen', async () => {
    const t = setup({ existing: 'https://fcm.googleapis.com/fcm/send/1', subscribed: { '/api/repos/o/a': false } });
    const fresh: (string | undefined)[] = [];
    const subscribed = t.env.subscribed;
    t.env.subscribed = (base, endpoint, f) => {
      fresh.push(f);
      return subscribed(base, endpoint, f);
    };
    const n = new Notify(t.env);
    const e = n.switchFor('o', 'a');
    document.body.append(e);
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    expect(box(e).checked).toBe(false);
    t.env.subscribed = async (_b, _e, f) => {
      fresh.push(f);
      return true;
    };
    await n.refresh('d1');
    expect(box(e).checked).toBe(true);
    expect(fresh).toEqual([undefined, 'd1']);
    e.remove();
  });

  it('redrawn with a known state: shows it at once, and keeps it when the ask fails', async () => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    const t = setup({ existing: endpoint, subscribed: { '/api/repos/o/a': true }, stored: { 'o/a': endpoint } });
    const n = new Notify(t.env);
    const first = n.switchFor('o', 'a');
    await vi.waitFor(() => expect(box(first)?.checked).toBe(true));
    let release: (() => void) | null = null;
    t.env.subscribed = () =>
      new Promise<boolean>((_resolve, reject) => {
        release = () => reject(new Error('down'));
      });
    const again = n.switchFor('o', 'a');
    expect(again.hidden).toBe(false);
    expect(box(again).checked).toBe(true);
    await vi.waitFor(() => expect(release).not.toBeNull());
    release!();
    await new Promise((r) => setTimeout(r, 10));
    expect(again.hidden).toBe(false);
    expect(box(again).checked).toBe(true);
  });

  it('a post the Worker refuses leaves the switch off and says so', async () => {
    const t = setup({ subscribed: { '/api/repos/o/a': false } });
    t.env.send = async () => new Response(null, { status: 403 });
    const e = new Notify(t.env).switchFor('o', 'a');
    await vi.waitFor(() => expect(shown(e)).toBe(true));
    toggle(e);
    await vi.waitFor(() => expect(e.textContent).toMatch(/Not turned on \(403\)/));
    expect(box(e).checked).toBe(false);
  });

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
});
