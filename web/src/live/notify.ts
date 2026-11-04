// The Notify me switch for the repo on screen (web.client-live/notify-switch): it subscribes the service
// worker's push manager with the Worker's key and posts the subscription to the repo's push
// route, which records it in the person's web ledger (`push-subscriptions-in-ledger`), and
// deletes it there when turned off. What the browser can do and what the Worker answers come
// in through `NotifyEnv`, so the tests fake them.

export interface SubscriptionLike {
  endpoint: string;
  toJSON(): unknown;
  unsubscribe(): Promise<boolean>;
}

export interface PushManagerLike {
  getSubscription(): Promise<SubscriptionLike | null>;
  subscribe(options: { userVisibleOnly: boolean; applicationServerKey: string }): Promise<SubscriptionLike>;
}

export interface NotifyEnv {
  /** The registered service worker's push manager, or null where the browser lacks
   * `Notification` or `PushManager`, or no service worker is registered. */
  pushManager(): Promise<PushManagerLike | null>;
  /** `GET /api/push/key`'s key, or null when it answers 404; throws on other failures. */
  key(): Promise<string | null>;
  permission(): NotificationPermission;
  requestPermission(): Promise<NotificationPermission>;
  /** `GET <base>/push?endpoint=`'s `subscribed`, with `fresh` when a notice asked. */
  subscribed(base: string, endpoint: string, fresh?: string): Promise<boolean>;
  send(method: 'POST' | 'DELETE', path: string, body: unknown): Promise<Response>;
  /** The repo's API base, `/api/repos/<o>/<r>`. */
  apiBase(owner: string, repo: string): string;
  online(): boolean;
  storage: { getItem(key: string): string | null; setItem(key: string, value: string): void };
}

/** Where the app remembers, per repo `<owner>/<repo>`, the endpoint it posted. */
const STORE = 'clm-push';

/** An element with attributes (false is left off, true is an empty attribute). */
function el<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Record<string, string | boolean>, ...kids: (Node | string)[]): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) if (v !== false) e.setAttribute(k, v === true ? '' : v);
  e.append(...kids);
  return e;
}

interface Ready {
  key: string;
  pm: PushManagerLike;
}

interface Drawn {
  el: HTMLElement;
  update(fresh?: string): Promise<void>;
}

export class Notify {
  private ready: Promise<Ready | null> | null = null;
  /** The browser endpoint the remembered ones were last brought up to date with. */
  private renewed: string | null = null;
  /** The last state shown per repo, so a redraw doesn't flicker. */
  private states = new Map<string, boolean>();
  private drawn: Drawn[] = [];

  constructor(private env: NotifyEnv) {}

  private remembered(): Record<string, string> {
    try {
      const v = JSON.parse(this.env.storage.getItem(STORE) ?? '{}');
      return v && typeof v === 'object' ? v : {};
    } catch {
      return {};
    }
  }

  /** Whether every write to storage so far succeeded: without it, what the app remembers
   * can't tell that no other repo's switch is on. */
  private stored = true;

  private remember(repo: string, endpoint: string | null) {
    const all = this.remembered();
    if (endpoint === null) delete all[repo];
    else all[repo] = endpoint;
    try {
      this.env.storage.setItem(STORE, JSON.stringify(all));
    } catch {
      // The switch still works; only renewal, and unsubscribing the browser, can't.
      this.stored = false;
    }
  }

  /** The key and push manager, once both are there; a failure is asked again next time. */
  private available(): Promise<Ready | null> {
    this.ready ??= (async () => {
      const pm = await this.env.pushManager();
      if (!pm) return null;
      const key = await this.env.key();
      return key ? { key, pm } : null;
    })().catch(() => {
      this.ready = null;
      return null;
    });
    return this.ready;
  }

  /** The push route's answer, shared by the switches drawn at once (the sidebar's and the
   * phone's top bar's). */
  private asking = new Map<string, Promise<boolean>>();
  private ask(base: string, endpoint: string, fresh?: string): Promise<boolean> {
    const key = `${base} ${endpoint} ${fresh ?? ''}`;
    let p = this.asking.get(key);
    if (!p) {
      p = this.env.subscribed(base, endpoint, fresh).finally(() => this.asking.delete(key));
      this.asking.set(key, p);
    }
    return p;
  }

  /** When the browser renewed its subscription, moves every repo that was on to the new
   * endpoint: the old one deleted, the new one posted. */
  private async renew(sub: SubscriptionLike) {
    if (this.renewed === sub.endpoint) return;
    // Set first, so the switches drawn at once move each repo only once.
    this.renewed = sub.endpoint;
    let done = true;
    for (const [repo, endpoint] of Object.entries(this.remembered())) {
      if (endpoint === sub.endpoint) continue;
      const [owner, name] = repo.split('/');
      const base = this.env.apiBase(owner, name);
      const res = await this.env.send('POST', `${base}/push`, sub.toJSON()).catch(() => null);
      if (!res?.ok) {
        done = false;
        continue;
      }
      this.remember(repo, sub.endpoint);
      await this.env.send('DELETE', `${base}/push`, { endpoint }).catch(() => undefined);
    }
    // A repo not moved is tried again at the next update.
    if (!done) this.renewed = null;
  }

  /** The switch for `owner/repo`, hidden until the browser and the Worker are known to
   * support notifications, and then kept up to date with the push route. */
  switchFor(owner: string, repo: string): HTMLElement {
    const id = `${owner}/${repo}`;
    const base = this.env.apiBase(owner, repo);
    const known = this.states.get(id);
    const box = el('input', { type: 'checkbox', role: 'switch', checked: known === true, disabled: !this.env.online() });
    // The message sits outside the label, so tapping it doesn't toggle the switch.
    const msg = el('span', { class: 'note' });
    const wrap = el('div', { class: 'notify', hidden: known === undefined }, el('label', {}, box, ' Notify me'), msg);
    let busy = false;
    const draw = (on: boolean) => {
      this.states.set(id, on);
      box.checked = on;
      box.disabled = busy || !this.env.online();
      wrap.hidden = false;
    };
    const update = async (fresh?: string) => {
      const ready = await this.available();
      if (!ready) {
        wrap.hidden = true;
        return;
      }
      if (busy) return;
      const sub = await ready.pm.getSubscription().catch(() => null);
      if (sub) await this.renew(sub).catch(() => undefined);
      const on = sub ? await this.ask(base, sub.endpoint, fresh).catch(() => this.states.get(id) ?? false) : false;
      if (!busy) draw(on);
    };
    const turnOn = async (ready: Ready): Promise<boolean> => {
      if (this.env.permission() !== 'granted' && (await this.env.requestPermission()) !== 'granted') {
        msg.textContent = 'Notifications are blocked for this site: allow them in the browser to turn this on.';
        return false;
      }
      const sub = (await ready.pm.getSubscription()) ?? (await ready.pm.subscribe({ userVisibleOnly: true, applicationServerKey: ready.key }));
      const res = await this.env.send('POST', `${base}/push`, sub.toJSON());
      if (!res.ok) {
        msg.textContent = `Not turned on (${res.status}).`;
        return false;
      }
      this.renewed = sub.endpoint;
      this.remember(id, sub.endpoint);
      return true;
    };
    const turnOff = async (ready: Ready): Promise<boolean> => {
      const sub = await ready.pm.getSubscription();
      const endpoint = sub?.endpoint ?? this.remembered()[id];
      if (endpoint) {
        const res = await this.env.send('DELETE', `${base}/push`, { endpoint });
        if (!res.ok) {
          msg.textContent = `Not turned off (${res.status}).`;
          return true;
        }
      }
      this.remember(id, null);
      if (sub && this.stored && Object.keys(this.remembered()).length === 0) await sub.unsubscribe();
      return false;
    };
    box.addEventListener('change', async () => {
      const ready = await this.available();
      if (!ready || busy) return;
      const want = box.checked;
      busy = true;
      msg.textContent = '';
      box.disabled = true;
      let on = !want;
      try {
        on = want ? await turnOn(ready) : await turnOff(ready);
      } catch {
        msg.textContent = want ? 'Not turned on: try again.' : 'Not turned off: try again.';
      }
      busy = false;
      draw(on);
    });
    this.drawn = [...this.drawn.filter((d) => d.el.isConnected), { el: wrap, update }];
    void update();
    return wrap;
  }

  /** Asks the push route again for the switches on screen, with `fresh` when a notice asked. */
  async refresh(fresh?: string): Promise<void> {
    this.drawn = this.drawn.filter((d) => d.el.isConnected);
    await Promise.all(this.drawn.map((d) => d.update(fresh)));
  }
}
