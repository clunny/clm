// The port the browser tests' preview server listens on. Checkouts built in parallel each
// run this suite, so a fixed port (4173, once) let one checkout's server starting or stopping
// break another's tests with connection refused.
import { createServer } from 'node:net';
import { describe, expect, it } from 'vitest';
import { previewPort } from './preview-port';

/** Resolves once `port` could be listened on, and released again. */
function free(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const server = createServer();
    server.once('error', () => resolve(false));
    server.listen(port, '127.0.0.1', () => server.close(() => resolve(true)));
  });
}

describe('previewPort', () => {
  it('uses CLM_WEB_PORT when it is set', () => {
    const env = { CLM_WEB_PORT: '5123' };
    expect(previewPort(env)).toBe(5123);
    expect(env).toEqual({ CLM_WEB_PORT: '5123' });
  });

  it('otherwise picks a free port once and keeps it in CLM_WEB_PORT for the workers', async () => {
    const env: Record<string, string | undefined> = {};
    const port = previewPort(env);
    expect(port).toBeGreaterThan(1023);
    expect(port).toBeLessThan(65536);
    expect(env.CLM_WEB_PORT).toBe(String(port));
    expect(previewPort(env)).toBe(port);
    expect(await free(port)).toBe(true);
  });
});
