// The site's own endpoints, the ones the app and the installers report to:
// functions/ping/ (the usage count) and functions/ping/sentry.js (crash
// reports from builds that hold no key).
//
//   node --test scripts/ping.test.mjs
//
// No network: `fetch` is replaced, so what is asserted is the request this
// would make — the whole point of the endpoints being here is that the payload
// Google and Sentry see is built on this side, from a closed list of fields.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { onRequestPost as count, payloadFor } from '../functions/ping/index.js';
import { onRequestPost as crash, ingest, withoutDsn } from '../functions/ping/sentry.js';

/** A POST the way the app makes it. */
const posting = (body, url = 'https://3dneweraai.com/ping') =>
  new Request(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });

/** Runs `handler` with `fetch` replaced, and hands back what it was called with. */
async function sent(handler, context) {
  const calls = [];
  const real = globalThis.fetch;
  globalThis.fetch = (url, options) => {
    calls.push({ url: String(url), options });
    return Promise.resolve(new Response('{}', { status: 200 }));
  };
  try {
    const answer = await handler(context);
    return { answer, calls };
  } finally {
    globalThis.fetch = real;
  }
}

const OPEN = { event: 'app_open', client_id: 'abc123', operating_system: 'macos', mode: 'gui' };

test('a run of the app becomes one Measurement Protocol event', () => {
  const payload = payloadFor({ ...OPEN, app_version: '1.10.0', architecture: 'aarch64' }, 'BR');
  assert.equal(payload.client_id, 'abc123');
  assert.equal(payload.events.length, 1);
  const event = payload.events[0];
  assert.equal(event.name, 'app_open');
  assert.equal(event.params.operating_system, 'macos');
  assert.equal(event.params.app_version, '1.10.0');
  assert.equal(event.params.mode, 'gui');
  // The country comes from the connection, so the address never has to.
  assert.equal(event.params.country, 'BR');
  // Google drops an event with no engagement time.
  assert.equal(event.params.engagement_time_msec, '1');
});

test('only the fields the endpoint knows reach Google', () => {
  const payload = payloadFor(
    { ...OPEN, project: '/Users/someone/house.newera', mode: 'nonsense', architecture: 'z80' },
    'not a country',
  );
  const { params } = payload.events[0];
  assert.equal(params.project, undefined, 'a field nobody asked for');
  assert.equal(params.mode, undefined, 'a value outside the list');
  assert.equal(params.architecture, undefined, 'the same');
  assert.equal(params.country, undefined, 'and a country that is not one');
});

test('an answer to the request for support is counted with its action', () => {
  const payload = payloadFor({ event: 'support_prompt', client_id: 'abc123', action: 'coffee' }, 'BR');
  assert.equal(payload.events[0].name, 'support_prompt');
  assert.equal(payload.events[0].params.action, 'coffee');
  const odd = payloadFor({ event: 'support_prompt', client_id: 'abc123', action: 'paid' }, 'BR');
  assert.equal(odd.events[0].params.action, undefined, 'a value outside the list');
});

test('an event nobody counts, and an id that is not one, are refused', () => {
  assert.equal(payloadFor({ event: 'bought_something', client_id: 'abc' }, 'BR'), null);
  assert.equal(payloadFor({ event: 'app_open', client_id: 'a b c' }, 'BR'), null);
  assert.equal(payloadFor({ event: 'app_open' }, 'BR'), null);
  assert.equal(payloadFor('app_open', 'BR'), null);
});

test('the count is forwarded with the secret the site holds', async () => {
  const { answer, calls } = await sent(count, {
    request: posting(OPEN),
    env: { NEWERA_GA_API_SECRET: 'a-secret' },
  });
  assert.equal(answer.status, 204);
  assert.equal(calls.length, 1);
  assert.match(calls[0].url, /^https:\/\/www\.google-analytics\.com\/mp\/collect\?/);
  assert.match(calls[0].url, /measurement_id=G-PLY5GQC6EP/);
  assert.match(calls[0].url, /api_secret=a-secret/);
  assert.equal(JSON.parse(calls[0].options.body).events[0].name, 'app_open');
});

test('without a secret nothing is sent, and nothing fails', async () => {
  const { answer, calls } = await sent(count, { request: posting(OPEN), env: {} });
  assert.equal(answer.status, 204);
  assert.equal(calls.length, 0);
});

test('a body that is not an event is a bad request, not a count', async () => {
  const bad = new Request('https://3dneweraai.com/ping', { method: 'POST', body: 'hello' });
  const { answer, calls } = await sent(count, { request: bad, env: { NEWERA_GA_API_SECRET: 's' } });
  assert.equal(answer.status, 400);
  assert.equal(calls.length, 0);
});

test('a DSN says where an envelope goes and how it is signed', () => {
  const target = ingest('https://0123456789abcdef@o4508.ingest.sentry.io/4509');
  assert.equal(target.url, 'https://o4508.ingest.sentry.io/api/4509/envelope/');
  assert.match(target.auth, /sentry_key=0123456789abcdef/);
  assert.match(target.auth, /sentry_version=7/);
  assert.equal(ingest('not a dsn'), null);
  assert.equal(ingest('https://o4508.ingest.sentry.io/4509'), null, 'no key');
  assert.equal(ingest('https://key@o4508.ingest.sentry.io/'), null, 'no project');
});

test("the placeholder DSN of a key-less build is taken out of the envelope", () => {
  const envelope = `{"event_id":"abc","dsn":"https://tunnel@3dneweraai.com/0"}\n{"type":"event"}\n{}`;
  const kept = new TextDecoder().decode(withoutDsn(new TextEncoder().encode(envelope)));
  assert.equal(kept, '{"event_id":"abc"}\n{"type":"event"}\n{}');
  // Nothing to take out: the bytes pass through untouched.
  const plain = '{"event_id":"abc"}\n{"type":"event"}\n{}';
  assert.equal(new TextDecoder().decode(withoutDsn(new TextEncoder().encode(plain))), plain);
  assert.equal(withoutDsn(new TextEncoder().encode('not an envelope')), null);
});

test('a crash report is signed with the DSN the site holds', async () => {
  const envelope = '{"event_id":"abc"}\n{"type":"event"}\n{}';
  const request = new Request('https://3dneweraai.com/ping/sentry', {
    method: 'POST',
    body: envelope,
  });
  const { answer, calls } = await sent(crash, {
    request,
    env: { NEWERA_SENTRY_DSN: 'https://thekey@o1.ingest.sentry.io/2' },
  });
  assert.equal(answer.status, 204);
  assert.equal(calls[0].url, 'https://o1.ingest.sentry.io/api/2/envelope/');
  assert.match(calls[0].options.headers['x-sentry-auth'], /sentry_key=thekey/);
  assert.equal(new TextDecoder().decode(calls[0].options.body), envelope);
});

test('a crash report with nowhere to go is accepted and dropped', async () => {
  const request = new Request('https://3dneweraai.com/ping/sentry', { method: 'POST', body: 'x' });
  const { answer, calls } = await sent(crash, { request, env: {} });
  assert.equal(answer.status, 204);
  assert.equal(calls.length, 0);
});
