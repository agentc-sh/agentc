// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::{
    backend::ExecutorBackend, executor::ExecutorBuilder, guestjs::errors::Error, host::HostRuntime,
};

use crate::{fs::Dir, typescript::library::FsLibrary};

pub trait ExecutorBuilderFsExt {
    fn with_fs(self, dir: Dir) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<B: ExecutorBackend> ExecutorBuilderFsExt for ExecutorBuilder<B> {
    fn with_fs(self, dir: Dir) -> Result<Self, Error> {
        // We need to capture the host runtime here instead of in the configure closure because the configure closure
        // is executed on each worker thread, and the host runtime is from the main thread.
        let host_runtime = HostRuntime::current().map_err(|e| {
            Error::unexpected(format!("agentc:fs: cannot access host runtime: {e}"))
        })?;

        Ok(self.configure(move |runtime| {
            Ok(runtime.bind(FsLibrary::bind::<B>(dir.clone(), host_runtime.clone())?))
        }))
    }
}

#[cfg(test)]
mod tests {
    use agentc_executor_typescript::{
        executor::Executor,
        guestjs::{
            bundle::Bundle,
            handle::{
                promise::Promise,
                protocol::{CallableProtocol, ObjectProtocol},
            },
            quickjs::QuickJs,
        },
    };

    use super::ExecutorBuilderFsExt;
    use crate::fs::Fs;

    const FS_SOURCE: &str = r#"
import { readFile, writeFile } from "agentc:fs";

export async function roundTrip(path, text) {
    await writeFile(path, text, "utf8");

    return await readFile(path, "utf8");
}
"#;

    #[tokio::test]
    async fn the_extension_trait_binds_the_module_on_every_worker() {
        let fs = Fs::memory();
        let executor = Executor::<QuickJs>::builder("fs")
            .bundle(Bundle::single("fs.ts", FS_SOURCE).unwrap())
            .workers(2)
            .standard_environment()
            .with_fs(fs.root())
            .expect("filesystem binding configures")
            .build()
            .await
            .expect("executor builds");

        for index in 0..4 {
            assert_eq!(
                executor
                    .execute(move |context| {
                        Box::pin(async move {
                            context
                                .module()
                                .function("roundTrip")?
                                .call::<_, Promise<QuickJs, String>>((
                                    format!("/file-{index}.txt"),
                                    format!("value-{index}"),
                                ))?
                                .await
                        })
                    })
                    .await
                    .expect("guest call succeeds"),
                format!("value-{index}"),
            );
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
