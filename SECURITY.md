# Security policy

## Reporting a vulnerability

Please report security issues privately, not in a public issue:

- through GitHub: **Security → Report a vulnerability** on this repository
  ([private vulnerability reporting](https://github.com/leandrodaf/3d-new-era-ai/security/advisories/new)), or
- by email to **<contato@3dneweraai.com>**.

Include what is affected (desktop app, browser editor, relay or the hosted service at
`mcp.3dneweraai.com`), the version, and the steps to reproduce. You will get an answer
as soon as possible, and a fix is released as soon as it is ready, with credit to you in the
changelog unless you prefer otherwise.

## Supported versions

Only the latest release receives fixes. The hosted service always runs the latest
release.

## Scope

In scope: the `newera` binary and its local HTTP/MCP server, the browser editor, the relay
and the hosted service (accounts, OAuth, cloud projects).

Out of scope: denial of service by volume, findings that need a compromised machine, and
reports from automated scanners without a working proof of concept.

## How the project limits exposure

- The local server binds to `127.0.0.1`, validates the `Host` header against DNS
  rebinding, and refuses to listen beyond loopback without a token.
- The relay stores no project data; it forwards messages between a tab and an AI client.
- The hosted service keeps secrets only as hashes and jails each project's file reads.

More in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#security).
