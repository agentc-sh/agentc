declare const fetch: typeof import('agentc:http').fetch

declare const Headers: typeof import('agentc:http').Headers
type Headers = import('agentc:http').Headers

declare const Response: typeof import('agentc:http').Response
type Response = import('agentc:http').Response

declare const Buffer: typeof import('node:buffer').Buffer
type Buffer = import('node:buffer').Buffer

declare class Blob {
  constructor(
    parts?: unknown,
    options?: { type?: string; endings?: 'transparent' | 'native' },
  )
  readonly size: number
  readonly type: string
  text(): Promise<string>
  arrayBuffer(): Promise<ArrayBuffer>
  bytes(): Promise<Uint8Array>
  slice(start?: number, end?: number, contentType?: string): Blob
  stream(): ReadableStream<Uint8Array>
}

declare class File {
  constructor(
    parts: unknown,
    name: string,
    options?: {
      type?: string
      lastModified?: number
      endings?: 'transparent' | 'native'
    },
  )
  readonly size: number
  readonly name: string
  readonly type: string
  readonly lastModified: number
  text(): Promise<string>
  arrayBuffer(): Promise<ArrayBuffer>
  bytes(): Promise<Uint8Array>
  slice(start?: number, end?: number, contentType?: string): Blob
}

/** Carries more members than the `Console` class exported by `node:console`. */
declare const console: {
  assert(expression: boolean, ...args: unknown[]): void
  clear(): void
  count(label?: string): void
  countReset(label?: string): void
  debug(...args: unknown[]): void
  error(...args: unknown[]): void
  info(...args: unknown[]): void
  log(...args: unknown[]): void
  time(label?: string): void
  timeEnd(label?: string): void
  timeLog(label?: string, ...args: unknown[]): void
  trace(...args: unknown[]): void
  warn(...args: unknown[]): void
}

declare const setTimeout: typeof import('node:timers').setTimeout
declare const clearTimeout: typeof import('node:timers').clearTimeout
declare const setInterval: typeof import('node:timers').setInterval
declare const clearInterval: typeof import('node:timers').clearInterval
declare const setImmediate: typeof import('node:timers').setImmediate
declare const queueMicrotask: typeof import('node:timers').queueMicrotask

declare const URL: typeof import('node:url').URL
type URL = import('node:url').URL

declare const URLSearchParams: typeof import('node:url').URLSearchParams
type URLSearchParams = import('node:url').URLSearchParams

/** The host populates `env` from the process environment and sets nothing else on this object. */
declare const process: {
  readonly env: Record<string, string | undefined>
}

declare const ReadableStream: typeof import('node:stream/web').ReadableStream
type ReadableStream<R = any> = import('node:stream/web').ReadableStream<R>

declare const ReadableStreamDefaultReader: typeof import('node:stream/web').ReadableStreamDefaultReader
type ReadableStreamDefaultReader<R = any> =
  import('node:stream/web').ReadableStreamDefaultReader<R>

declare const ReadableStreamBYOBReader: typeof import('node:stream/web').ReadableStreamBYOBReader
type ReadableStreamBYOBReader =
  import('node:stream/web').ReadableStreamBYOBReader

declare const ReadableStreamDefaultController: typeof import('node:stream/web').ReadableStreamDefaultController
type ReadableStreamDefaultController<R = any> =
  import('node:stream/web').ReadableStreamDefaultController<R>

declare const ReadableByteStreamController: typeof import('node:stream/web').ReadableByteStreamController
type ReadableByteStreamController =
  import('node:stream/web').ReadableByteStreamController

declare const ReadableStreamBYOBRequest: typeof import('node:stream/web').ReadableStreamBYOBRequest
type ReadableStreamBYOBRequest =
  import('node:stream/web').ReadableStreamBYOBRequest

declare const WritableStream: typeof import('node:stream/web').WritableStream
type WritableStream<W = any> = import('node:stream/web').WritableStream<W>

declare const WritableStreamDefaultWriter: typeof import('node:stream/web').WritableStreamDefaultWriter
type WritableStreamDefaultWriter<W = any> =
  import('node:stream/web').WritableStreamDefaultWriter<W>

declare const WritableStreamDefaultController: typeof import('node:stream/web').WritableStreamDefaultController
type WritableStreamDefaultController =
  import('node:stream/web').WritableStreamDefaultController

declare const TransformStream: typeof import('node:stream/web').TransformStream
type TransformStream<I = any, O = any> =
  import('node:stream/web').TransformStream<I, O>

declare const TransformStreamDefaultController: typeof import('node:stream/web').TransformStreamDefaultController
type TransformStreamDefaultController<O = any> =
  import('node:stream/web').TransformStreamDefaultController<O>

declare const ByteLengthQueuingStrategy: typeof import('node:stream/web').ByteLengthQueuingStrategy
type ByteLengthQueuingStrategy =
  import('node:stream/web').ByteLengthQueuingStrategy

declare const CountQueuingStrategy: typeof import('node:stream/web').CountQueuingStrategy
type CountQueuingStrategy = import('node:stream/web').CountQueuingStrategy

/** Installed when `node:fs` or `node:fs/promises` is evaluated, and named by their signatures. */
declare class Dirent {
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

declare class Stats {
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
  isDir(): boolean
  isDirectory(): boolean
  isSymlink(): boolean
  isSymbolicLink(): boolean
  isFIFO(): boolean
  isBlockDevice(): boolean
  isCharacterDevice(): boolean
  isSocket(): boolean
}

declare class FileHandle {
  private constructor()
  readonly fd: Promise<number>
  chmod(mode: number): Promise<void>
  chown(uid: number, gid: number): Promise<void>
  close(): Promise<void>
  datasync(): Promise<void>
  read(
    ...args: unknown[]
  ): Promise<{ bytesRead: number; buffer: ArrayBufferView }>
  /** Resolves to a string when an encoding is given, and to a `Uint8Array` otherwise. */
  readFile(options?: string | { encoding?: string }): Promise<unknown>
  stat(): Promise<Stats>
  sync(): Promise<void>
  truncate(len?: number): Promise<void>
  write(
    ...args: unknown[]
  ): Promise<{ bytesWritten: number; buffer: string | ArrayBufferView }>
  writeFile(
    data: string | ArrayBufferView,
    options?: string | { encoding?: string },
  ): Promise<void>
}
