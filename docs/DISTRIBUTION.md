# Distribution

How 3D New Era AI reaches people: the ways to run it, the channels it is
published on, and what a release updates on its own.

## One tool surface, three places to run it

Every path calls the same `newera-mcp` crate: the same tools, the same
annotations and the same in-chat plan viewer. The AI client cannot tell them
apart.

```text
 AI clients: Claude · ChatGPT · Codex · Cursor · VS Code · Gemini CLI · LM Studio · Zed ...
        │                                           │
        │ local: .mcpb, stdio, 127.0.0.1:7878       │ hosted: https://mcp.3dneweraai.com/mcp
        │ (free, no account, nothing leaves         │ (OAuth; free, no limits,
        │  the machine)                             │  nothing sold)
        ▼                                           ▼
 ┌──────────────────┐        ┌──────────────────────── newera-cloud ────────────────────────┐
 │ Desktop app      │        │ accounts + OAuth 2.1 (CIMD and DCR)   plans (no limits)      │
 │ `newera`         │        │                                                              │
 └──────────────────┘        │ per-account session router:                                  │
                             │   ├─ the user's editor tab is open → relay → that tab        │
                             │   └─ otherwise → headless engine → cloud project (Postgres)  │
                             │                                                              │
                             │ anonymous link, no account: /r/<room>/<token>/mcp            │
                             └──────────────────────────────────────────────────────────────┘
```

- **Desktop:** `newera` serves MCP over HTTP on `127.0.0.1:7878/mcp`. `newera mcp`
  (stdio) attaches to the open window, or runs headless when there is none.
- **Browser editor:** `crates/newera-editor-web`, served at
  [3dneweraai.com/app](https://3dneweraai.com/app/). A tab can join the relay and
  give an AI client a URL of its own.
- **Hosted:** `crates/newera-cloud` mounts the relay (`crates/newera-relay`) and
  adds accounts, OAuth, cloud projects and a job queue for heavy renders. It
  exposes the same tools minus the ones that only make sense on the
  user's machine (`feedback`, `plugins`/`run_plugin`, and free file paths, which
  map to the account's own projects and 24-hour download links).

### Commitments

1. The desktop app and the browser editor are free, open source and need no
   account.
2. The cloud is optional. A cloud project is the same `.newera` file as on the
   desktop and can be downloaded at any time.
3. There is one tool surface. No channel gets a fork of `newera-mcp`.
4. Nothing is sold. Everything is free, and anything that runs on the user's
   machine stays free.

## Channels

| Channel | Artifact | Updated by |
|---|---|---|
| GitHub Releases | installers, archives, `newera-mcp.mcpb` | `release.yml` on a `v*` tag |
| [Official MCP Registry](https://registry.modelcontextprotocol.io) (`com.3dneweraai/newera`) | [`server.json`](../server.json): the `.mcpb` package and the hosted remote | `release.yml` job `registry` ([`scripts/registry-publish.sh`](../scripts/registry-publish.sh)) |
| Registry mirrors: VS Code / GitHub MCP registry, PulseMCP, JetBrains, Zed | — | read from the Official Registry |
| Claude Desktop extensions | `.mcpb` | manual submission |
| Claude Code and Codex plugin marketplaces | [`plugin/`](../plugin) (hosted MCP), [`plugin-desktop/`](../plugin-desktop) (local app), [`.claude-plugin/`](../.claude-plugin), [`.agents/plugins/`](../.agents/plugins) | the repository itself |
| Cursor Marketplace | [`.cursor-plugin/marketplace.json`](../.cursor-plugin/marketplace.json) | the repository itself |
| Gemini CLI | [`gemini-extension.json`](../gemini-extension.json) | the repository itself |
| [Glama](https://glama.ai/mcp/servers/leandrodaf/3d-new-era-ai) | [`glama.json`](../glama.json), [`Dockerfile`](../Dockerfile) | Glama rebuilds from the repository |
| [Smithery](https://smithery.ai/servers/leandro-daf4/new-era-3d) | the hosted URL | nothing to update |
| mcp.so, awesome-mcp-servers | listing | nothing to update |
| winget (`LeandroFerreira.3DNewEraAI`) | manifests in [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) | `release.yml` job `winget` (winget-releaser); [`scripts/winget.sh`](../scripts/winget.sh) wrote the first submission |
| Microsoft Store ([9MT55M4M7B76](https://apps.microsoft.com/detail/9MT55M4M7B76)) | MSIX (`newera-windows-x64.msix`), signed by the Store | `release.yml` jobs `msix` ([`scripts/msix.py`](../scripts/msix.py), [`packaging/msix/AppxManifest.xml`](../packaging/msix/AppxManifest.xml)) and `store` (Microsoft Store CLI; needs the `MSSTORE_*` secrets) |
| Snap Store / Ubuntu App Center (`newera`) | snap around the Linux build ([`snap/snapcraft.yaml`](../snap/snapcraft.yaml)); snapd updates installs on its own | `release.yml` jobs `snap` and `snapstore` (stable channel; needs `SNAPCRAFT_STORE_CREDENTIALS`); [`snap.yml`](../.github/workflows/snap.yml) installs and runs it on pull requests |
| Linux packages: `.deb`, `.rpm`, AppImage | `newera-linux-x64.deb`, `newera-linux-x64.rpm`, `newera-linux-x64.AppImage` on the GitHub release, around the Linux build | `release.yml` job `linux-packages` ([`scripts/linux-packages.sh`](../scripts/linux-packages.sh)); [`linux-packages.yml`](../.github/workflows/linux-packages.yml) installs each on pull requests |
| Flathub (`io.github.leandrodaf.newera`): GNOME Software, KDE Discover | Flatpak built from source, offline ([`packaging/flatpak/`](../packaging/flatpak)) | Flathub's own build from its repository; [`flatpak.yml`](../.github/workflows/flatpak.yml) builds and runs it here on pull requests |
| Homebrew (`leandrodaf/tap/3d-new-era-ai`) | cask in [leandrodaf/homebrew-tap](https://github.com/leandrodaf/homebrew-tap) | `release.yml` job `homebrew` ([`scripts/homebrew.sh`](../scripts/homebrew.sh)) |
| One-click install buttons (Cursor, VS Code, LM Studio) | links in the README and on the site | by hand when the URL changes |
| Claude and ChatGPT custom connectors | `https://mcp.3dneweraai.com/mcp` | `deploy-vps.yml` on a release tag |

Reviewer material for the connector directories (descriptions, example prompts,
test cases, screenshots) is in [CONNECTOR-REVIEW.md](CONNECTOR-REVIEW.md).

### Directory rules the code keeps

These are enforced by tests in `crates/newera-mcp`, so a new tool that breaks
them fails CI:

- every tool has a `title` and correct `readOnlyHint`, `destructiveHint` and
  `openWorldHint` annotations (`every_tool_has_hints`);
- reads never change the document (`reads_change_nothing`), and a read refuses
  write arguments (`a_read_refuses_a_write_argument`);
- descriptions do not give orders to the model or advertise
  (`descriptions_neither_order_nor_sell`);
- errors say what to do next;
- hosted tools declare their OAuth scope in `securitySchemes`
  (`every_hosted_tool_asks_for_the_account`).

Inside an AI client nothing is sold: no prices, no upgrade buttons and no
checkout links in tool output.

## Release flow

1. Bump the version in `Cargo.toml`, then run `cargo update -w` and
   [`scripts/sync-version.sh`](../scripts/sync-version.sh), which carries it into
   `server.json`, the plugin manifests and `gemini-extension.json` (CI runs it with
   `--check`).
2. Add the `CHANGELOG.md` entry and its compare link.
3. Commit `Release X.Y.Z` and push a signed tag `vX.Y.Z`.

The tag triggers:

- `release.yml`: builds every platform, packs the `.mcpb`, publishes the GitHub
  release, the MCP Registry entry, the winget PR and the Homebrew cask;
- `deploy-vps.yml`: the `newera-cloud` image, rolled out with a health check.

The site and the browser editor (`publish.yml`, `pages.yml`) deploy from every
push to `main` that touches them.

## What the site counts

Two endpoints live with the site, in [`functions/`](../functions), as Cloudflare Pages
Functions:

| Route | What it does |
|---|---|
| `/ping` | One usage event — `app_open` from the app, `install` and `uninstall` from the installers — forwarded to Google Analytics. |
| `/ping/sentry` | A crash envelope from a build with no DSN, signed with the one the site holds and forwarded to Sentry. |

They exist because of where a secret can live. Baked into a binary, a key only reaches the
builds CI makes: everyone who compiles the app themselves counted for nothing, and a release
built without the secret counted nothing at all. Here each key is one environment variable of
the Pages project — `NEWERA_GA_API_SECRET` and `NEWERA_SENTRY_DSN` — and every build reports
the same way.

What the endpoints accept is a closed list: three event names, a handful of parameters, each
one capped, and the payload Google sees is built on this side rather than forwarded as it
arrived. The visitor's address is not passed on; the country Cloudflare already knows from the
connection goes instead. A debug build and CI count nothing.

The installers draw the installation id the app then uses, so an install and the first run are
one installation in the count and not two. `node --test scripts/ping.test.mjs` tests both
endpoints; `NEWERA_PING_URL` and `NEWERA_SENTRY_TUNNEL` point a build at a local
`wrangler pages dev`.

## Hosted service

`newera-cloud` runs as a single container with Postgres. To run it locally with
its own database:

```sh
docker compose -f crates/newera-cloud/local/docker-compose.yml up --build
```

Sign-in links are printed to the log. Every setting is documented in
[`crates/newera-cloud/local/app.env.example`](../crates/newera-cloud/local/app.env.example).

Production runs on the maintainer's VPS. [`app.onboard`](../app.onboard) describes it
to the provisioning tool, which generates [`deploy/`](../deploy) and
[`.github/workflows/deploy-vps.yml`](../.github/workflows/deploy-vps.yml). Those files are
rewritten on every provisioning run, so change the manifest or the tool's templates rather
than editing them by hand.

- **Accounts:** our own account id; identity is a verified email (magic link or
  Google).
- **OAuth 2.1:** PKCE S256, client registration by CIMD and DCR, protected
  resource metadata (RFC 9728/8414), refresh rotation with reuse detection.
- **Limits:** none. The `plans` rows (`free`, `supporter`) still exist and tools
  still read a plan's limits, but every plan's `limits` is empty
  (migration `0005_no_limits.sql`), and a limit that is not written is no limit.
  A limit can come back as data, without code, if the server ever needs one.
  Heavy renders go through a queue: one per account, at most one per free core.
- **Storage:** the whole `.newera` file in Postgres (`bytea`) behind a storage
  layer, so it can move to object storage without touching the tools.
- **Sandboxing:** project file reads are jailed (`newera_core::vfs::jail`); a
  project cannot read another project's files or the server's.

### Donations

Nothing is sold: there is no paid plan, no checkout and no billing code. Supporting
the project is a donation on [Buy Me a Coffee](https://buymeacoffee.com/leandrodaf),
which gives nothing in return and is not tied to any account. The site explains it
at [support](https://3dneweraai.com/pricing/), next to the
[refunds](https://3dneweraai.com/refund/), [terms](https://3dneweraai.com/terms/)
and [privacy](https://3dneweraai.com/privacy/) pages.
