// The site's pages written twice, in Portuguese and English, open in the
// language the visitor reads: legal.js shows one section and hides the other.
// A page that has the English text but not the Portuguese section, or not the
// script, opens in Portuguese with the English stacked below — /connector/ did.
//
//   node --test scripts/bilingual-pages.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = join(dirname(fileURLToPath(import.meta.url)), '..', 'site');
const pages = readdirSync(site, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && existsSync(join(site, entry.name, 'index.html')))
  .map((entry) => `${entry.name}/index.html`);

for (const page of pages) {
  const html = readFileSync(join(site, page), 'utf8');
  if (!html.includes('id="en"')) continue;
  test(`/${dirname(page)}/ opens in the visitor's language`, () => {
    for (const code of ['pt', 'en']) {
      const section = html.match(new RegExp(`<section id="${code}"[^>]*>`));
      assert.ok(section, `${page}: no <section id="${code}">`);
      assert.match(section[0], /data-title="[^"]+"/, `${page}: #${code} has no data-title`);
      assert.match(section[0], /data-description="[^"]+"/, `${page}: #${code} has no data-description`);
    }
    assert.equal((html.match(/data-lang="en"/g) || []).length, 2, `${page}: the PT · EN switch is not in both sections`);
    assert.ok(html.includes('<script src="/legal.js" defer></script>'), `${page}: legal.js is not loaded`);
  });
}
