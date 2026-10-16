/** File operations against the filesystem supplied by the host. */
declare module 'agentc:fs' {
  /**
   * A supported text or binary-to-text encoding. `utf-8` aliases `utf8`;
   * `binary` aliases `latin1`. `hex` and `base64` encode bytes when reading and
   * decode strings when writing.
   */
  export type FileEncoding =
    | 'utf8'
    | 'utf-8'
    | 'hex'
    | 'base64'
    | 'latin1'
    | 'binary'
    | 'ascii'

  /** Encoding, creation permissions, and open flags for file operations. */
  export interface FileOptions {
    /** The encoding used to convert between strings and file bytes. */
    encoding?: FileEncoding
    /** Permission bits applied when creating a new file. */
    mode?: number
    /**
     * The open flag string: `r`, `r+`, `w`, `wx`, `w+`, `wx+`, `a`, `ax`, `a+`,
     * or `ax+`. Read, write, and append operations default to `r`, `w`, and `a`
     * respectively.
     */
    flag?: string
  }

  /** Options for creating a directory. */
  export interface MkdirOptions {
    /** Whether to create missing parent directories. Defaults to `false`. */
    recursive?: boolean
    /** Permission bits applied only when a directory is newly created. */
    mode?: number
  }

  /** Options controlling directory traversal and returned entry values. */
  export interface ReaddirOptions {
    /**
     * Whether to return `Dirent` objects instead of names. Defaults to `false`.
     */
    withFileTypes?: boolean
    /** Whether to include descendant entries. Defaults to `false`. */
    recursive?: boolean
  }

  /** Options for removing a filesystem entry. */
  export interface RmOptions {
    /**
     * Whether to remove directory contents recursively. Defaults to `false`.
     */
    recursive?: boolean
    /**
     * Whether to ignore a missing entry during removal. Defaults to `false`.
     */
    force?: boolean
  }

  /** Options for removing a directory. */
  export interface RmdirOptions {
    /**
     * Whether to remove directory contents recursively. Defaults to `false`.
     */
    recursive?: boolean
  }

  /** Buffer range and file position for a file-handle read. */
  export interface ReadOptions {
    /**
     * The buffer to fill. If omitted, a new buffer is allocated for the read.
     */
    buffer?: Uint8Array
    /** The byte offset in the destination buffer. Defaults to zero. */
    offset?: number
    /**
     * The maximum bytes to read. Defaults to the remaining buffer space, or
     * 16384 for a new buffer.
     */
    length?: number
    /**
     * The file offset in bytes. If omitted, reading uses and advances the
     * current file position.
     */
    position?: number
  }

  /** The byte count and buffer returned by a file-handle read. */
  export interface ReadResult {
    /** The number of bytes written into the destination buffer. */
    bytesRead: number
    /** The destination buffer, preserving the identity of a supplied buffer. */
    buffer: Uint8Array
  }

  /** The byte count and original byte buffer returned by a write. */
  export interface WriteBufferResult {
    /** The number of bytes written to the file. */
    bytesWritten: number
    /** The original byte buffer supplied to the write. */
    buffer: Uint8Array
  }

  /** The byte count and original string returned by a write. */
  export interface WriteStringResult {
    /** The number of bytes written to the file. */
    bytesWritten: number
    /** The original string supplied to the write. */
    buffer: string
  }

  /** A directory entry returned by `readdir` with `withFileTypes: true`. */
  export class Dirent {
    private constructor()

    /** The entry name. */
    readonly name: string
    /** The path of the entry's parent directory. */
    readonly parentPath: string

    /** Returns whether the entry is a regular file. */
    isFile(): boolean
    /** Returns whether the entry is a directory. */
    isDirectory(): boolean
    /** Returns whether the entry is a symbolic link. */
    isSymbolicLink(): boolean
    /** Returns whether the entry is a named pipe. */
    isFIFO(): boolean
    /** Returns whether the entry is a block device. */
    isBlockDevice(): boolean
    /** Returns whether the entry is a character device. */
    isCharacterDevice(): boolean
    /** Returns whether the entry is a socket. */
    isSocket(): boolean
  }

  /**
   * Metadata describing a filesystem entry. Timestamps use milliseconds since
   * the Unix epoch or corresponding `Date` values.
   */
  export class Stats {
    private constructor()

    /** The device identifier. */
    readonly dev: number
    /** The inode number. */
    readonly ino: number
    /** The file type and permission bits. */
    readonly mode: number
    /** The number of hard links. */
    readonly nlink: number
    /** The owner user identifier. */
    readonly uid: number
    /** The owner group identifier. */
    readonly gid: number
    /** The device identifier for a special file. */
    readonly rdev: number
    /** The entry size in bytes. */
    readonly size: number
    /** The preferred block size for I/O. */
    readonly blksize: number
    /** The number of allocated 512-byte blocks. */
    readonly blocks: number
    /**
     * The last access time in milliseconds since the Unix epoch, or zero when
     * unavailable.
     */
    readonly atimeMs: number
    /**
     * The last modification time in milliseconds since the Unix epoch, or zero
     * when unavailable.
     */
    readonly mtimeMs: number
    /**
     * The metadata change time in epoch milliseconds, falling back to
     * modification time and then zero.
     */
    readonly ctimeMs: number
    /**
     * The creation time in milliseconds since the Unix epoch, or zero when
     * unavailable.
     */
    readonly birthtimeMs: number
    /**
     * The last access time as a `Date`, using the Unix epoch when unavailable.
     */
    readonly atime: Date
    /**
     * The last modification time as a `Date`, using the Unix epoch when
     * unavailable.
     */
    readonly mtime: Date
    /**
     * The metadata change time as a `Date`, falling back to modification time
     * and then the Unix epoch.
     */
    readonly ctime: Date
    /** The creation time as a `Date`, using the Unix epoch when unavailable. */
    readonly birthtime: Date

    /** Returns whether the entry is a regular file. */
    isFile(): boolean
    /** Returns whether the entry is a directory. */
    isDir(): boolean
    /** Returns whether the entry is a directory. */
    isDirectory(): boolean
    /** Returns whether the entry is a symbolic link. */
    isSymlink(): boolean
    /** Returns whether the entry is a symbolic link. */
    isSymbolicLink(): boolean
    /** Returns whether the entry is a named pipe. */
    isFIFO(): boolean
    /** Returns whether the entry is a block device. */
    isBlockDevice(): boolean
    /** Returns whether the entry is a character device. */
    isCharacterDevice(): boolean
    /** Returns whether the entry is a socket. */
    isSocket(): boolean
  }

  /**
   * An open file returned by {@link open}. Close the handle when finished to
   * release its descriptor.
   */
  export class FileHandle {
    private constructor()

    /** The numeric descriptor identifying this open file. */
    readonly fd: number

    /** Closes the file and releases its descriptor. */
    close(): Promise<void>
    /** Returns metadata for the file's path. */
    stat(): Promise<Stats>
    /** Changes the permission bits at the file's path. */
    chmod(mode: number): Promise<void>
    /** Changes the owner user and group at the file's path. */
    chown(uid: number, gid: number): Promise<void>
    /** Resizes the open file to `len` bytes. Defaults to zero. */
    truncate(len?: number): Promise<void>
    /** Synchronizes file data and metadata. */
    sync(): Promise<void>
    /** Synchronizes file data. */
    datasync(): Promise<void>
    /**
     * Reads the remaining contents from the current file position.
     * @returns Text when an encoding is supplied, otherwise bytes.
     */
    readFile(options?: FileEncoding | FileOptions): Promise<string | Uint8Array>
    /**
     * Writes data at the current file position, using UTF-8 for strings unless
     * an encoding is supplied.
     */
    writeFile(
      data: string | Uint8Array,
      options?: FileEncoding | FileOptions,
    ): Promise<void>
    /**
     * Reads into the supplied buffer and returns that same buffer with the byte
     * count.
     * @param buffer - The destination buffer.
     * @param offset - The destination offset in bytes, defaulting to zero.
     * @param length - The maximum bytes to read, defaulting to the remaining
     * buffer space.
     * @param position - An explicit file offset. Omit it to use and advance the
     * current position.
     */
    read(
      buffer: Uint8Array,
      offset?: number,
      length?: number,
      position?: number,
    ): Promise<ReadResult>
    /**
     * Reads using the requested buffer range and file position.
     * @returns The byte count and destination buffer.
     */
    read(options: ReadOptions): Promise<ReadResult>
    /**
     * Reads up to 16384 bytes at the current file position into a new buffer.
     */
    read(): Promise<ReadResult>
    /**
     * Writes a range of the supplied buffer and returns the original buffer
     * with the byte count.
     * @param buffer - The source buffer.
     * @param offset - The source offset in bytes, defaulting to zero.
     * @param length - The maximum bytes to write, defaulting to the remaining
     * buffer space.
     * @param position - An explicit file offset. Omit it to use and advance the
     * current position.
     */
    write(
      buffer: Uint8Array,
      offset?: number,
      length?: number,
      position?: number,
    ): Promise<WriteBufferResult>
    /**
     * Writes a string and returns the original string with the byte count.
     * @param data - The string to write.
     * @param position - An explicit file offset. Omit it to use and advance the
     * current position.
     * @param encoding - The string encoding, defaulting to UTF-8.
     */
    write(
      data: string,
      position?: number,
      encoding?: FileEncoding,
    ): Promise<WriteStringResult>
  }

  /**
   * Access, file type, permission, open, and copy flags for filesystem
   * operations.
   */
  export const constants: {
    /** Check that the entry exists. */
    F_OK: number
    /** Check read access. */
    R_OK: number
    /** Check write access. */
    W_OK: number
    /** Check execute access. */
    X_OK: number
    /** Mask for the file type bits. */
    S_IFMT: number
    /** Regular file type bits. */
    S_IFREG: number
    /** Directory type bits. */
    S_IFDIR: number
    /** Character device type bits. */
    S_IFCHR: number
    /** Block device type bits. */
    S_IFBLK: number
    /** Named pipe type bits. */
    S_IFIFO: number
    /** Symbolic link type bits. */
    S_IFLNK: number
    /** Socket type bits. */
    S_IFSOCK: number
    /** Owner read, write, and execute permission bits. */
    S_IRWXU: number
    /** Owner read permission bit. */
    S_IRUSR: number
    /** Owner write permission bit. */
    S_IWUSR: number
    /** Owner execute permission bit. */
    S_IXUSR: number
    /** Group read, write, and execute permission bits. */
    S_IRWXG: number
    /** Group read permission bit. */
    S_IRGRP: number
    /** Group write permission bit. */
    S_IWGRP: number
    /** Group execute permission bit. */
    S_IXGRP: number
    /** Other-user read, write, and execute permission bits. */
    S_IRWXO: number
    /** Other-user read permission bit. */
    S_IROTH: number
    /** Other-user write permission bit. */
    S_IWOTH: number
    /** Other-user execute permission bit. */
    S_IXOTH: number
    /** Open for reading. */
    O_RDONLY: number
    /** Open for writing. */
    O_WRONLY: number
    /** Open for reading and writing. */
    O_RDWR: number
    /** Create the file when missing. */
    O_CREAT: number
    /** Require a new file when combined with `O_CREAT`. */
    O_EXCL: number
    /** Truncate the file when opening. */
    O_TRUNC: number
    /** Append writes to the file. */
    O_APPEND: number
    /** Open without following the final symbolic link. */
    O_NOFOLLOW: number
    /** Fail if the copy destination exists. */
    COPYFILE_EXCL: number
    /** Permit a copy with ordinary-copy fallback. */
    COPYFILE_FICLONE: number
    /** Request mandatory cloning, which this binding rejects. */
    COPYFILE_FICLONE_FORCE: number
  }

  /**
   * Checks entry existence or the requested access flags. Defaults to an
   * existence check.
   */
  export function access(path: string, mode?: number): Promise<void>
  /**
   * Appends data to a file, creating it when needed. Strings use UTF-8 unless
   * an encoding is supplied.
   */
  export function appendFile(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): Promise<void>
  /** Changes an entry's permission bits. */
  export function chmod(path: string, mode: number): Promise<void>
  /** Changes an entry's owner user and group. */
  export function chown(path: string, uid: number, gid: number): Promise<void>
  /**
   * Copies a file, replacing an existing destination unless `COPYFILE_EXCL` is
   * set. `COPYFILE_FICLONE` permits an ordinary copy; `COPYFILE_FICLONE_FORCE`
   * raises an error.
   */
  export function copyFile(
    path: string,
    destination: string,
    mode?: number,
  ): Promise<void>
  /**
   * Changes a symbolic link's owner user and group without following the link.
   */
  export function lchown(path: string, uid: number, gid: number): Promise<void>
  /**
   * Returns metadata for an entry without following its final symbolic link.
   */
  export function lstat(path: string): Promise<Stats>
  /**
   * Creates a directory and returns its path. Set `recursive` to create missing
   * parents. A supplied mode applies only to a newly created directory.
   */
  export function mkdir(path: string, options?: MkdirOptions): Promise<string>
  /**
   * Creates a uniquely named temporary directory using the prefix and returns
   * its path.
   */
  export function mkdtemp(prefix: string): Promise<string>
  /**
   * Opens a file and returns a handle.
   * @param path - The file path.
   * @param flags - A supported flag string or numeric flags. Defaults to `r`.
   * @param mode - Permission bits applied when creating a file.
   */
  export function open(
    path: string,
    flags?: string | number,
    mode?: number,
  ): Promise<FileHandle>
  /**
   * Lists entries in sorted path order. Recursive name results are relative to
   * the requested directory. Returns `Dirent` objects.
   */
  export function readdir(
    path: string,
    options: ReaddirOptions & { withFileTypes: true },
  ): Promise<Dirent[]>
  /**
   * Lists entries in sorted path order. Recursive name results are relative to
   * the requested directory. Returns entry names when `withFileTypes` is
   * omitted or false.
   */
  export function readdir(
    path: string,
    options?: ReaddirOptions,
  ): Promise<string[]>
  /**
   * Reads an entire file. An encoding selects a string result; omission selects
   * raw bytes. This overload returns encoded text.
   */
  export function readFile(
    path: string,
    options: FileEncoding | (FileOptions & { encoding: FileEncoding }),
  ): Promise<string>
  /**
   * Reads an entire file. An encoding selects a string result; omission selects
   * raw bytes. This overload returns raw bytes.
   */
  export function readFile(
    path: string,
    options?: FileOptions,
  ): Promise<Uint8Array>
  /** Returns the stored target of a symbolic link. */
  export function readlink(path: string): Promise<string>
  /** Renames or moves an entry. */
  export function rename(oldPath: string, newPath: string): Promise<void>
  /**
   * Removes an entry. Use `recursive` for directory trees and `force` to ignore
   * a missing entry.
   */
  export function rm(path: string, options?: RmOptions): Promise<void>
  /** Removes a directory. Use `recursive` to remove its descendants. */
  export function rmdir(path: string, options?: RmdirOptions): Promise<void>
  /** Returns metadata for an entry, following symbolic links. */
  export function stat(path: string): Promise<Stats>
  /**
   * Creates a symbolic link with its target resolved against the working
   * directory and stored as an absolute path.
   */
  export function symlink(
    target: string,
    path: string,
    type?: string,
  ): Promise<void>
  /** Resizes a file to `len` bytes. Defaults to zero. */
  export function truncate(path: string, len?: number): Promise<void>
  /** Removes a file or symbolic link. */
  export function unlink(path: string): Promise<void>
  /**
   * Writes data to a file, creating or truncating it by default. Strings use
   * UTF-8 unless an encoding is supplied.
   */
  export function writeFile(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): Promise<void>

  /**
   * Synchronously checks entry existence or the requested access flags.
   * Defaults to an existence check.
   */
  export function accessSync(path: string, mode?: number): void
  /**
   * Synchronously appends data to a file, creating it when needed. Strings use
   * UTF-8 unless an encoding is supplied.
   */
  export function appendFileSync(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): void
  /** Synchronously changes an entry's permission bits. */
  export function chmodSync(path: string, mode: number): void
  /** Synchronously changes an entry's owner user and group. */
  export function chownSync(path: string, uid: number, gid: number): void
  /** Closes an open file descriptor. */
  export function closeSync(fd: number): void
  /**
   * Synchronously copies a file, replacing an existing destination unless
   * `COPYFILE_EXCL` is set. `COPYFILE_FICLONE` permits an ordinary copy;
   * `COPYFILE_FICLONE_FORCE` raises an error.
   */
  export function copyFileSync(
    path: string,
    destination: string,
    mode?: number,
  ): void
  /** Synchronizes the file data for an open descriptor. */
  export function fdatasyncSync(fd: number): void
  /** Returns metadata for an open descriptor's path. */
  export function fstatSync(fd: number): Stats
  /** Resizes the open file to `len` bytes. Defaults to zero. */
  export function ftruncateSync(fd: number, len?: number): void
  /** Synchronizes file data and metadata for an open descriptor. */
  export function fsyncSync(fd: number): void
  /**
   * Synchronously changes a symbolic link's owner user and group without
   * following the link.
   */
  export function lchownSync(path: string, uid: number, gid: number): void
  /**
   * Synchronously returns metadata for an entry without following its final
   * symbolic link.
   */
  export function lstatSync(path: string): Stats
  /**
   * Synchronously creates a directory and returns its path. Set `recursive` to
   * create missing parents. A supplied mode applies only to a newly created
   * directory.
   */
  export function mkdirSync(path: string, options?: MkdirOptions): string
  /**
   * Synchronously creates a uniquely named temporary directory using the prefix
   * and returns its path.
   */
  export function mkdtempSync(prefix: string): string
  /**
   * Synchronously opens a file and returns its numeric descriptor. Flags
   * default to `r`; mode applies to newly created files.
   */
  export function openSync(
    path: string,
    flags?: string | number,
    mode?: number,
  ): number
  /**
   * Synchronously lists entries in sorted path order. Recursive name results
   * are relative to the requested directory. Returns `Dirent` objects.
   */
  export function readdirSync(
    path: string,
    options: ReaddirOptions & { withFileTypes: true },
  ): Dirent[]
  /**
   * Synchronously lists entries in sorted path order. Recursive name results
   * are relative to the requested directory. Returns entry names when
   * `withFileTypes` is omitted or false.
   */
  export function readdirSync(path: string, options?: ReaddirOptions): string[]
  /**
   * Synchronously reads an entire file. An encoding selects a string result;
   * omission selects raw bytes. This overload returns encoded text.
   */
  export function readFileSync(
    path: string,
    options: FileEncoding | (FileOptions & { encoding: FileEncoding }),
  ): string
  /**
   * Synchronously reads an entire file. An encoding selects a string result;
   * omission selects raw bytes. This overload returns raw bytes.
   */
  export function readFileSync(path: string, options?: FileOptions): Uint8Array
  /**
   * Reads into the supplied buffer and returns the byte count. An explicit
   * position leaves the current file position unchanged.
   */
  export function readSync(
    fd: number,
    buffer: Uint8Array,
    offset?: number,
    length?: number,
    position?: number,
  ): number
  /** Synchronously returns the stored target of a symbolic link. */
  export function readlinkSync(path: string): string
  /** Synchronously renames or moves an entry. */
  export function renameSync(oldPath: string, newPath: string): void
  /**
   * Synchronously removes an entry. Use `recursive` for directory trees and
   * `force` to ignore a missing entry.
   */
  export function rmSync(path: string, options?: RmOptions): void
  /**
   * Synchronously removes a directory. Use `recursive` to remove its
   * descendants.
   */
  export function rmdirSync(path: string, options?: RmdirOptions): void
  /** Synchronously returns metadata for an entry, following symbolic links. */
  export function statSync(path: string): Stats
  /**
   * Synchronously creates a symbolic link with its target resolved against the
   * working directory and stored as an absolute path.
   */
  export function symlinkSync(target: string, path: string, type?: string): void
  /** Synchronously resizes a file to `len` bytes. Defaults to zero. */
  export function truncateSync(path: string, len?: number): void
  /** Synchronously removes a file or symbolic link. */
  export function unlinkSync(path: string): void
  /**
   * Synchronously writes data to a file, creating or truncating it by default.
   * Strings use UTF-8 unless an encoding is supplied.
   */
  export function writeFileSync(
    path: string,
    data: string | Uint8Array,
    options?: FileEncoding | FileOptions,
  ): void
  /**
   * Writes the selected buffer range and returns the byte count. An explicit
   * position leaves the current file position unchanged.
   */
  export function writeSync(
    fd: number,
    buffer: Uint8Array,
    offset?: number,
    length?: number,
    position?: number,
  ): number
}
