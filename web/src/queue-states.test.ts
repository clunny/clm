// req:web/queue-states@340afeed
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { renderImplementation } from './implementation';
import { NOW, flight, implementation, snapshot } from '../test/fixtures';

const SRC = dirname(fileURLToPath(import.meta.url));
// Built in parts, so this file doesn't name what it looks for: the state as a string, a
// key or a class, and the line that explained it.
const STATE = `${'inter'}active`;
const GONE = [
  new RegExp(`['"\`.]${STATE}\\b|\\b${STATE}\\s*:`),
  new RegExp(`Needs an ${STATE} session`),
  new RegExp(`${'waiting'}_on`),
];

describe('queue states', () => {
  it('the totals give the states in the order claimed, ready, blocked, and know no waiting state', () => {
    const totals = (by_state: Record<string, number>) => {
      const f = flight({ totals: { pending_nodes: 4, by_state, live_changes: 1, requirements_pending: 4 } });
      const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
      return view.querySelector('.stat[data-stat=pending] .note')?.textContent;
    };
    expect(totals({ blocked: 1, ready: 1, claimed: 1 })).toBe('1 claimed, 1 ready, 1 blocked');
    // A state the app doesn't know comes after the known ones.
    expect(totals({ waiting: 1, blocked: 1 })).toBe('1 blocked, 1 waiting');
  });

  it('a waiting node has no label of its own: it shows its state as the app gets it', () => {
    const f = flight();
    f.nodes[1] = { ...f.nodes[1], state: 'waiting' };
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    const top = view.querySelector('#node-a .top')?.textContent ?? '';
    expect(top).toContain('waiting');
    expect(top).not.toContain('Waiting');
  });

  it('a node is drawn without reading paths from the flight model', () => {
    const f = flight();
    f.nodes[0] = { ...f.nodes[0], paths: ['web/.npmrc'] } as (typeof f.nodes)[0];
    const view = renderImplementation(implementation({ flight: f }), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    expect(view.textContent).not.toContain('web/.npmrc');
  });

  it('no source file under web/src names the state the queue no longer has', () => {
    const files = readdirSync(SRC, { recursive: true, encoding: 'utf8' }).filter((f) => /\.\w+$/.test(f));
    expect(files.length).toBeGreaterThan(0);
    const naming = files.filter((f) => GONE.some((re) => re.test(readFileSync(join(SRC, f), 'utf8'))));
    expect(naming).toEqual([]);
  });
});
