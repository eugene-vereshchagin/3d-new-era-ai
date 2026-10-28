// The landing page shows the demo video: a poster the site publishes, a link
// that works without JavaScript, and its words in every language the page
// switches to.
//
//   node --test scripts/landing-video.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const html = readFileSync(join(root, 'site/index.html'), 'utf8');
const js = readFileSync(join(root, 'site/app.js'), 'utf8');

test('the landing page links the demo video, with a poster it publishes', () => {
  const link = html.match(/<a class="demo__play" href="([^"]+)" data-youtube="([^"]+)"[^>]*>\s*<img src="([^"]+)"/);
  assert.ok(link, 'no demo video on the landing page');
  const [, href, id, poster] = link;
  assert.match(id, /^[\w-]{11}$/, `not a YouTube id: ${id}`);
  assert.equal(href, `https://www.youtube.com/watch?v=${id}`, 'the link and the embed play different videos');
  assert.ok(existsSync(join(root, 'site', poster)), `site/ has no ${poster}`);
});

for (const lang of ['EN', 'ES', 'FR']) {
  test(`${lang} has the video's words`, () => {
    const start = js.indexOf(`const ${lang} = {`);
    assert.ok(start >= 0, `no ${lang} dictionary`);
    const body = js.slice(start, js.indexOf('\n  };', start));
    for (const key of ['demo.fig', 'alt.video', 'aria.video']) {
      assert.ok(body.includes(`"${key}":`), `${lang} has no "${key}"`);
    }
  });
}
