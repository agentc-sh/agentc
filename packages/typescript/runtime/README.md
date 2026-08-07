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

This package is not published to npm yet. Add it directly from git, pinned to a release tag.

The two package managers spell a git subdirectory differently, so use the line that matches yours:

```bash
pnpm add github:agentc-sh/agentc#0.3.1&path:/packages/typescript/runtime
```

```bash
npm install github:agentc-sh/agentc#0.3.1::path:/packages/typescript/runtime
```

pnpm joins the committish and the subdirectory with `&`, npm with `::`. There is no single string that
both parsers accept. Tags are bare semver with no `v` prefix.

npm 11.10.0 or later is required for the `path:` fragment to be honoured. An older npm ignores it
silently and installs the repository root instead, so the symptom is a `node_modules/@agentc-sh/runtime`
directory containing the whole agentc repository rather than a package.

## Quick start

A component scaffolded by the agentc CLI already depends on this package and already references it, so
there is nothing to do. To wire it up by hand, add one file under your source root:

```ts
/// <reference types="@agentc-sh/runtime" />
```

and set `types` to an empty array in `tsconfig.json`, so nothing else is pulled in uninvited:

```json
{
  "compilerOptions": {
    "types": []
  }
}
```

Every declaration is then in scope. There is nothing to import from this package and nothing to
configure per library.

## What is declared

Everything lives under `src/`, one file per surface. `globals.d.ts` holds every ambient global the host
installs, from every surface, so that the answer to "what globals exist?" is one file rather than a
search. Each `agentc-*.d.ts` beside it declares the modules of a single surface. `index.d.ts` is the
entry point and references them all.

Those files are the reference for what each library exposes. This README does not restate their
contents and does not list them, because it would then need editing every time a surface is added or
removed. To find out what is available, read `src/`, or hover the symbol in an editor.

Declarations are unconditional even where the underlying module is not. A module granted only under a
capability is still declared, because visibility cannot depend on a manifest that the type checker
never sees. Importing one without the capability fails at runtime, exactly as it did before this
package existed.

## A note on `@types/node`

This package replaces `@types/node` rather than supplementing it, and the two should not be installed
together.

The guest is not a Node process. `@types/node` describes a great deal that the host never binds, so
with it installed the editor will accept code that fails at runtime, which is the problem this package
exists to remove. That is what the `"types": []` setting above is for.

## Maintenance

These declarations are written and maintained by hand. The runtime surface is type-erased on the Rust
side and cannot be reflected, so nothing generates or verifies them. Whenever a host module gains,
loses, or changes an export, the matching `.d.ts` in `src/` must be edited in the same change.

## License

The code is distributed under the MIT License. See `LICENSE` for more information.
