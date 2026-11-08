// The browser tests' timeouts. Checkouts land in parallel on one machine, so a suite runs
// while other builds compile and run their own suites: a first paint that takes a second on an
// idle machine took over five under that load, and the default 5 s assertion timeout failed
// tests that were only slow, so an unrelated build's land check failed at random.
import { describe, expect, it } from 'vitest';
import config from '../playwright.config';

describe('playwright config', () => {
  it('gives an assertion long enough for a first paint on a loaded machine', () => {
    expect(config.expect?.timeout).toBeGreaterThanOrEqual(20_000);
  });

  it('gives a test room for more than one slow assertion', () => {
    expect(config.timeout).toBeGreaterThanOrEqual(2 * config.expect!.timeout!);
  });

  // Two runs on one host can be handed the same `CLM_WEB_PORT`, or one can take the port the
  // other probed as free. Reusing a server already answering there would test the other
  // checkout's build, so the run fails instead (`tests-run-concurrently`).
  it('never tests a preview server another run started', () => {
    const servers = [config.webServer].flat();
    expect(servers).toHaveLength(1);
    expect(servers[0]?.reuseExistingServer).toBe(false);
  });
});
