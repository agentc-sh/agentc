# agentc runtime

Python type stubs for the runtime libraries the agentc host provides to guest code.

Any agentc component written in Python runs inside RustPython or an embedded CPython, and the
host binds the modules it provides into that interpreter at startup. Those modules do not exist
on disk, so an editor or type checker cannot find them. This package describes them. It is a
set of declarations and nothing else, so it adds no runtime weight and has no dependencies of
its own.

Installing it means the editor reports what the host actually provides. Without it, imports of
the host modules do not resolve at all, and everything imported from them is untyped.

## Installation

This package is not published to PyPI. Add it directly from git as a development dependency,
pinned to a release tag:

```bash
uv add --dev "agentc-runtime @ git+https://github.com/agentc-sh/agentc@0.3.1#subdirectory=packages/python/runtime"
```

With pip, install the same requirement:

```bash
pip install "agentc-runtime @ git+https://github.com/agentc-sh/agentc@0.3.1#subdirectory=packages/python/runtime"
```

## Quick start

A component scaffolded by the agentc CLI already depends on this package in its `dev` group, so
there is nothing to do. To wire it up by hand, add the development dependency above.

Every host module is then known to the editor and the type checker. There is nothing to import
from this package and nothing to configure per library.

## What is declared

Everything lives under `src/`, one directory per host module, named after the module with a
`-stubs` suffix. Type checkers find a module's stubs by that name. Those directories are the
reference for what each library exposes:

- `agentc_tools-stubs/` declares `agentc_tools`, the tool contract.

## Outside the host

The package holds declarations only. In a normal Python interpreter, `import agentc_tools` still
raises `ModuleNotFoundError`, because the modules exist only inside the agentc runtime.

## Maintenance

These stubs are written and maintained by hand. Whenever a Python host module gains, loses, or
changes an export, the matching `.pyi` in `src/` must be edited in the same change.

`uv run pyright` checks the stubs in strict mode, and `uv run ruff format --check` and
`uv run ruff check` check their formatting and lint.

## License

The code is distributed under the MIT License. See `LICENSE` for more information.
