<a id="readme-top"></a>

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/agentc-sh/agentc">
    <img src="https://agentc.sh/icon-256.png" alt="Logo" width="128" height="128">
  </a>

  <h3 align="center">agentc</h3>

  <p>
    <a href="https://github.com/agentc-sh/agentc/graphs/contributors">
      <img src="https://img.shields.io/github/contributors/agentc-sh/agentc.svg?style=for-the-badge" alt="Contributors">
    </a>
    <a href="https://github.com/agentc-sh/agentc/network/members">
      <img src="https://img.shields.io/github/forks/agentc-sh/agentc.svg?style=for-the-badge" alt="Forks">
    </a>
    <a href="https://github.com/agentc-sh/agentc/stargazers">
      <img src="https://img.shields.io/github/stars/agentc-sh/agentc.svg?style=for-the-badge" alt="Stars">
    </a>
    <a href="https://github.com/agentc-sh/agentc/issues">
      <img src="https://img.shields.io/github/issues/agentc-sh/agentc.svg?style=for-the-badge" alt="Issues">
    </a>
    <a href="https://github.com/agentc-sh/agentc/blob/master/LICENSE.txt">
      <img src="https://img.shields.io/github/license/agentc-sh/agentc.svg?style=for-the-badge" alt="License">
    </a>
    <img src="https://img.shields.io/badge/language-Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  </p>

  <p align="center">
    <code>agentc</code> is a compiler and runtime for building production ready LLM agents.
    <br />
    <a href="https://docs.agentc.sh"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/agentc-sh/agentc/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    &middot;
    <a href="https://github.com/agentc-sh/agentc/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>


<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

`agentc` is a compiler for building production-ready LLM agents from a declarative manifest. Instead of hand-wiring model clients, tool execution, checkpointing, persistence, and serving infrastructure in every project, you describe the agent you want in `agent.acl` and compile it into a deployable artifact.

Today, the primary output is a standalone binary. The generated artifact uses the `agentc` runtime to provide durable graph-based execution, model and prompt orchestration, tools and skills, persistent state recovery, and optional HTTP and streaming interfaces.

The project is designed to be composable. Archetypes define what kind of artifact gets produced, graph runtimes define how an agent executes, and tools, skills, providers, and protocols are modular layers around that core. The result is a system that is built for real deployment rather than one-off agent demos.

<!-- GETTING STARTED -->
## Getting Started

Install the `agentc` CLI and the local toolchains needed to compile agents for your target setup.

### Prerequisites

- Required:
  - [Rust toolchain](https://rustup.rs) via `rustup`. The `standalone` archetype compiles agents with `cargo`, so Rust must be installed on the build machine.
- Optional:
  - `pnpm` and `esbuild` if the agent uses JavaScript or TypeScript tools
  - `uv` if the agent uses Python tools

### Installation

Choose one installation method:

Install the latest `agentc` release locally:

```sh
curl -sSfL https://install.agentc.sh | bash
```

> [!NOTE]
> Before executing any script fetched from the internet, review its contents first.

> [!TIP]
> To inspect what the installer will do without making changes, run it in dry-run mode:
>
> ```sh
> curl -sSfL https://install.agentc.sh | bash -s -- --dry-run
> ```

Or use the Docker image directly instead of installing `agentc` locally:

```sh
docker run --rm -it \
  --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghcr.io/agentc-sh/agentc:toolchain \
  --version
```

> [!NOTE]
> The Docker image uses the `toolchain` variant so `agentc` can compile projects that require Rust, Node.js, `pnpm`, `esbuild`, `python`, and `uv`.

If you want a local `agentc` command that runs the Docker image for you, run:

```sh
mkdir -p ~/.local/bin && cat > ~/.local/bin/agentc <<'EOF'
#!/usr/bin/env bash
set -e

docker run --rm -it \
  --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghcr.io/agentc-sh/agentc:toolchain \
  "$@"
EOF
chmod +x ~/.local/bin/agentc
```

Verify the installation or Docker-backed command:

```sh
agentc --version
```

<!-- USAGE EXAMPLES -->
## Usage

The quickest way to try `agentc` is to scaffold a project and compile it into a standalone agent binary:

```sh
agentc init my-agent
cd my-agent
agentc build
```

This creates a minimal agent project, compiles it, and writes the generated binary to `artifacts/build/`.

To run the agent for a single prompt from the command line:

```sh
./artifacts/build/my_agent run "Hello, who are you?"
```

To start the agent as an HTTP server:

```sh
./artifacts/build/my_agent serve
```

The generated scaffold includes a minimal manifest and HTTP server configuration so you can get to a running agent quickly. For the full getting started guide, manifest reference, runtime configuration, and deployment details, see the [documentation](https://docs.agentc.sh).

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for the full contribution workflow, required checks, commit message rules, and DCO signoff requirements.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feat/amazing-feature`)
3. Commit your Changes (`git commit -m 'feat: Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.

<!-- CONTACT -->
## Contact

Email: [hello@agentc.sh](mailto:hello@agentc.sh)
Project Link: [https://github.com/agentc-sh/agentc](https://github.com/agentc-sh/agentc)
Website: [https://agentc.sh](https://agentc.sh)

<p align="right">(<a href="#readme-top">back to top</a>)</p>
