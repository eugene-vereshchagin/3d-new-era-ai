// The usage count, on the site's own domain: https://3dneweraai.com/ping
//
// The app and the installers post one small JSON object here and this forwards
// it to Google Analytics through the Measurement Protocol. The reason for the
// hop is the api secret: baked into a binary it only reaches the builds CI
// makes, so everyone who installs from source counts for nothing. Here it is
// one environment variable of the Pages project, and every build reports the
// same way.
//
// What is accepted is a closed list — four event names, a handful of
// parameters, each capped — and the payload Google sees is built here from
// those fields alone, never forwarded as it arrived. The visitor's address is
// not passed on: the country Cloudflare already knows goes in as a parameter,
// so the ping says where without saying who.
//
// Environment (Pages project, Production and Preview):
//   NEWERA_GA_API_SECRET   Measurement Protocol secret for the data stream
//   NEWERA_GA_MEASUREMENT_ID   optional, defaults to the property below

/** The property the site and the app share. */
const MEASUREMENT_ID = 'G-PLY5GQC6EP';

const COLLECT = 'https://www.google-analytics.com/mp/collect';
/** Google's validator: answers with what it thinks of the payload. */
const DEBUG_COLLECT = 'https://www.google-analytics.com/debug/mp/collect';

/** The largest body worth reading: every real ping is a few hundred bytes. */
const MAX_BODY = 2048;

/** What may be counted. An unknown name is a bad request, not a new event. */
const EVENTS = new Set(['app_open', 'install', 'uninstall', 'support_prompt']);

/**
 * The parameters that reach Google, and the values each one may take. A list
 * means those values only; `true` means any short token.
 */
const PARAMS = {
  app_version: true,
  operating_system: ['macos', 'windows', 'linux'],
  architecture: ['aarch64', 'x86_64'],
  mode: ['gui', 'serve', 'mcp', 'cli'],
  channel: ['release', 'source'],
  installer: ['script', 'brew', 'winget', 'mcpb'],
  first_install: ['0', '1'],
  // What was done with the request for support the app shows at launch.
  action: ['shown', 'coffee', 'github', 'dismiss'],
};

/** An installation id: what the app keeps beside its settings, nothing more. */
const CLIENT_ID = /^[A-Za-z0-9_-]{1,64}$/;

/** A parameter value: a short token, no spaces, nothing to escape downstream. */
const VALUE = /^[A-Za-z0-9_.:+-]{1,64}$/;

/**
 * The Measurement Protocol payload for one accepted ping, or `null` when the
 * ping is not one. Built field by field: nothing the caller sent is passed
 * through, so a new key in the body cannot become a new dimension by accident.
 */
export function payloadFor(body, country) {
  if (!body || typeof body !== 'object') return null;
  const { event, client_id: client } = body;
  if (!EVENTS.has(event) || typeof client !== 'string' || !CLIENT_ID.test(client)) return null;

  const params = {
    // Google drops an event with no engagement time: it counts as a session
    // that never happened.
    engagement_time_msec: '1',
    session_id: client,
  };
  for (const [name, allowed] of Object.entries(PARAMS)) {
    const value = body[name];
    if (value === undefined || value === null) continue;
    const text = String(value).trim();
    if (!VALUE.test(text)) continue;
    if (allowed === true || allowed.includes(text)) params[name] = text;
  }
  // Cloudflare knows the country from the connection; the address itself stays
  // with Cloudflare and is never sent on.
  if (typeof country === 'string' && /^[A-Z]{2}$/.test(country)) params.country = country;

  return {
    client_id: client,
    non_personalized_ads: true,
    events: [{ name: event, params }],
  };
}

export async function onRequestPost({ request, env }) {
  const secret = (env.NEWERA_GA_API_SECRET || '').trim();
  const measurement = (env.NEWERA_GA_MEASUREMENT_ID || MEASUREMENT_ID).trim();
  // Nothing configured: the endpoint exists and counts nothing, which is what
  // a preview deployment should do.
  if (!secret) return new Response(null, { status: 204 });

  const length = Number(request.headers.get('content-length') || 0);
  if (length > MAX_BODY) return new Response('too large', { status: 413 });

  let body;
  try {
    const text = await request.text();
    if (text.length > MAX_BODY) return new Response('too large', { status: 413 });
    body = JSON.parse(text);
  } catch {
    return new Response('not json', { status: 400 });
  }

  const payload = payloadFor(body, request.cf?.country);
  if (!payload) return new Response('not an event', { status: 400 });

  const debug = new URL(request.url).searchParams.has('debug');
  const target = `${debug ? DEBUG_COLLECT : COLLECT}?measurement_id=${encodeURIComponent(measurement)}&api_secret=${encodeURIComponent(secret)}`;
  const answer = await fetch(target, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });

  // The validator's answer is the whole point of asking it; the real collector
  // says nothing worth passing back.
  if (debug) {
    return new Response(await answer.text(), {
      status: answer.status,
      headers: { 'content-type': 'application/json' },
    });
  }
  return new Response(null, { status: answer.ok ? 204 : 502 });
}
