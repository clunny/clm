import { describe, expect, it } from 'vitest';
import { canvasHeight } from './fit';

describe('canvasHeight', () => {
  // req:web/graph-fills-height@f8bc8a07
  it('runs from the canvas top to the limit', () => {
    expect(canvasHeight(150, 978)).toBe(828);
  });
  // req:web/graph-fills-height@f8bc8a07
  it('is at least 320 px', () => {
    expect(canvasHeight(150, 300)).toBe(320);
  });
  // req:web/graph-fills-height@f8bc8a07
  it('rounds down, so it never passes the limit', () => {
    expect(canvasHeight(100.4, 600.9)).toBe(500);
  });
});
