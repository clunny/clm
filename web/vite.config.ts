import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';
import { defineConfig } from 'vitest/config';
import { buildAssets } from './build-assets.ts';
import { iconPng } from './icons.ts';

/** The push handlers' module, built as a chunk and then inlined where `sw.js` imports it. */
const PUSH = 'sw-push.js';

/** `code`, an ES module chunk, as an expression giving an object of its exports, so a
 * classic script can hold it: browsers without module service workers still install it. */
function inlineModule(code: string): string {
  const exports = /export\s*\{([^}]*)\};?\s*$/.exec(code);
  if (!exports) throw new Error(`${PUSH}: no export list at its end`);
  const fields = exports[1]
    .split(',')
    .map((e) => e.trim().split(/\s+as\s+/))
    .map(([local, name]) => `${name ?? local}: ${local}`);
  return `(() => {\n${code.slice(0, exports.index)}\nreturn { ${fields.join(', ')} };\n})()`;
}

/** The app is served under this path, at `/app/` of the zone (web/app-at-app-path). */
const BASE = '/app/';

/** Emits the PNG icons, and `sw.js` (served at `/app/sw.js`) with the build's files to
 * precache and the push handlers it imports from `src/live/push-sw.ts` inlined
 * (web/push-shown). Beside the app's output it writes the retiring worker `/sw.js`
 * (web/retire-root-service-worker). */
function pwa(): Plugin {
  return {
    name: 'clm-pwa',
    apply: 'build',
    buildStart() {
      this.emitFile({ type: 'chunk', id: fileURLToPath(new URL('./src/live/push-sw.ts', import.meta.url)), fileName: PUSH, preserveSignature: 'strict' });
    },
    generateBundle(_, bundle) {
      for (const size of [192, 512]) {
        this.emitFile({ type: 'asset', fileName: `icon-${size}.png`, source: iconPng(size) });
      }
      const files = [
        BASE,
        `${BASE}manifest.webmanifest`,
        `${BASE}icon.svg`,
        `${BASE}icon-192.png`,
        `${BASE}icon-512.png`,
        ...Object.keys(bundle)
          .filter((f) => f !== 'index.html' && f !== PUSH)
          .map((f) => `${BASE}${f}`),
      ].sort();
      // From every precached file's content, so changing any of them, even one whose name
      // has no hash (the manifest, the icons, `index.html`), installs a new service worker.
      const hash = createHash('sha256');
      for (const [name, out] of Object.entries(bundle).sort(([a], [b]) => a.localeCompare(b))) {
        hash.update(name).update(out.type === 'chunk' ? out.code : out.source);
      }
      for (const name of ['manifest.webmanifest', 'icon.svg']) {
        hash.update(name).update(readFileSync(new URL(`./public/${name}`, import.meta.url)));
      }
      const version = hash.digest('hex').slice(0, 12);
      const push = bundle[PUSH];
      if (push?.type !== 'chunk') throw new Error(`${PUSH} was not built`);
      delete bundle[PUSH];
      const source = readFileSync(new URL('./sw.js', import.meta.url), 'utf8')
        .replace('self.__CLM_VERSION__', JSON.stringify(version))
        .replace('self.__CLM_PRECACHE__', JSON.stringify(files))
        .replace(/^import \{ (\w+), (\w+) \} from '\.\/sw-push\.js';$/m, (_, a, b) => `const { ${a}, ${b} } = ${inlineModule(push.code)};`);
      if (source.includes(PUSH)) throw new Error(`sw.js: its import of ${PUSH} was not inlined`);
      this.emitFile({ type: 'asset', fileName: 'sw.js', source });
    },
    writeBundle(options) {
      const out = join(dirname(options.dir ?? 'dist/app'), 'sw.js');
      mkdirSync(dirname(out), { recursive: true });
      copyFileSync(fileURLToPath(new URL('./sw-retire.js', import.meta.url)), out);
    },
  };
}

export default defineConfig({
  base: BASE,
  plugins: [buildAssets(new URL('../', import.meta.url)), pwa()],
  // ELK's bundle alone is about 1.4 MB; it loads on its own when a graph is drawn.
  // The app goes under `dist/app/`; the site is no part of this build (web/app-at-app-path).
  build: { outDir: 'dist/app', chunkSizeWarningLimit: 1600 },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts', 'test/**/*.test.ts'],
  },
});
