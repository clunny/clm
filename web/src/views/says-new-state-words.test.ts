import { describe, expect, it } from 'vitest';
import { renderInspector } from './inspector';
import { snapshot } from './fixtures';

describe('says the new state words', () => {
  // req:web.views/says-new-state-words@ac738b16
  it('an unbuilt component: the inspector names the status the model gives', () => {
    const insp = renderInspector(snapshot(), null, [], 'b');
    expect(insp.querySelector('.top')?.textContent).toContain('unbuilt');
  });

  // req:web.views/says-new-state-words@ac738b16
  it('the app maps no word to unbuilt: a status shows as given', () => {
    const snap = snapshot();
    snap.spec.nodes.find((n) => n.node === 'b')!.status = 'whatever';
    const insp = renderInspector(snap, null, [], 'b');
    expect(insp.querySelector('.top')?.textContent).toContain('whatever');
  });
});
