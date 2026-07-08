# Changelog

---
## [0.2.1](https://github.com/agentc-sh/agentc/compare/0.2.0..0.2.1) - 2026-07-08

### Bug Fixes

- Update slim and toolchain images and ensure grype only fails on fixed issues - ([4b081cc](https://github.com/agentc-sh/agentc/commit/4b081ccf7a51e730942d96f3720e156961bddda9)) - Timothy Pogue
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

