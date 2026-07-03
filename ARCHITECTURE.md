# agentc Architecture

## What agentc is

`agentc` is a compiler for production-grade LLM agents.

Users describe an agent in `agent.acl`, and `agentc` turns that manifest into a deployable artifact for a selected archetype. Today the primary archetype produces a standalone binary. That generated binary uses the `agentc` runtime crates for execution, persistence, serving, and integration, but the compiler is the primary product surface.

The project is therefore best understood as two connected parts:

- A **compiler** that reads a manifest and produces a deployable agent artifact
- A **runtime** that provides the execution substrate embedded into generated artifacts

This split is intentional. The compiler decides what gets built. The runtime decides how the built agent executes.

## Architecture in one picture

```text
┌─────────────────────────────────────────────────────────────┐
│                        Compiler Side                        │
│                                                             │
│   agent.acl                                                 │
│      │                                                      │
│      ▼                                                      │
│   parse + resolve configuration                             │
│      │                                                      │
│      ▼                                                      │
│   gather tools, skills, assets, and build context           │
│      │                                                      │
│      ▼                                                      │
│   generate an archetype-specific Rust project               │
│      │                                                      │
│      ▼                                                      │
│   compile the generated project into a deployable artifact  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                        Runtime Side                         │
│                                                             │
│   generated artifact                                        │
│      │                                                      │
│      ▼                                                      │
│   graph execution + checkpointing                           │
│   model + prompt services                                   │
│   tools + skills + external integrations                    │
│   persistence + state recovery                              │
│   HTTP serving + streaming protocols                        │
│   configuration + observability                             │
└─────────────────────────────────────────────────────────────┘
```

## Compiler-first design

The compiler is the entry point for users of the project.

At a high level, the compiler is responsible for:

1. Reading and validating the declarative manifest
2. Resolving build-time structure and runtime-configurable values
3. Preparing referenced inputs such as tools, skills, and other assets
4. Selecting an archetype and generating a complete project for it
5. Compiling that generated project into the final artifact

The important architectural boundary is that the compiler does not directly act as the agent runtime. Instead, it produces a normal build artifact whose execution behavior comes from the runtime layers that were wired into the generated project.

This matters because it keeps generation concerns separate from execution concerns:

- Compiler changes can add new artifact shapes without redesigning the runtime
- Runtime changes can improve execution behavior without changing the compiler model
- Generated output remains inspectable and debuggable as an ordinary project

## The manifest as the system boundary

`agent.acl` is the contract between the user and the compiler.

The manifest describes stable intent, not low-level implementation details. In broad terms, it defines:

- What agent is being built
- Which archetype should be targeted
- Which model providers and default model behavior are available
- Which tools and skills are exposed
- Which runtime and server capabilities should exist in the generated artifact
- Which parts of configuration are fixed at build time versus supplied at runtime

The manifest is deliberately higher level than the runtime crates. Users describe the agent they want. The compiler resolves that description into generated code and runtime wiring.

## Runtime responsibilities

The runtime exists to support the artifacts produced by the compiler.

The current runtime architecture has several stable responsibility areas:

### Graph execution

Agents execute as graph-driven workflows. A graph defines the execution loop: how input is received, how model calls and tool calls are sequenced, how state changes, and how a run completes.

Durability is a core runtime property. Execution state is checkpointed so an agent can recover from process failure or restart without losing its place.

### Persistence and recovery

The runtime persists sessions, runs, state, and related execution records through repository and storage layers. The graph engine is kept separate from storage details so execution behavior and persistence behavior can evolve independently.

### Model and prompt services

Model access, prompt rendering, prompt variables, token-aware behavior, and related context management are separate runtime concerns. They support graph implementations rather than being hard-coded into a single loop.

### Tools, skills, and integrations

Tools and skills are first-class runtime concepts.

Tools are executable capabilities the agent can invoke while running. Skills package reusable behavior and operational constraints around tool use and related context. External integration surfaces, such as model-context-protocol-style tool connections, are treated as modular adapters rather than as hard-coded assumptions of the core graph engine.

### Serving and protocols

Generated agents can expose HTTP interfaces and streaming protocols. These transport concerns are runtime services layered around the agent execution core rather than definitions of the core itself.

### Configuration and observability

Runtime configuration, secret handling, telemetry, tracing, logging, and similar operational concerns are part of the runtime support system. They are necessary for deploying generated agents in real environments, but they remain separate from the compiler pipeline and separate from graph logic.

## Composability is a first-class architectural goal

The system is designed so major pieces can evolve without forcing a rewrite of the whole stack.

That composability shows up in several directions:

- **Archetypes** define different output forms for the compiler
- **Graph runtimes** define different execution loops for generated agents
- **Tools** define executable capabilities available to an agent
- **Skills** define reusable capability bundles and behavior scaffolding
- **Providers and model layers** define how LLM access is supplied
- **Protocols and serving layers** define how external systems interact with the running agent

This separation is not accidental. It exists so `agentc` can support more deployment targets, more runtime models, and more integration styles over time without turning the project into one fixed stack.

## Extension model

There are two primary ways the project grows.

### New archetypes extend the compiler

An archetype is a blueprint for a deployable output. Adding a new archetype means teaching the compiler how to generate and build a different kind of artifact while preserving the same high-level manifest model.

That allows the project to target different environments without changing what `agentc` fundamentally is: a compiler from declarative agent definition to deployable artifact.

### New graph runtimes extend the generated runtime

A graph runtime defines the agent’s execution behavior. Adding a new graph runtime means introducing a different durable control loop while still reusing the surrounding runtime services where appropriate.

That allows the project to support multiple agent interaction models without coupling the entire system to a single implementation style.

## Dependency direction

The most important dependency rule is conceptual rather than crate-by-crate:

- The compiler depends on compiler-side abstractions for parsing, resolution, generation, and build orchestration
- Generated artifacts depend on runtime-side abstractions for execution and operations
- Runtime execution logic should not be tightly coupled to any single storage backend, tool backend, protocol, or graph flavor

In other words, generation should stay separate from execution, and execution should stay modular internally.

## What should stay stable over time

This document is intentionally written around statements that should remain true even as individual crates and modules change:

- `agentc` is primarily a compiler
- The compiler turns `agent.acl` into a deployable artifact
- Generated artifacts rely on shared runtime layers
- Graph execution is durable and stateful
- Tools, skills, providers, protocols, and serving layers are modular concerns
- Archetypes and graph runtimes are independent extension axes
- Composability is a core design principle, not an implementation accident

As the project evolves, those ideas should remain useful even if internal pipeline names, crate boundaries, or specific integrations continue to change.
