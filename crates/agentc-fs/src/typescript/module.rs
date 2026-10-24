// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;
use futures::FutureExt;

use agentc_executor_typescript::{
    backend::ExecutorBackend,
    errors::GuestError,
    guestjs::{
        errors::Error,
        eval::GuestGlobalsExt,
        handle::{class::Payload, protocol::ObjectProtocol},
        host::{
            library::{HostInitializer, HostLibrary},
            module::ModuleDeclaration,
            state::ModuleState,
        },
        imports::GuestModulesExt,
    },
    host::HostRuntime,
};

use crate::{
    fs::Dir,
    typescript::{
        constants::Constants,
        descriptors::Descriptors,
        dirent::Dirent,
        handle::FileHandle,
        operations::{
            Access, AppendFile, Chmod, Chown, CopyFile, Lchown, Lstat, Mkdir, Mkdtemp, Open,
            ReadFile, Readdir, Readlink, Rename, Rm, Rmdir, Stat, Symlink, Truncate, Unlink,
            WriteFile, FsOperation,
        },
        stats::Stats,
        types::BufferRange,
    },
};

#[derive(Clone)]
pub struct FsModule {
    specifier: Cow<'static, str>,
    dir: Dir,
    runtime: HostRuntime,
    descriptors: Descriptors,
}

impl FsModule {
    const DEFAULT_SPECIFIER: &'static str = "agentc:fs";

    pub fn new(dir: Dir, runtime: HostRuntime) -> Self {
        Self {
            specifier: Cow::Borrowed(Self::DEFAULT_SPECIFIER),
            dir,
            runtime,
            descriptors: Descriptors::new(),
        }
    }

    pub fn with_specifier(mut self, specifier: impl Into<Cow<'static, str>>) -> Self {
        self.specifier = specifier.into();
        self
    }

    pub fn specifier(&self) -> &str {
        &self.specifier
    }

    pub fn dir(&self) -> &Dir {
        &self.dir
    }

    pub(crate) fn runtime(&self) -> &HostRuntime {
        &self.runtime
    }

    pub(crate) fn descriptors(&self) -> &Descriptors {
        &self.descriptors
    }
}

struct FsDeclaration<B: ExecutorBackend>(ModuleDeclaration<B>);

impl<B: ExecutorBackend> FsDeclaration<B> {
    fn operation<O: FsOperation<B>>(self) -> Self {
        Self(
            self.0
                .function_async(O::NAME, |context, args| {
                    let dir = context
                        .resolve::<ModuleState<FsModule>>()?
                        .dir()
                        .clone();

                    Ok(O::read(context.enter(), args)?.run(dir))
                })
                .function(format!("{}Sync", O::NAME), |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;

                    module
                        .runtime()
                        .block_on(
                            O::read(context.enter(), args)?
                                .run(module.dir().clone())
                                .map(|result| result.map_err(GuestError::from)),
                        )
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
                        .map_err(Error::host)
                }),
        )
    }

    fn descriptor_functions(self) -> Self {
        Self(
            self.0
                .function_async("open", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let open = Open::read(context.enter(), args)?;

                    Ok(async move {
                        let (resolved, file) = open
                            .run(module.dir().clone())
                            .await?;

                        Ok(
                            FileHandle::<B>::new(
                                module
                                    .descriptors()
                                    .insert(resolved.clone(), file),
                                resolved,
                                module.dir().clone(),
                                module.descriptors().clone(),
                            )
                        )
                    })
                })
                .function("openSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let (resolved, file) = module
                        .runtime()
                        .block_on(
                            Open::read(context.enter(), args)?
                                .run(module.dir().clone())
                                .map(|result| result.map_err(GuestError::from)),
                        )
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
                        .map_err(Error::host)?;

                    Ok(
                        module
                            .descriptors()
                            .insert(resolved, file)
                    )
                })
                .function("closeSync", |context, args| {
                    context
                        .resolve::<ModuleState<FsModule>>()?
                        .descriptors()
                        .close(args.next::<u32>(context.enter())?)?;

                    Ok(())
                })
                .function("readSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let fd = args.next::<u32>(context.enter())?;
                    let BufferRange {
                        buffer,
                        offset,
                        length,
                    } = BufferRange::read(context.enter(), args)?;
                    let position = args.next::<Option<u64>>(context.enter())?;
                    let mut session = module.descriptors().take(fd)?;

                    let (session, result) = module
                        .runtime()
                        .block_on(async move {
                            let result = session.read(length, position).await;

                            (session, result)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                    module
                        .descriptors()
                        .restore(fd, session);

                    let bytes = result?;

                    buffer.write(offset, &bytes)?;

                    Ok(bytes.len() as u32)
                })
                .function("writeSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let fd = args.next::<u32>(context.enter())?;
                    let bytes = BufferRange::read(context.enter(), args)?.bytes()?;
                    let position = args.next::<Option<u64>>(context.enter())?;
                    let mut session = module.descriptors().take(fd)?;

                    let (session, result) = module
                        .runtime()
                        .block_on(async move {
                            let result = session.write(bytes, position).await;

                            (session, result)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                    module
                        .descriptors()
                        .restore(fd, session);

                    Ok(result? as u32)
                })
                .function("fstatSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let path = module
                        .descriptors()
                        .path(args.next::<u32>(context.enter())?)?;
                    let dir = module.dir().clone();

                    module
                        .runtime()
                        .block_on(async move { dir.metadata(path).await })
                        .map(|result| {
                            result
                                .map(Stats::<B>::new)
                                .map_err(Into::into)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?
                })
                .function("ftruncateSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let fd = args.next::<u32>(context.enter())?;
                    let len = args
                        .next::<Option<u64>>(context.enter())?
                        .unwrap_or_default();
                    let mut session = module.descriptors().take(fd)?;

                    let (session, result) = module
                        .runtime()
                        .block_on(async move {
                            let result = session.truncate(len).await;

                            (session, result)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                    module
                        .descriptors()
                        .restore(fd, session);

                    result.map_err(Into::into)
                })
                .function("fsyncSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let fd = args.next::<u32>(context.enter())?;
                    let mut session = module.descriptors().take(fd)?;

                    let (session, result) = module
                        .runtime()
                        .block_on(async move {
                            let result = session.sync_all().await;

                            (session, result)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                    module
                        .descriptors()
                        .restore(fd, session);

                    result.map_err(Into::into)
                })
                .function("fdatasyncSync", |context, args| {
                    let module = context.resolve::<ModuleState<FsModule>>()?;
                    let fd = args.next::<u32>(context.enter())?;
                    let mut session = module.descriptors().take(fd)?;

                    let (session, result) = module
                        .runtime()
                        .block_on(async move {
                            let result = session.sync_data().await;

                            (session, result)
                        })
                        .map_err(|error| Error::unexpected(format!("agentc:fs: {error}")))?;

                    module
                        .descriptors()
                        .restore(fd, session);

                    result.map_err(Into::into)
                }),
        )
    }
}

impl<B: ExecutorBackend> TryFrom<FsModule> for HostLibrary<B> {
    type Error = Error;

    fn try_from(module: FsModule) -> Result<Self, Self::Error> {
        let specifier = module.specifier.clone();

        Ok(
            HostLibrary::from(
                FsDeclaration(
                    ModuleDeclaration::new(specifier.clone())
                        .state(move |_| Ok(module.clone()))
                        .class::<Stats<B>>()
                        .class::<Dirent>()
                        .class::<FileHandle<B>>()
                        .namespace("constants", Constants::declaration()),
                )
                .operation::<Access>()
                .operation::<ReadFile>()
                .operation::<WriteFile>()
                .operation::<AppendFile>()
                .operation::<Rename>()
                .operation::<Readdir>()
                .operation::<Mkdir>()
                .operation::<Mkdtemp>()
                .operation::<Rm>()
                .operation::<Rmdir>()
                .operation::<Unlink>()
                .operation::<Stat>()
                .operation::<Lstat>()
                .operation::<Chmod>()
                .operation::<Chown>()
                .operation::<Lchown>()
                .operation::<Symlink>()
                .operation::<Readlink>()
                .operation::<Truncate>()
                .operation::<CopyFile>()
                .descriptor_functions()
                .0,
            )
            .extend(HostInitializer::new(move |enter| {
                let module = enter.guest().host_module(&specifier)?;
                let globals = enter.guest().globals()?;

                globals.set(Stats::<B>::NAME, module.class(Stats::<B>::NAME)?)?;
                globals.set(Dirent::NAME, module.class(Dirent::NAME)?)?;
                globals.set(FileHandle::<B>::NAME, module.class(FileHandle::<B>::NAME)?)?;

                Ok(())
            })),
        )
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
            marshal::{FromGuest, ToGuestArgs},
            quickjs::QuickJs,
        },
    };

    use crate::{
        fs::{Dir, Fs, Permissions},
        typescript::executor::ExecutorBuilderFsExt,
    };

    const FS_SOURCE: &str = r#"
import {
    appendFile,
    chmod,
    closeSync,
    constants,
    copyFile,
    fdatasyncSync,
    fstatSync,
    ftruncateSync,
    fsyncSync,
    lstat,
    mkdir,
    mkdtemp,
    open,
    openSync,
    readFile,
    readFileSync,
    readSync,
    readdir,
    readlink,
    rename,
    rm,
    stat,
    statSync,
    symlink,
    truncate,
    writeSync,
    writeFile,
    Dirent as ImportedDirent,
    FileHandle as ImportedFileHandle,
    Stats as ImportedStats,
} from "agentc:fs";

export async function writeReadBytes(path) {
    await writeFile(path, Uint8Array.from([1, 2, 3]));

    return Array.from(await readFile(path)).join(",");
}

export async function writeReadUtf8(path) {
    await writeFile(path, "hello", "utf8");

    return await readFile(path, "utf8");
}

export async function appendText(path) {
    await writeFile(path, "hello", "utf8");
    await appendFile(path, " world", "utf8");

    return await readFile(path, "utf8");
}

export async function readEncoded(path) {
    await writeFile(path, Uint8Array.from([104, 101, 108, 108, 111]));

    return `${await readFile(path, "hex")}:${await readFile(path, "base64")}`;
}

export async function makeNested(path) {
    return await mkdir(path, { recursive: true });
}

export async function makeTemps(prefix) {
    const first = await mkdtemp(prefix);
    const second = await mkdtemp(prefix);

    return `${first !== second}:${first.startsWith(prefix)}:${second.startsWith(prefix)}`;
}

export async function mkdirModeOnExisting(path) {
    await mkdir(path, { recursive: true, mode: 0o600 });

    const { mode } = await stat(path);

    return mode & 0o777;
}

export async function mkdirModeOnNew(path) {
    await mkdir(path, { recursive: true, mode: 0o600 });

    const { mode } = await stat(path);

    return mode & 0o777;
}

export async function sortedNames(path) {
    return (await readdir(path)).join(",");
}

export async function direntSummary(path) {
    return (await readdir(path, { withFileTypes: true }))
        .map(entry => `${entry.name}:${entry.parentPath}:${entry.isFile()}:${entry.isDirectory()}`)
        .join("|");
}

export async function recursiveNames(path) {
    return (await readdir(path, { recursive: true })).join(",");
}

export async function statKinds(path) {
    return `${(await stat(path)).isFile()}:${(await lstat(path)).isSymbolicLink()}`;
}

export async function statDates(path) {
    const stats = await stat(path);

    return stats.mtime instanceof Date
        && stats.mtime.getTime() === Math.trunc(stats.mtimeMs);
}

export async function statModeBits(path) {
    return ((await stat(path)).mode & constants.S_IFMT) === constants.S_IFREG;
}

export async function chmodMode(path) {
    await chmod(path, 0o600);

    return (await stat(path)).mode & 0o7777;
}

export async function rmForce(path) {
    await rm(path, { force: true });

    return "ok";
}

export async function rmRecursive(path) {
    await rm(path, { recursive: true });

    try {
        await stat(path);

        return "present";
    } catch {
        return "gone";
    }
}

export async function renameRead(from, to) {
    await rename(from, to);

    return await readFile(to, "utf8");
}

export async function copyExclusive(from, to) {
    try {
        await copyFile(from, to, constants.COPYFILE_EXCL);

        return "copied";
    } catch (error) {
        return error.message;
    }
}

export async function truncateRead(path, length) {
    await truncate(path, length);

    return await readFile(path, "utf8");
}

export async function linkTarget(path) {
    return await readlink(path);
}

export async function writeText(path, text) {
    await writeFile(path, text, "utf8");

    return "ok";
}

export async function missingMessage(path) {
    try {
        await readFile(path, "utf8");

        return "ok";
    } catch (error) {
        return error.message;
    }
}

export async function syncSurface(path) {
    const text = readFileSync(path, "utf8");
    const stats = statSync(path);

    return typeof text === "string"
        && !("then" in Object(text))
        && !("then" in stats)
        && stats.isFile();
}

export async function openReadInto(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(5);
    const result = await handle.read(buffer, 0, 5, 0);

    await handle.close();

    return `${result.bytesRead}:${Array.from(buffer).join(",")}`;
}

export async function identity(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(5);
    const result = await handle.read(buffer, 0, 5, 0);

    await handle.close();

    return result.buffer === buffer;
}

export async function readWithoutBuffer(path) {
    const handle = await open(path, "r");
    const result = await handle.read();

    await handle.close();

    return `${result.bytesRead}:${Array.from(result.buffer).join(",")}`;
}

export async function readAtPosition(path) {
    const handle = await open(path, "r+");
    const positioned = await handle.read(new Uint8Array(2), 0, 2, 1);
    const current = await handle.read(new Uint8Array(2), 0, 2);

    await handle.close();

    return `${Array.from(positioned.buffer).join(",")}:${Array.from(current.buffer).join(",")}`;
}

export async function writeThroughHandle(path) {
    const handle = await open(path, "a+");

    await handle.write(" world");
    await handle.close();

    return await readFile(path, "utf8");
}

export async function syncReadInto(path) {
    const fd = openSync(path, "r");
    const buffer = new Uint8Array(5);
    const bytesRead = readSync(fd, buffer, 0, 5, 0);

    closeSync(fd);

    return `${bytesRead}:${Array.from(buffer).join(",")}`;
}

export async function syncWriteFrom(path) {
    const fd = openSync(path, "w+");
    const buffer = Uint8Array.from([104, 101, 108, 108, 111]);
    const bytesWritten = writeSync(fd, buffer, 0, 5, 0);

    closeSync(fd);

    return `${bytesWritten}:${await readFile(path, "utf8")}`;
}

export async function readSubarray(path) {
    const handle = await open(path, "r");
    const buffer = new Uint8Array(16);

    await handle.read(buffer.subarray(4, 8), 0, 4, 0);
    await handle.close();

    return Array.from(buffer).join(",");
}

export async function syncFstatMatches(path) {
    const fd = openSync(path, "r");
    const same = fstatSync(fd).mode === (await stat(path)).mode;

    closeSync(fd);

    return same;
}

export async function closeTwice(path) {
    const fd = openSync(path, "r");

    closeSync(fd);

    try {
        closeSync(fd);

        return "ok";
    } catch (error) {
        return error.message;
    }
}

export async function busyDescriptor(path) {
    const handle = await open(path, "r");
    const first = handle.read(new Uint8Array(5), 0, 5, 0);

    try {
        await handle.read(new Uint8Array(5), 0, 5, 0);

        await first;
        await handle.close();

        return "ok";
    } catch (error) {
        await first;
        await handle.close();

        return error.message;
    }
}

export async function syncDescriptorOps(path) {
    const fd = openSync(path, "r+");
    const before = fstatSync(fd).size;

    ftruncateSync(fd, 2);
    fsyncSync(fd);
    fdatasyncSync(fd);

    const after = fstatSync(fd).size;

    closeSync(fd);

    return `${before}:${after}`;
}

export async function globalsIdentity() {
    return [
        globalThis.Stats === ImportedStats,
        globalThis.Dirent === ImportedDirent,
        globalThis.FileHandle === ImportedFileHandle,
    ].join(":");
}
"#;

    async fn executor(dir: Dir) -> Executor<QuickJs> {
        Executor::<QuickJs>::builder("fs")
            .bundle(Bundle::single("fs.ts", FS_SOURCE).unwrap())
            .workers(1)
            .standard_environment()
            .with_fs(dir)
            .expect("filesystem binding configures")
            .build()
            .await
            .expect("executor builds")
    }

    async fn call<T, A>(executor: &Executor<QuickJs>, export: &'static str, args: A) -> T
    where
        A: ToGuestArgs<QuickJs> + Send + 'static,
        T: FromGuest<QuickJs, Owned = T> + Send + 'static,
    {
        executor
            .execute(move |context| {
                Box::pin(async move {
                    context
                        .module()
                        .function(export)?
                        .call::<_, Promise<QuickJs, T>>(args)?
                        .await
                })
            })
            .await
            .expect("guest call succeeds")
    }

    #[tokio::test]
    async fn writes_and_reads_a_file_as_bytes() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeReadBytes", ("/bytes.bin".to_owned(),)).await,
            "1,2,3",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn writes_and_reads_a_file_as_utf8() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeReadUtf8", ("/notes.txt".to_owned(),)).await,
            "hello",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn append_file_extends_an_existing_file() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "appendText", ("/notes.txt".to_owned(),)).await,
            "hello world",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_file_with_hex_and_base64_encodings() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "readEncoded", ("/encoded.bin".to_owned(),)).await,
            "68656c6c6f:aGVsbG8=",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdir_recursive_creates_a_nested_path() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "makeNested", ("/work/nested/path".to_owned(),)).await,
            "/work/nested/path",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdir_mode_is_not_applied_to_an_existing_directory() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir_all("/work")
            .await
            .unwrap();
        root.set_permissions("/work", Permissions::new(0o700))
            .await
            .unwrap();

        let mode = call::<u32, _>(&executor, "mkdirModeOnExisting", ("/work".to_owned(),)).await;

        assert_eq!(mode, 0o700);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdir_mode_is_applied_to_a_new_directory() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        let mode = call::<u32, _>(&executor, "mkdirModeOnNew", ("/fresh".to_owned(),)).await;

        assert_eq!(mode, 0o600);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn mkdtemp_creates_a_unique_directory() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "makeTemps", ("/tmp-".to_owned(),)).await,
            "true:true:true",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_returns_sorted_names() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/dir").await.unwrap();

        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/dir/b.txt")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/a.txt")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/c.txt")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "sortedNames", ("/dir".to_owned(),)).await,
            "a.txt,b.txt,c.txt",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_with_file_types_returns_dirent_objects() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir("/dir").await.unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/file.txt")
            .await
            .unwrap();
        root.create_dir("/dir/folder")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "direntSummary", ("/dir".to_owned(),)).await,
            "file.txt:/dir:true:false|folder:/dir:false:true",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn readdir_recursive_returns_relative_names() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir_all("/dir/sub")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/a.txt")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/sub/b.txt")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "recursiveNames", ("/dir".to_owned(),)).await,
            "a.txt,sub,sub/b.txt",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_follows_a_symlink_and_lstat_does_not() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/target.txt")
            .await
            .unwrap();
        root.symlink("/target.txt", "/link.txt")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "statKinds", ("/link.txt".to_owned(),)).await,
            "true:true",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_reports_dates_as_date_objects() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap();

        assert!(call::<bool, _>(&executor, "statDates", ("/file.txt".to_owned(),)).await);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn stat_mode_carries_the_file_type_bits() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap();

        assert!(call::<bool, _>(&executor, "statModeBits", ("/file.txt".to_owned(),)).await);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn chmod_changes_the_reported_mode() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap();

        assert_eq!(
            call::<u32, _>(&executor, "chmodMode", ("/file.txt".to_owned(),)).await,
            0o600_u32,
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn rm_force_ignores_a_missing_path() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(
            call::<String, _>(&executor, "rmForce", ("/missing.txt".to_owned(),)).await,
            "ok",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn rm_recursive_removes_a_populated_directory() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.create_dir_all("/dir/sub")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/dir/sub/file.txt")
            .await
            .unwrap();

        assert_eq!(call::<String, _>(&executor, "rmRecursive", ("/dir".to_owned(),)).await, "gone",);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn rename_moves_a_file() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/from.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(
                &executor,
                "renameRead",
                ("/from.txt".to_owned(), "/to.txt".to_owned())
            )
            .await,
            "hello",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn copy_file_exclusive_refuses_an_existing_destination() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/from.txt")
            .await
            .unwrap()
            .write_all(b"a")
            .await
            .unwrap();
        root.options()
            .write(true)
            .create(true)
            .open("/to.txt")
            .await
            .unwrap()
            .write_all(b"b")
            .await
            .unwrap();

        assert!(
            call::<String, _>(
                &executor,
                "copyExclusive",
                ("/from.txt".to_owned(), "/to.txt".to_owned())
            )
            .await
            .contains("agentc:fs:"),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn truncate_shortens_a_file() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "truncateRead", ("/file.txt".to_owned(), 2_u64)).await,
            "he",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn readlink_returns_the_stored_target() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/target.txt")
            .await
            .unwrap();
        root.symlink("/target.txt", "/link.txt")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "linkTarget", ("/link.txt".to_owned(),)).await,
            "/target.txt",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn relative_paths_resolve_against_the_bound_directory() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/work").await.unwrap();

        let executor = executor(root.open_dir("/work").await.unwrap()).await;

        assert_eq!(
            call::<String, _>(&executor, "writeText", ("data.json".to_owned(), "hello".to_owned()))
                .await,
            "ok",
        );
        assert_eq!(
            root.open_file("/work/data.json")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "hello",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn absolute_paths_resolve_from_the_root() {
        let fs = Fs::memory();
        let root = fs.root();

        root.create_dir("/work").await.unwrap();

        let executor = executor(root.open_dir("/work").await.unwrap()).await;

        assert_eq!(
            call::<String, _>(
                &executor,
                "writeText",
                ("/other.txt".to_owned(), "hello".to_owned())
            )
            .await,
            "ok",
        );
        assert_eq!(
            root.open_file("/other.txt")
                .await
                .unwrap()
                .read_to_string()
                .await
                .unwrap(),
            "hello",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_missing_path_surfaces_as_a_guest_exception() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert!(
            call::<String, _>(&executor, "missingMessage", ("/missing.txt".to_owned(),))
                .await
                .contains("agentc:fs:"),
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn the_synchronous_functions_return_values_rather_than_promises() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert!(call::<bool, _>(&executor, "syncSurface", ("/file.txt".to_owned(),)).await);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn open_and_read_fills_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "openReadInto", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn open_and_read_returns_the_same_buffer_object() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert!(call::<bool, _>(&executor, "identity", ("/file.txt".to_owned(),)).await);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_without_a_buffer_allocates_one() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readWithoutBuffer", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_with_a_position_does_not_move_the_cursor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readAtPosition", ("/file.txt".to_owned(),)).await,
            "101,108:104,101",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn write_through_a_handle_appends_at_the_cursor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "writeThroughHandle", ("/file.txt".to_owned(),)).await,
            "hello world",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn read_sync_fills_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "syncReadInto", ("/file.txt".to_owned(),)).await,
            "5:104,101,108,108,111",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn write_sync_consumes_the_callers_buffer() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        assert_eq!(
            call::<String, _>(&executor, "syncWriteFrom", ("/file.txt".to_owned(),)).await,
            "5:hello",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_subarray_view_is_written_at_its_own_offset() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"rust")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "readSubarray", ("/file.txt".to_owned(),)).await,
            "0,0,0,0,114,117,115,116,0,0,0,0,0,0,0,0",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn fstat_sync_matches_stat_on_the_same_path() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert!(call::<bool, _>(&executor, "syncFstatMatches", ("/file.txt".to_owned(),)).await);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn closing_twice_reports_a_bad_descriptor() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap();

        assert!(
            call::<String, _>(&executor, "closeTwice", ("/file.txt".to_owned(),))
                .await
                .contains("bad file descriptor")
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn a_second_operation_on_a_busy_descriptor_reports_it() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert!(
            call::<String, _>(&executor, "busyDescriptor", ("/file.txt".to_owned(),))
                .await
                .contains("is busy")
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn sync_descriptor_operations_work() {
        let fs = Fs::memory();
        let root = fs.root();
        let executor = executor(root.clone()).await;

        root.options()
            .write(true)
            .create(true)
            .open("/file.txt")
            .await
            .unwrap()
            .write_all(b"hello")
            .await
            .unwrap();

        assert_eq!(
            call::<String, _>(&executor, "syncDescriptorOps", ("/file.txt".to_owned(),)).await,
            "5:2",
        );

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }

    #[tokio::test]
    async fn the_classes_are_installed_as_globals() {
        let fs = Fs::memory();
        let executor = executor(fs.root()).await;

        assert_eq!(call::<String, _>(&executor, "globalsIdentity", ()).await, "true:true:true",);

        executor
            .shutdown()
            .await
            .expect("executor shuts down");
    }
}
