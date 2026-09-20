// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod path;
mod timestamp;

pub mod directory;
pub mod entry;
pub mod exceptions;
pub mod executor;
pub mod file;
pub mod library;
pub mod module;
pub mod stat;

pub use crate::python::{executor::ExecutorBuilderFsExt, library::FsLibrary};

#[cfg(test)]
mod tests {
    use agentc_executor_python::{
        executor::Executor,
        guestpy::{
            bundle::Bundle,
            handle::{Coroutine, ObjectProtocol},
            marshal::FromGuest,
            rustpython::RustPython,
        },
    };

    use super::ExecutorBuilderFsExt;
    use crate::fs::{Dir, Fs};

    const SOURCE: &str = r#"
from datetime import datetime
from os import SEEK_END, SEEK_SET
from pathlib import PurePosixPath

import agentc_fs


async def foundation():
    root = agentc_fs.Directory.root()
    stat = await root.stat(PurePosixPath("a.txt"))

    try:
        await root.stat("missing.txt")
    except agentc_fs.NotFoundError as error:
        missing = (
            isinstance(error, agentc_fs.FilesystemError)
            and error.path == "/missing.txt"
        )
    else:
        missing = False

    return "|".join(
        (
            root.path,
            root.authority_root,
            str(stat.size),
            str(isinstance(stat.modified, datetime)),
            str(missing),
        )
    )


async def directory_operations():
    root = agentc_fs.Directory.root()
    workspace = await root.mkdir("projects/demo", parents=True)
    opened = await root.open_dir(PurePosixPath("projects") / "demo")

    await opened.write_text("notes.txt", "hello")
    await opened.write_bytes("data.bin", b"data")

    names = sorted([entry.name async for entry in opened.scandir()])
    walked = sorted([entry.path async for entry in root.walk()])
    entry = await opened.entry("notes.txt")
    stat = await opened.stat("notes.txt")
    text = await opened.read_text("notes.txt")

    await opened.rename("notes.txt", "renamed.txt")
    await opened.symlink("renamed.txt", "notes.link")
    link = await opened.readlink("notes.link")
    exists = await opened.exists("renamed.txt")
    accessible = await opened.access("renamed.txt", read=True)

    await opened.remove("notes.link")
    await opened.remove("renamed.txt")
    await opened.remove("data.bin")
    await root.rmtree("projects")

    return "|".join(
        (
            workspace.path,
            text,
            ",".join(names),
            ",".join(walked),
            entry.type,
            str(stat.size),
            link,
            str(exists),
            str(accessible),
            str(await root.exists("projects")),
        )
    )


async def file_operations():
    root = agentc_fs.Directory.root()
    file = await root.open("file.bin", write=True, create=True)

    await file.write(b"content")
    end = await file.tell()
    await file.seek(0, SEEK_SET)
    first = await file.read(4)
    await file.seek(-3, SEEK_END)
    last = await file.read()
    await file.truncate(4)
    await file.flush()
    await file.fsync()
    await file.fdatasync()
    await file.close()
    await file.close()

    try:
        await file.read()
    except agentc_fs.UnsupportedError as error:
        closed_error = str(error)
    else:
        closed_error = ""

    async with await root.open("file.bin") as context_file:
        context_value = await context_file.read()

    return "|".join(
        (
            str(end),
            first.decode(),
            last.decode(),
            str(file.closed),
            closed_error,
            context_value.decode(),
            str(context_file.closed),
        )
    )


async def argument_and_error_contract():
    root = agentc_fs.Directory.root()

    try:
        await root.stat(path="missing")
    except TypeError:
        positional = True
    else:
        positional = False

    try:
        await root.open_dir("..")
    except agentc_fs.PathEscapesAuthorityError as error:
        escape = error.path == ".."
    else:
        escape = False

    try:
        await root.read_bytes("missing")
    except agentc_fs.NotFoundError:
        mapped = True
    else:
        mapped = False

    return f"{positional}|{escape}|{mapped}"
"#;

    async fn executor(dir: Dir) -> Executor<RustPython> {
        Executor::<RustPython>::builder("agentc_fs_integration_test")
            .bundle(
                Bundle::single("agentc_fs_integration_test", SOURCE).expect("the bundle builds"),
            )
            .workers(2)
            .with_fs(dir)
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T>(executor: &Executor<RustPython>, export: &'static str) -> T::Owned
    where
        T: FromGuest<RustPython> + 'static,
        T::Owned: Send + 'static,
    {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, Coroutine<RustPython, T>>(())?
                        .await
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn the_bound_directory_and_values_cross_every_worker() {
        let fs = Fs::memory();
        let root = fs.root();

        root.options()
            .write(true)
            .create(true)
            .open("a.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        let executor = executor(root).await;

        for _ in 0..4 {
            assert_eq!(call::<String>(&executor, "foundation").await, "/|/|5|True|True",);
        }

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn directory_operations_cross_the_binding() {
        let executor = executor(Fs::memory().root()).await;

        assert_eq!(
            call::<String>(&executor, "directory_operations").await,
            concat!(
                "/projects/demo|hello|data.bin,notes.txt|",
                "/projects,/projects/demo,/projects/demo/data.bin,/projects/demo/notes.txt|",
                "file|5|/projects/demo/renamed.txt|True|True|False",
            ),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn file_lifecycle_crosses_the_binding() {
        let executor = executor(Fs::memory().root()).await;

        assert_eq!(
            call::<String>(&executor, "file_operations").await,
            "7|cont|ent|True|File is closed|cont|True",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn argument_roles_and_error_classes_cross_the_binding() {
        let executor = executor(Fs::memory().root()).await;

        assert_eq!(
            call::<String>(&executor, "argument_and_error_contract").await,
            "True|True|True",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
