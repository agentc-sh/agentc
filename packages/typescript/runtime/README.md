# agentc runtime

Typescript declarations for the runtime libraries the agentc host provides to guest code.

Any agentc component written in Typescript runs inside an embedded QuickJS engine, and the environment
it sees is assembled by the host rather than by Node. This package describes that environment: the
`agentc:*` modules, the Node-compatible modules the host binds, and the globals it installs. It is a
set of declarations and nothing else, so it adds no runtime weight and has no dependencies of its own.

Installing it means the editor reports what the host actually provides. Without it, `agentc:*` imports
do not resolve at all, and the globals resolve against whatever other declarations happen to be
installed, which describe a runtime the component is not running in.

## Installation

This package is not published to npm. Add it directly from git, pinned to a release tag.

The two package managers spell a git subdirectory differently, so use the line that matches yours:

```bash
pnpm add github:agentc-sh/agentc#0.3.1&path:/packages/typescript/runtime
```

```bash
npm install github:agentc-sh/agentc#0.3.1::path:/packages/typescript/runtime
```

npm 11.10.0 or later is required for the `path:` fragment to be honoured. An older npm ignores it
silently and installs the repository root instead, so the symptom is a `node_modules/@agentc-sh/runtime`
directory containing the whole agentc repository rather than a package.

## Quick start

A component scaffolded by the agentc CLI already depends on this package and already references it, so
there is nothing to do. To wire it up by hand, add one line to the top of your entrypoint file:

```ts
/// <reference types="@agentc-sh/runtime" />
```

Every declaration is then in scope. There is nothing to import from this package and nothing to
configure per library.

## What is declared

Everything lives under `src/`, one file per surface. `globals.d.ts` holds every ambient global the host
installs, from every surface, so that the answer to "what globals exist?" is one file rather than a
search. Each `agentc-*.d.ts` beside it declares the modules of a single surface. `index.d.ts` is the
entry point and references them all. Those files are the reference for what each library exposes.

## A note on `@types/node`

This package replaces `@types/node` rather than supplementing it, and the two should not be installed
together.

The runtime is not a Node process. `@types/node` describes a great deal that the host never binds, so
with it installed the editor will accept code that fails at runtime, which is the problem this package
exists to remove.

## Maintenance

These declarations are written and maintained by hand. The runtime surface is type-erased on the Rust
side and cannot be reflected, so nothing generates or verifies them. Whenever a host module gains,
loses, or changes an export, the matching `.d.ts` in `src/` must be edited in the same change.

## License

The code is distributed under the MIT License. See `LICENSE` for more information.
