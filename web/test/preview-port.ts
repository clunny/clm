// The port the browser tests' preview server listens on.
import { execFileSync } from 'node:child_process';

/** The port `playwright.config.ts` serves the built app on: `CLM_WEB_PORT` when `env` sets
 * it, else a free port picked once and kept there. Playwright's workers load the config
 * again with the runner's environment, so they read the port the runner picked, and
 * checkouts whose suites run at the same time each get a port of their own instead of
 * all sharing one. */
export function previewPort(env: Record<string, string | undefined> = process.env): number {
  if (!env.CLM_WEB_PORT) env.CLM_WEB_PORT = String(freePort());
  return Number(env.CLM_WEB_PORT);
}

/** A port nothing listens on right now, from the OS: a child process listens on port 0
 * and prints what it got, since a config file can't wait on a server. */
function freePort(): number {
  const script = "const s=require('net').createServer();s.listen(0,'127.0.0.1',()=>{process.stdout.write(String(s.address().port));s.close()})";
  return Number(execFileSync(process.execPath, ['-e', script], { encoding: 'utf8' }));
}
