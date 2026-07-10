// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use flate2::{Compression, write::GzEncoder};
use ignore::WalkBuilder;
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use toml_edit::DocumentMut;

/// Runtime crates that are embedded into the compiler binary.
/// Update here when adding new runtime crates,
/// then also update EMBEDDED_RUNTIME in src/runtime/mod.rs and Cargo.toml.j2.
const RUNTIME_CRATES: &[&str] = &[
    "agentc-agent",
    "agentc-agent-macros",
    "agentc-agent-react",
    "agentc-config",
    "agentc-database",
    "agentc-domain",
    "agentc-domain-sql",
    "agentc-http",
    "agentc-mcp",
    "agentc-model",
    "agentc-prompt",
    "agentc-protocol-ag-ui",
    "agentc-protocol-a2a",
    "agentc-skills",
    "agentc-telemetry",
    "agentc-tools",
];

#[derive(Debug)]
pub enum BundleError {
    Io(io::Error),
    Toml(String),
    MissingDependency(String),
}

/// The generic Archiver engine.
pub struct Archiver {
    source: PathBuf,
    overrides: HashMap<PathBuf, Vec<u8>>,
}

impl Archiver {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            overrides: HashMap::new(),
        }
    }

    pub fn with_override(mut self, path: impl AsRef<Path>, content: Vec<u8>) -> Self {
        self.overrides
            .insert(path.as_ref().to_path_buf(), content);
        self
    }

    pub fn archive_to(&self, destination: impl AsRef<Path>) -> Result<(), BundleError> {
        let tar_gz = fs::File::create(destination).map_err(BundleError::Io)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut builder = tar::Builder::new(enc);

        let walker = WalkBuilder::new(&self.source)
            .git_ignore(true)
            .hidden(false)
            .build();

        for entry in walker {
            let path = entry
                .map_err(|e| BundleError::Io(io::Error::other(e)))?
                .into_path();
            let relative = path
                .strip_prefix(&self.source)
                .map_err(|e| BundleError::Io(io::Error::other(e)))?;

            if !path.is_file() {
                continue;
            }

            if let Some(content) = self.overrides.get(relative) {
                self.append_bytes(&mut builder, relative, content)?;
            } else {
                builder
                    .append_path_with_name(&path, relative)
                    .map_err(BundleError::Io)?;
            }
        }

        builder
            .into_inner()
            .map_err(BundleError::Io)?
            .finish()
            .map_err(BundleError::Io)?;

        Ok(())
    }

    fn append_bytes<W: Write>(
        &self,
        builder: &mut tar::Builder<W>,
        path: &Path,
        content: &[u8],
    ) -> Result<(), BundleError> {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, path, content)
            .map_err(BundleError::Io)
    }
}

/// The Domain-Specific Logic for Cargo manifests.
struct WorkspaceResolver;

impl WorkspaceResolver {
    fn resolve(member: &mut DocumentMut, workspace: &DocumentMut) -> Result<(), BundleError> {
        Self::resolve_package_fields(member, workspace)?;
        Self::resolve_dependencies(member, workspace)?;

        Ok(())
    }

    fn resolve_package_fields(
        member: &mut DocumentMut,
        workspace: &DocumentMut,
    ) -> Result<(), BundleError> {
        let workspace_pkg = workspace
            .get("workspace")
            .and_then(|w| w.get("package"));
        let member_pkg = member
            .get_mut("package")
            .and_then(|p| p.as_table_mut());

        if let (Some(w_pkg), Some(m_pkg)) = (workspace_pkg, member_pkg) {
            for (key, val) in m_pkg.iter_mut() {
                if val
                    .get("workspace")
                    .and_then(|w| w.as_bool())
                    == Some(true)
                {
                    *val = match w_pkg.get(key.get()) {
                        Some(workspace_val) => workspace_val.clone(),
                        None => return Err(BundleError::MissingDependency(key.to_string())),
                    };
                }
            }
        }

        Ok(())
    }

    fn resolve_dependencies(
        member: &mut DocumentMut,
        workspace: &DocumentMut,
    ) -> Result<(), BundleError> {
        let Some(w_deps) = workspace
            .get("workspace")
            .and_then(|w| w.get("dependencies")?.as_table())
        else {
            return Ok(());
        };

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = member
                .get_mut(section)
                .and_then(|d| d.as_table_mut())
            else {
                continue;
            };

            for (name, item) in deps.iter_mut() {
                if item
                    .get("workspace")
                    .and_then(|w| w.as_bool())
                    != Some(true)
                {
                    continue;
                }

                let workspace_val = w_deps.get(name.get()).ok_or_else(|| {
                    BundleError::MissingDependency(format!(
                        "Dependency '{}' not found in workspace",
                        name
                    ))
                })?;

                let member_table = item
                    .as_table_like_mut()
                    .ok_or_else(|| {
                        BundleError::Toml(format!("Dependency '{}' must be a table", name))
                    })?;

                member_table.remove("workspace");

                if let Some(w_table) = workspace_val.as_table_like() {
                    for (k, v) in w_table.iter() {
                        if !member_table.contains_key(k) {
                            member_table.insert(k, v.clone());
                        }
                    }
                } else if !member_table.contains_key("version") {
                    member_table.insert("version", workspace_val.clone());
                }
            }
        }
        Ok(())
    }
}

/// The Public API
pub fn bundle_crate(
    crate_path: impl Into<PathBuf>,
    workspace_path: impl Into<PathBuf>,
    output_path: impl Into<PathBuf>,
) -> Result<(), BundleError> {
    let crate_path = crate_path.into();
    let workspace_path = workspace_path.into();

    let workspace_toml_path = workspace_path.join("Cargo.toml");
    let member_toml_path = crate_path.join("Cargo.toml");

    let workspace_doc = fs::read_to_string(&workspace_toml_path)
        .map_err(BundleError::Io)?
        .parse::<DocumentMut>()
        .map_err(|e| BundleError::Toml(e.to_string()))?;

    let mut member_doc = fs::read_to_string(&member_toml_path)
        .map_err(BundleError::Io)?
        .parse::<DocumentMut>()
        .map_err(|e| BundleError::Toml(e.to_string()))?;

    WorkspaceResolver::resolve(&mut member_doc, &workspace_doc)?;

    Archiver::new(&crate_path)
        .with_override("Cargo.toml", member_doc.to_string().into_bytes())
        .archive_to(output_path.into())
}

fn main() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("Failed to read CARGO_MANIFEST_DIR");
    let root_dir = Path::new(&manifest_dir)
        .parent()
        .unwrap();
    let workspace_dir = root_dir.parent().unwrap();
    let embedded_dir = Path::new(&manifest_dir).join("embedded");

    if !embedded_dir.exists() {
        fs::create_dir_all(&embedded_dir).unwrap();
    }

    for crate_name in RUNTIME_CRATES {
        let crate_dir = workspace_dir
            .join("crates")
            .join(crate_name);
        let output_path = embedded_dir.join(format!("{}.crate", crate_name));

        println!("cargo:rerun-if-changed={}/src", crate_dir.display());
        println!("cargo:rerun-if-changed={}/Cargo.toml", crate_dir.display());
        println!("cargo:rerun-if-changed={}/Cargo.lock", crate_dir.display());
        println!("cargo:rerun-if-changed={}/build.rs", crate_dir.display());

        if let Err(e) = bundle_crate(&crate_dir, workspace_dir, &output_path) {
            panic!("Error bundling {}: {:?}", crate_name, e);
        }
    }
}
