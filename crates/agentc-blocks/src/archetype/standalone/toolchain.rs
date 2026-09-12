// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_compiler::{
    artifact::ExecutableArtifact,
    compiler::{cargo::CargoCompiler, traits::Compiler},
    runner::{ProcessRunner, traits::Runner},
    toolchain::traits::Toolchain,
};

use crate::archetype::standalone::archetype::TargetTriple;

/// The [`CargoCompiler`](agentc_compiler::compiler::cargo::CargoCompiler) paired with
/// the [`ProcessRunner`](agentc_compiler::runner::process::ProcessRunner) that invokes
/// the binary it builds.
pub struct StandaloneToolchain {
    compiler: CargoCompiler,
    runner: Option<ProcessRunner>,
}

impl StandaloneToolchain {
    /// Builds a toolchain for `target`, or for the host when `target` is `None`.
    pub fn new(target: Option<TargetTriple>, host: TargetTriple) -> Self {
        Self {
            compiler: CargoCompiler::new().maybe_with_target(
                target
                    .as_ref()
                    .map(TargetTriple::as_str),
            ),
            // A binary built for a foreign target cannot be started on this host.
            runner: target
                .is_none_or(|triple| triple == host)
                .then(ProcessRunner::new),
        }
    }
}

impl Toolchain for StandaloneToolchain {
    type Artifact = ExecutableArtifact;

    fn compiler(&self) -> &dyn Compiler<Artifact = Self::Artifact> {
        &self.compiler
    }

    fn runner(&self) -> Option<&dyn Runner<Artifact = Self::Artifact>> {
        self.runner
            .as_ref()
            .map(|runner| runner as &dyn Runner<Artifact = Self::Artifact>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::standalone::archetype::{Arch, Os};

    fn host() -> TargetTriple {
        TargetTriple::from((Os::Linux, Arch::X86_64))
    }

    #[test]
    fn host_builds_are_runnable() {
        assert!(
            StandaloneToolchain::new(None, host())
                .runner()
                .is_some()
        );
    }

    #[test]
    fn explicit_host_target_is_runnable() {
        assert!(
            StandaloneToolchain::new(Some(host()), host())
                .runner()
                .is_some()
        );
    }

    #[test]
    fn foreign_target_is_not_runnable() {
        assert!(
            StandaloneToolchain::new(
                Some(TargetTriple::from((Os::Windows, Arch::Aarch64))),
                host(),
            )
            .runner()
            .is_none()
        );
    }
}
