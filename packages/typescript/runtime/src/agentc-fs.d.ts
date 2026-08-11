declare module 'agentc:fs' {
  export type FileEncoding =
    | 'utf8'
    | 'utf-8'
    | 'hex'
    | 'base64'
    | 'latin1'
    | 'binary'
    | 'ascii'

  export interface FileOptions {
    encoding?: FileEncoding
    mode?: number
    flag?: string | number
  }

  export interface MkdirOptions {
    recursive?: boolean
    mode?: number
  }

  export interface ReaddirOptions {
    withFileTypes?: boolean
    recursive?: boolean
  }

  export interface RmOptions {
    recursive?: boolean
    force?: boolean
  }

  export interface RmdirOptions {
    recursive?: boolean
  }

  export interface ReadOptions {
    buffer?: Uint8Array
    offset?: number
    length?: number
    position?: number
  }

  export interface ReadResult {
    bytesRead: number
    buffer: Uint8Array
  }

  export interface WriteBufferResult {
    bytesWritten: number
    buffer: Uint8Array
  }

  export interface WriteStringResult {
    bytesWritten: number
    buffer: string
  }

  export class Dirent {
    private constructor()
    readonly name: string
    readonly parentPath: string
    isFile(): boolean
    isDirectory(): boolean
    isSymbolicLink(): boolean
    isFIFO(): boolean
    isBlockDevice(): boolean
    isCharacterDevice(): boolean
    isSocket(): boolean
  }

  export class Stats {
    private constructor()
    readonly dev: number
    readonly ino: number
    readonly mode: number
    readonly nlink: number
    readonly uid: number
    readonly gid: number
    readonly rdev: number
    readonly size: number
    readonly blksize: number
    readonly blocks: number
    readonly atimeMs: number
    readonly mtimeMs: number
    readonly ctimeMs: number
    readonly birthtimeMs: number
    readonly atime: Date
    readonly mtime: Date
    readonly ctime: Date
    readonly birthtime: Date
    isFile(): boolean
    isDir(): boolean
    isDirectory(): boolean
    isSymlink(): boolean
    isSymbolicLink(): boolean
    isFIFO(): boolean
    isBlockDevice(): boolean
    isCharacterDevice(): boolean
    isSocket(): boolean
  }

  export class FileHandle {
    private constructor()
    readonly fd: number
    close(): Promise<void>
    stat(): Promise<Stats>
    chmod(mode: number): Promise<void>
    chown(uid: number, gid: number): Promise<void>
    truncate(len?: number): Promise<void>
    sync(): Promise<void>
    datasync(): Promise<void>
    /** Resolves to a string when an encoding is given, and to a `Uint8Array` otherwise. */
    readFile(options?: FileEncoding | FileOptions): Promise<string | Uint8Array>
    writeFile(data: string | Uint8Array, options?: FileEncoding | FileOptions): Promise<void>
    read(buffer: Uint8Array, offset?: number, length?: number, position?: number): Promise<ReadResult>
    read(options: ReadOptions): Promise<ReadResult>
    read(): Promise<ReadResult>
    write(buffer: Uint8Array, offset?: number, length?: number, position?: number): Promise<WriteBufferResult>
    write(data: string, position?: number, encoding?: FileEncoding): Promise<WriteStringResult>
  }

  export const constants: {
    F_OK: number
    R_OK: number
    W_OK: number
    X_OK: number
    S_IFMT: number
    S_IFREG: number
    S_IFDIR: number
    S_IFCHR: number
    S_IFBLK: number
    S_IFIFO: number
    S_IFLNK: number
    S_IFSOCK: number
    S_IRWXU: number
    S_IRUSR: number
    S_IWUSR: number
    S_IXUSR: number
    S_IRWXG: number
    S_IRGRP: number
    S_IWGRP: number
    S_IXGRP: number
    S_IRWXO: number
    S_IROTH: number
    S_IWOTH: number
    S_IXOTH: number
    O_RDONLY: number
    O_WRONLY: number
    O_RDWR: number
    O_CREAT: number
    O_EXCL: number
    O_TRUNC: number
    O_APPEND: number
    O_NOFOLLOW: number
    COPYFILE_EXCL: number
    COPYFILE_FICLONE: number
    COPYFILE_FICLONE_FORCE: number
  }

  export function access(path: string, mode?: number): Promise<void>
  export function appendFile(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): Promise<void>
  export function chmod(path: string, mode: number): Promise<void>
  export function chown(path: string, uid: number, gid: number): Promise<void>
  export function copyFile(path: string, destination: string, mode?: number): Promise<void>
  export function lchown(path: string, uid: number, gid: number): Promise<void>
  export function lstat(path: string): Promise<Stats>
  export function mkdir(path: string, options?: MkdirOptions): Promise<string>
  export function mkdtemp(prefix: string): Promise<string>
  export function open(path: string, flags?: string | number, mode?: number): Promise<FileHandle>
  /** Resolves to `Dirent[]` when `withFileTypes` is set, and to `string[]` otherwise. */
  export function readdir(path: string, options: ReaddirOptions & { withFileTypes: true }): Promise<Dirent[]>
  /** Resolves to `Dirent[]` when `withFileTypes` is set, and to `string[]` otherwise. */
  export function readdir(path: string, options?: ReaddirOptions): Promise<string[]>
  /** Resolves to a string when an encoding is given, and to a `Uint8Array` otherwise. */
  export function readFile(path: string, options: FileEncoding | (FileOptions & { encoding: FileEncoding })): Promise<string>
  /** Resolves to a string when an encoding is given, and to a `Uint8Array` otherwise. */
  export function readFile(path: string, options?: FileOptions): Promise<Uint8Array>
  export function readlink(path: string): Promise<string>
  export function rename(oldPath: string, newPath: string): Promise<void>
  export function rm(path: string, options?: RmOptions): Promise<void>
  export function rmdir(path: string, options?: RmdirOptions): Promise<void>
  export function stat(path: string): Promise<Stats>
  /** The target is resolved against the working directory and stored absolute. */
  export function symlink(target: string, path: string, type?: string): Promise<void>
  export function truncate(path: string, len?: number): Promise<void>
  export function unlink(path: string): Promise<void>
  export function writeFile(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): Promise<void>

  export function accessSync(path: string, mode?: number): void
  export function appendFileSync(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): void
  export function chmodSync(path: string, mode: number): void
  export function chownSync(path: string, uid: number, gid: number): void
  export function closeSync(fd: number): void
  export function copyFileSync(path: string, destination: string, mode?: number): void
  export function fdatasyncSync(fd: number): void
  export function fstatSync(fd: number): Stats
  export function ftruncateSync(fd: number, len?: number): void
  export function fsyncSync(fd: number): void
  export function lchownSync(path: string, uid: number, gid: number): void
  export function lstatSync(path: string): Stats
  export function mkdirSync(path: string, options?: MkdirOptions): string
  export function mkdtempSync(prefix: string): string
  export function openSync(path: string, flags?: string | number, mode?: number): number
  /** Returns `Dirent[]` when `withFileTypes` is set, and `string[]` otherwise. */
  export function readdirSync(path: string, options: ReaddirOptions & { withFileTypes: true }): Dirent[]
  /** Returns `Dirent[]` when `withFileTypes` is set, and `string[]` otherwise. */
  export function readdirSync(path: string, options?: ReaddirOptions): string[]
  /** Returns a string when an encoding is given, and a `Uint8Array` otherwise. */
  export function readFileSync(path: string, options: FileEncoding | (FileOptions & { encoding: FileEncoding })): string
  /** Returns a string when an encoding is given, and a `Uint8Array` otherwise. */
  export function readFileSync(path: string, options?: FileOptions): Uint8Array
  export function readSync(
    fd: number,
    buffer: Uint8Array,
    offset?: number,
    length?: number,
    position?: number,
  ): number
  export function readlinkSync(path: string): string
  export function renameSync(oldPath: string, newPath: string): void
  export function rmSync(path: string, options?: RmOptions): void
  export function rmdirSync(path: string, options?: RmdirOptions): void
  export function statSync(path: string): Stats
  /** The target is resolved against the working directory and stored absolute. */
  export function symlinkSync(target: string, path: string, type?: string): void
  export function truncateSync(path: string, len?: number): void
  export function unlinkSync(path: string): void
  export function writeFileSync(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): void
  export function writeSync(
    fd: number,
    buffer: Uint8Array,
    offset?: number,
    length?: number,
    position?: number,
  ): number
}
