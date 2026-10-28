// Every public page, pasted into a chat or a post, shows a card: a title, a
// line about it and a picture. Without these tags WhatsApp, X, LinkedIn and
// Slack show a bare link — /app/ did, and it is the link people share most.
//
//   node --test scripts/share-preview.test.mjs

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

// The file each public address is built from (scripts/site-build.sh).
const PAGES = {
  '/': 'site/index.html',
  '/app/': 'web/editor/index.html',
  '/viewer/': 'web/index.html',
  '/connector/': 'site/connector/index.html',
  '/pricing/': 'site/pricing/index.html',
  '/privacy/': 'site/privacy/index.html',
  '/terms/': 'site/terms/index.html',
  '/refund/': 'site/refund/index.html',
};

/** The content of the first `<meta>` whose `attr` is `name`. */
function meta(html, attr, name) {
  const tag = html.match(new RegExp(`<meta\\s+${attr}="${name}"\\s+content="([^"]*)"`));
  return tag ? tag[1] : null;
}

for (const [path, file] of Object.entries(PAGES)) {
  test(`${path} shows a card when shared`, () => {
    const html = readFileSync(join(root, file), 'utf8');
    for (const name of ['og:title', 'og:description', 'og:url', 'og:image', 'og:image:alt']) {
      assert.ok(meta(html, 'property', name), `${file}: no ${name}`);
    }
    for (const name of ['description', 'twitter:card', 'twitter:title', 'twitter:image']) {
      assert.ok(meta(html, 'name', name), `${file}: no ${name}`);
    }
    assert.equal(meta(html, 'name', 'twitter:card'), 'summary_large_image', `${file}: small card`);
    assert.equal(meta(html, 'property', 'og:url'), `https://3dneweraai.com${path}`, `${file}: og:url is not ${path}`);

    // Crawlers fetch the picture by its absolute address, and it has to be
    // one the site actually publishes.
    const image = meta(html, 'property', 'og:image');
    assert.match(image, /^https:\/\/3dneweraai\.com\/[^/]+\.(jpg|png)$/, `${file}: og:image ${image}`);
    assert.ok(existsSync(join(root, 'site', image.split('/').pop())), `${file}: site/ has no ${image}`);
    assert.equal(meta(html, 'name', 'twitter:image'), image, `${file}: twitter:image differs from og:image`);
  });
}
