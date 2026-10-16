# Changelog

---
## [0.4.0](https://github.com/agentc-sh/agentc/compare/0.3.1..0.4.0) - 2026-10-16

### Bug Fixes

- **(blocks)** Relax clippy requirements in standalone archetype generated output - ([9ecb901](https://github.com/agentc-sh/agentc/commit/9ecb901ca233096a2a7d883d0c30634dbbe47286)) - Timothy Pogue
- **(blocks)** Add filesystem components and thread typescript bindings to tools  - ([1537370](https://github.com/agentc-sh/agentc/commit/1537370dd1193955ab4f2ec3591c727c038ebe80)) - Timothy Pogue
- **(blocks)** Make generated runtime config component-driven  - ([e3066df](https://github.com/agentc-sh/agentc/commit/e3066df25589a3fc205dedd9c24492c5f504b264)) - Timothy Pogue
- **(blocks)** Expose the governed http client to typescript and build only what an agent uses  - ([7c0a0bb](https://github.com/agentc-sh/agentc/commit/7c0a0bb0452845bdd0d53e80cdd44530396c4215)) - Timothy Pogue
- **(cli)** Support specifying custom manifests and artifact directories  - ([55634d8](https://github.com/agentc-sh/agentc/commit/55634d83c9c88412963f8d9ee7a920c069ce5d55)) - Timothy Pogue
- **(core)** Include pnpm-workspace.yaml in javascript tool scaffold  - ([32bf72f](https://github.com/agentc-sh/agentc/commit/32bf72f7cd592e7dca3ea77797d12e71ce81d9b0)) - Timothy Pogue
- **(fs)** Rework mount machinery into concrete namespace and support runtime grafting  - ([8405c0c](https://github.com/agentc-sh/agentc/commit/8405c0c4267a6fe8fce33d0ac21b4a32dce10c03)) - Timothy Pogue
- **(fs)** Add typescript bindings for agentc-fs library to replace LLRT fs handling in typescript executor  - ([8aebad8](https://github.com/agentc-sh/agentc/commit/8aebad87d2b2b72634cec638f37f75595424a364)) - Timothy Pogue
- **(http)** Fix openapi extension trait document method to propagate tags  - ([7da878f](https://github.com/agentc-sh/agentc/commit/7da878fdd40199f4ac018438f0738bd0259a4846)) - Timothy Pogue
- **(react)** Fix tool call error events carrying incorrect message ID and add parent_message_id  - ([fdd111f](https://github.com/agentc-sh/agentc/commit/fdd111f63637b62b6f27e0cbd899971acd72ad1e)) - Timothy Pogue
- **(react)** Remove unnecessary validation requirement on start run DTO - ([81a2441](https://github.com/agentc-sh/agentc/commit/81a2441b09fc63fdbcb9fba20874398f9cae3c2d)) - Timothy Pogue
- **(react)** Fix invalid routing for various input and resumed states  - ([74eabc0](https://github.com/agentc-sh/agentc/commit/74eabc0eea161a3e621f7869434ef0d301985a42)) - Timothy Pogue
- **(tools)** Bump rustpython to ede18e4 for malachite pymath fix  - ([a4a90d4](https://github.com/agentc-sh/agentc/commit/a4a90d424a49daeedbdaf763514bf4f2cf3c6c20)) - Timothy Pogue
- Bump and migrate several dependencies and set msrv to 1.99  - ([93ed7a1](https://github.com/agentc-sh/agentc/commit/93ed7a19f2290cc024ef1beed8bd80bcdd818162)) - Timothy Pogue
- Rework custom manifest blocks handling and add archetypes docs reference  - ([f28c95a](https://github.com/agentc-sh/agentc/commit/f28c95a8432cb4283607e4038e67327f991548bf)) - Timothy Pogue
- Improve config handling and parsing at runtime and nested runtime variables in code generation  - ([39450c8](https://github.com/agentc-sh/agentc/commit/39450c8d9a39ae9f957ad0732e9497ac911149f9)) - Timothy Pogue
- Favor root user and set workdir in slim and toolchain docker images - ([190010b](https://github.com/agentc-sh/agentc/commit/190010b1322b891f5aa43525d9750d4b52195f29)) - Timothy Pogue
- Bump python to 3.14.8 in toolchain image - ([c712082](https://github.com/agentc-sh/agentc/commit/c712082b7f3e93b6cb4c37f345de05895fe60e97)) - Timothy Pogue
- Prioritize tool names from manifest blocks over exports in code tools  - ([d30ee9d](https://github.com/agentc-sh/agentc/commit/d30ee9d33a8265f43063c211b6959a07328bf025)) - Timothy Pogue
- Make context vars, tools, model config, and capabilities in run request apply per run instead of at session start  - ([1f91709](https://github.com/agentc-sh/agentc/commit/1f917092c501290124480e131a4eeb786251df91)) - Timothy Pogue
- Align AG-UI protocol machinery with 1.0 spec  - ([5622d46](https://github.com/agentc-sh/agentc/commit/5622d46f1696b4b53e0e786985d948cb0ac5d59f)) - Timothy Pogue
- Improve runtime value serialization structure ambiguity and improve manifest parsing errors  - ([0029045](https://github.com/agentc-sh/agentc/commit/0029045c880380afe813c1d3d41ee68b355ed35a)) - Timothy Pogue
- Bump jobq and migrate breaking changes - ([befc1cf](https://github.com/agentc-sh/agentc/commit/befc1cf0f7af79ff5ed20b4443c9bedf76aba587)) - Timothy Pogue
- Support reading files in HCL deserialization for string values  - ([277a2bc](https://github.com/agentc-sh/agentc/commit/277a2bc0dafdb857aa64a5a5f4ea24bc782c9249)) - Timothy Pogue
- Bump toolchain image dependencies - ([d21941e](https://github.com/agentc-sh/agentc/commit/d21941e83a7f3921007a6b7bfc04e395f06a7324)) - Timothy Pogue
- Add python runtime type stubs for improved developer experience  - ([4fb0109](https://github.com/agentc-sh/agentc/commit/4fb01097ae1c2f3b1147c0dbb25a282e987f6bb9)) - Timothy Pogue
- Bump guestjs to 0.4.0 release - ([1a6daf1](https://github.com/agentc-sh/agentc/commit/1a6daf1e8a67121acaedb84708e6aeb441dff679)) - Timothy Pogue
- Rework python tool machinery to use new executor API  - ([2db0904](https://github.com/agentc-sh/agentc/commit/2db09049eb11effad65a350f9d806f4e46ffa8b1)) - Timothy Pogue
- Rework skills to interop with virtual filesystem for unified view and fix small issues  - ([d7fc7ce](https://github.com/agentc-sh/agentc/commit/d7fc7cea5b0f00ee924dc6588835fc5438a7a92b)) - Timothy Pogue
- Expose filesystem and network blocks and runtime configs to control shape and policies  - ([e8df370](https://github.com/agentc-sh/agentc/commit/e8df370c5b099947f7ad7360e827457644b9a0c8)) - Timothy Pogue
- Add POSIX filesystem semantics to fs and host runtime support to typescript executor  - ([6e95304](https://github.com/agentc-sh/agentc/commit/6e953042e920864f2cb4f980170bfa193241e771)) - Timothy Pogue
- Expose types stub package for agentc runtime libraries in typescript and support pnpm as additional package manager  - ([e5479d7](https://github.com/agentc-sh/agentc/commit/e5479d7c53b8c39925b8efa55440e3e86ee2110c)) - Timothy Pogue
- Add policy controlled HTTP client for shared use in runtime with typescript bindings  - ([c18e0d7](https://github.com/agentc-sh/agentc/commit/c18e0d7151acabab3084cb966e9824b203aff461)) - Timothy Pogue
- Rework typescript execution to modular foundation and migrate JavascriptTool  - ([51755bd](https://github.com/agentc-sh/agentc/commit/51755bd771e44ccc4bbb6be6074fe9d1cc397f72)) - Timothy Pogue
- Export input and output information for model and tool calls in OTLP traces  - ([81d4fe6](https://github.com/agentc-sh/agentc/commit/81d4fe6949312a2adfb083518896a4294f0ec386)) - Timothy Pogue

### Features

- **(cli)** Add support for run command  - ([a221fec](https://github.com/agentc-sh/agentc/commit/a221fecdce16e22464b882f20085db905f608cc9)) - Timothy Pogue
- Add support for custom headers and invocation context for model calls  - ([81e736e](https://github.com/agentc-sh/agentc/commit/81e736ee6af48972d7fb5803686459001635fdc9)) - Timothy Pogue
-  [**breaking**]Separate telemetry variables to distinguish between which exporters to register  - ([c37e3b6](https://github.com/agentc-sh/agentc/commit/c37e3b62efeaf9563b9a71c00f0bf74d4ddc6c9f)) - Timothy Pogue
-  [**breaking**]Rework provider block in manifest and model crate to support separate provider identity from kind  - ([8dc94f6](https://github.com/agentc-sh/agentc/commit/8dc94f66a4641e70066305784607cd1935f286e9)) - Timothy Pogue
- Add datetime filters in prompt templates  - ([be418a0](https://github.com/agentc-sh/agentc/commit/be418a0186548188dff3fcbcb5d8417df27eef02)) - Timothy Pogue
- Add python bindings for agentc-fs machinery  - ([61360a5](https://github.com/agentc-sh/agentc/commit/61360a5078b731faa5233e80a06de4acb47fba07)) - Timothy Pogue
- Add python bindings for http library  - ([ad74c79](https://github.com/agentc-sh/agentc/commit/ad74c79480b23646fdd9c930df345d40ca2b65d4)) - Timothy Pogue
-  [**breaking**]Rework python tools to remove TDK dependency with new contract  - ([e942f3b](https://github.com/agentc-sh/agentc/commit/e942f3b579040c61285bb6976390f303a8fb3d60)) - Timothy Pogue
-  [**breaking**]Rework javascript tools to remove TDK dependency with new contract  - ([ac15479](https://github.com/agentc-sh/agentc/commit/ac15479487fe27c02b993b8d12c138348b6adc35)) - Timothy Pogue
- Add python executor machinery for supporting python components  - ([93be0d1](https://github.com/agentc-sh/agentc/commit/93be0d1462cc7e1615ddb0345c0a687705c586f7)) - Timothy Pogue
- Integrate bash tool with application wide filesystem and egress controls  - ([92e2304](https://github.com/agentc-sh/agentc/commit/92e23043abbd0982202d2a57e32a0333f190e5ce)) - Timothy Pogue
- Add filesystem abstraction crate for controlled virtual filesystems  - ([cc0bb4f](https://github.com/agentc-sh/agentc/commit/cc0bb4fbb4be27b8ee876acebd3662c237e4796a)) - Timothy Pogue
- Add support for runtime prompt sources  - ([c7b0f4c](https://github.com/agentc-sh/agentc/commit/c7b0f4c81911ebe88e5fc63123c9efd7b710c6e0)) - Timothy Pogue
- Add checkpoint time travel and branching to the ReAct graph  - ([0d622b0](https://github.com/agentc-sh/agentc/commit/0d622b089bb96e5cdd4012986d2ac011abee79f2)) - Timothy Pogue

### Miscellaneous

- **(blocks)** Refactor extract_all function into trait  - ([e0510db](https://github.com/agentc-sh/agentc/commit/e0510db9f2a17ed2c7d34f6c07bbed6e61645cde)) - Timothy Pogue
- **(docs)** Fix invalid jsx in ag-ui reference - ([0a4f035](https://github.com/agentc-sh/agentc/commit/0a4f03563eb76b19217ba4eb210bd3473af32a4d)) - Timothy Pogue
- **(docs)** Rework linked mdx linked components - ([8715999](https://github.com/agentc-sh/agentc/commit/87159991759c8ad5a39d00e0ace7d08e42af26cd)) - Timothy Pogue
- **(docs)** Remove unnecessary quotes on enum types in reference - ([85bcb74](https://github.com/agentc-sh/agentc/commit/85bcb7490dce698fa4c94c1ed4bd7e5835b332ea)) - Timothy Pogue
- **(docs)** Ensure related tab sections are grouped together for shared state - ([1e52071](https://github.com/agentc-sh/agentc/commit/1e52071c599ee43f9b670777e89deb8dbfd8eb42)) - Timothy Pogue
- **(docs)** Document the runtime surface, filesystem and http client support  - ([1169f16](https://github.com/agentc-sh/agentc/commit/1169f16f8cf9402d3c790181d61f995b0fa69f50)) - Timothy Pogue
- Update .gitignore - ([0a31a07](https://github.com/agentc-sh/agentc/commit/0a31a0754cb78b222c045cc7f0128d278ecfc2b5)) - Timothy Pogue
- Increase max commit message length to 130 - ([b3352b7](https://github.com/agentc-sh/agentc/commit/b3352b7689f8c51bdd6689d9339605ba64c99b08)) - Timothy Pogue
---
## [0.3.1](https://github.com/agentc-sh/agentc/compare/0.3.0..0.3.1) - 2026-07-25

### Bug Fixes

- **(compiler)** Fix symlink handling for windows builds  - ([6b8f39e](https://github.com/agentc-sh/agentc/commit/6b8f39e227e672e23073ea0a679ed67bc6ca2f96)) - Timothy Pogue
---
## [0.3.0](https://github.com/agentc-sh/agentc/compare/0.3.0-rc.17.a352e10..0.3.0) - 2026-07-25

### Miscellaneous

- Fix bump version task in version taskfile - ([1dc4863](https://github.com/agentc-sh/agentc/commit/1dc4863a2181fdf493406738c4d2c1b58d37c6ef)) - Timothy Pogue
- Fix current version task in version taskfile - ([4bb7be6](https://github.com/agentc-sh/agentc/commit/4bb7be6763845990da7ce5c7a42403600e181833)) - Timothy Pogue
---
## [0.3.0-rc.17.a352e10](https://github.com/agentc-sh/agentc/compare/0.3.0-rc.15.27d1eaa..0.3.0-rc.17.a352e10) - 2026-07-23

### Bug Fixes

- **(agent)** Ensure runs are properly finished when errored in graph runtime  - ([fc69785](https://github.com/agentc-sh/agentc/commit/fc697856329f3fb534a5503e3389efc6dc6bf44b)) - Timothy Pogue
- **(blocks)** Ensure config command in standalone archetype defaults to debug output for secrets  - ([91b254b](https://github.com/agentc-sh/agentc/commit/91b254bc5d71532d9c2d97aabcca3b5be463c7bb)) - Timothy Pogue
- Bump nodejs, pnpm, and npm in toolchain docker image  - ([a352e10](https://github.com/agentc-sh/agentc/commit/a352e10bfab0d0da6c0ff49fea1048b13ae85a19)) - Timothy Pogue
- Bump rust-python for libc build error and fix tool name handling for generated config  - ([70214d8](https://github.com/agentc-sh/agentc/commit/70214d880a49c907b54f77d61c34a05ed9ee69df)) - Timothy Pogue
- Ensure migrations are serial across replicas in production and provide method to disable automatic migrations  - ([c14345c](https://github.com/agentc-sh/agentc/commit/c14345cb7922ea1e7514c75bfa3833c6a3ecd809)) - Timothy Pogue
- Add max request size option for limiting incoming payloads in http layer  - ([ab3e146](https://github.com/agentc-sh/agentc/commit/ab3e146138cb9dbc2d81eec450e965245f92c976)) - Timothy Pogue

### Features

- Add support for static python interpreter via CPython  - ([563f51d](https://github.com/agentc-sh/agentc/commit/563f51d6e9d6f01031509b1e62a5896e128247ba)) - Timothy Pogue
-  [**breaking**]Add retries and timeouts to model calls and expose client params in ReAct graph  - ([8564570](https://github.com/agentc-sh/agentc/commit/856457074dfeabe686c99d4a7c5edfb2bc6fc4e1)) - Timothy Pogue
- Export semantic convention standard gen AI traces and metrics for observability platforms  - ([1550ae6](https://github.com/agentc-sh/agentc/commit/1550ae6db0e0d0c04caa22f3512fa97cb653f7ef)) - Timothy Pogue

### Miscellaneous

- Fix install script not decoding escaped SAN for signature verification - ([998757e](https://github.com/agentc-sh/agentc/commit/998757eb68ba0888893cac7719ac2b65f4825be9)) - Timothy Pogue
---
## [0.3.0-rc.15.27d1eaa](https://github.com/agentc-sh/agentc/compare/0.2.1..0.3.0-rc.15.27d1eaa) - 2026-07-12

### Bug Fixes

- Improve extension point and contributions system to support dynamic, structured contributions  - ([db7cea5](https://github.com/agentc-sh/agentc/commit/db7cea51e21cfdd3816d775362bd51deb0261bc3)) - Timothy Pogue
- Add support for huggingface inference router provider  - ([44e5460](https://github.com/agentc-sh/agentc/commit/44e5460d7c4200c9040454ce74687585978df300)) - Timothy Pogue

### Features

- Add tools for delegating tasks to subagents over the A2A protocol  - ([c730190](https://github.com/agentc-sh/agentc/commit/c7301905dfa625610d5aec08d65ad680ccbc9368)) - Timothy Pogue
- Add A2A protocol support on server side as optional addon  - ([8ac7298](https://github.com/agentc-sh/agentc/commit/8ac729834b77dcf80ac66401325c1a11803fc76a)) - Timothy Pogue
---
## [0.2.1](https://github.com/agentc-sh/agentc/compare/0.2.0..0.2.1) - 2026-07-08

### Bug Fixes

- Update slim and toolchain images and ensure grype only fails on fixed issues - ([4b081cc](https://github.com/agentc-sh/agentc/commit/4b081ccf7a51e730942d96f3720e156961bddda9)) - Timothy Pogue

### Features

- **(react)** Add support for starting detached runs  - ([e516ddf](https://github.com/agentc-sh/agentc/commit/e516ddf187a49edbb386dacd5f17b10f80eb5e9d)) - Timothy Pogue
---
## [0.2.0](https://github.com/agentc-sh/agentc/compare/0.1.0-rc.13.ed88495..0.2.0) - 2026-07-08

### Bug Fixes

- **(blocks)** Refactor standalone archetype to reorganize field and agent generation  - ([6a1d21b](https://github.com/agentc-sh/agentc/commit/6a1d21bb48b2f35a09de043fac0b41cef94f273e)) - Timothy Pogue
- **(blocks)** Small tweaks to shutdown signal in standalone archetype generated code - ([9d72c92](https://github.com/agentc-sh/agentc/commit/9d72c927766b570d8673e8583b9a954b5297ef6e)) - Timothy Pogue
- **(model)** Bump rig version - ([3144b6c](https://github.com/agentc-sh/agentc/commit/3144b6cd049a6ce346ca382f0fa947627c244c33)) - Timothy Pogue
- Add explicit handling for cancelling runs when connection is dropped in ReAct and AG-UI API layers  - ([8a06e21](https://github.com/agentc-sh/agentc/commit/8a06e21d828fbda85dbd34a398f7b589775168a2)) - Timothy Pogue
- Bump dependencies in Cargo.lock - ([479ca12](https://github.com/agentc-sh/agentc/commit/479ca126146efa2370004713f8fc5f07a066ecad)) - Timothy Pogue

### Features

- Add support for cancelling runs external of the running graph in runtime, harness, and ReAct service layer  - ([59b9c02](https://github.com/agentc-sh/agentc/commit/59b9c02f37ba2f67c8d39a0d7b01de750d9b03b5)) - Timothy Pogue
- Add support for selecting graph implementation in agent manifest  - ([f636844](https://github.com/agentc-sh/agentc/commit/f636844d6f6010a25333ac720f1767815cf81931)) - Timothy Pogue
- Add openrouter, xai, and gemini providers  - ([388751f](https://github.com/agentc-sh/agentc/commit/388751f52b6fd0fcb7fd119a3e7934781fbf0644)) - Timothy Pogue
- Add support for multimodal inputs in ReAct and AG-UI handling  - ([814d737](https://github.com/agentc-sh/agentc/commit/814d7379d427ae2d9f1235665bc4d8aabd3bd61d)) - Timothy Pogue

### Miscellaneous

- **(docs)** Add full documentation  - ([812ae5b](https://github.com/agentc-sh/agentc/commit/812ae5b43a19ffb7ec8f3076133c27363aaf3462)) - Timothy Pogue
- Update install script and add license notice for docs in README - ([59392f8](https://github.com/agentc-sh/agentc/commit/59392f87daf631cc0ae350a01ece8d07214226fc)) - Timothy Pogue
- Run formatting - ([3ae854d](https://github.com/agentc-sh/agentc/commit/3ae854da329aa663e76e2be6a1cf45747862cbd8)) - Timothy Pogue
- Update postCreateCommand in devcontainer manifest - ([8a2d529](https://github.com/agentc-sh/agentc/commit/8a2d529950b073e3b3acee2e08d2d5d558c62d96)) - Timothy Pogue
- Ensure signature verification skipped if no attestations available - ([7004c9f](https://github.com/agentc-sh/agentc/commit/7004c9faac981f2cd4ddc29acc388a87b2cfbff2)) - Timothy Pogue
- Make API_BASE overridable in install script - ([ee4b83f](https://github.com/agentc-sh/agentc/commit/ee4b83fe6fafaabf3bfcde73a44525c3854a6e16)) - Timothy Pogue
---
## [0.1.0-rc.13.ed88495] - 2026-07-03

### Features

- Initial project setup :tada: - ([dfaee2f](https://github.com/agentc-sh/agentc/commit/dfaee2f101787f14a7ac6b8d7d03110b756776cc)) - Timothy Pogue

### Miscellaneous

- Fix issue links in README - ([944887a](https://github.com/agentc-sh/agentc/commit/944887a977c619a5c8c92b8327f9c9f9cbdeb264)) - Timothy Pogue
- Fix gitignore to not catch templates - ([a6ad106](https://github.com/agentc-sh/agentc/commit/a6ad106612a9667dadd3309033d71721e1bd15d9)) - Timothy Pogue

