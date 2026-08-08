declare module 'node:buffer' {
  /** A `Uint8Array` subclass. The constructor is inherited and undocumented; prefer the static methods. */
  export interface Buffer extends Uint8Array {
    copy(
      target: Uint8Array,
      targetStart?: number,
      sourceStart?: number,
      sourceEnd?: number,
    ): number
    toString(encoding?: string, start?: number, end?: number): string
    write(value: string, ...args: unknown[]): number

    readInt8(offset?: number): number
    readUInt8(offset?: number): number
    readUint8(offset?: number): number
    readInt16LE(offset?: number): number
    readInt16BE(offset?: number): number
    readUInt16LE(offset?: number): number
    readUInt16BE(offset?: number): number
    readUint16LE(offset?: number): number
    readUint16BE(offset?: number): number
    readInt32LE(offset?: number): number
    readInt32BE(offset?: number): number
    readUInt32LE(offset?: number): number
    readUInt32BE(offset?: number): number
    readUint32LE(offset?: number): number
    readUint32BE(offset?: number): number
    readFloatLE(offset?: number): number
    readFloatBE(offset?: number): number
    readDoubleLE(offset?: number): number
    readDoubleBE(offset?: number): number
    readBigInt64LE(offset?: number): bigint
    readBigInt64BE(offset?: number): bigint
    readBigUInt64LE(offset?: number): bigint
    readBigUInt64BE(offset?: number): bigint
    readBigUint64LE(offset?: number): bigint
    readBigUint64BE(offset?: number): bigint

    writeInt8(value: number, offset?: number): number
    writeUInt8(value: number, offset?: number): number
    writeUint8(value: number, offset?: number): number
    writeInt16LE(value: number, offset?: number): number
    writeInt16BE(value: number, offset?: number): number
    writeUInt16LE(value: number, offset?: number): number
    writeUInt16BE(value: number, offset?: number): number
    writeUint16LE(value: number, offset?: number): number
    writeUint16BE(value: number, offset?: number): number
    writeInt32LE(value: number, offset?: number): number
    writeInt32BE(value: number, offset?: number): number
    writeUInt32LE(value: number, offset?: number): number
    writeUInt32BE(value: number, offset?: number): number
    writeUint32LE(value: number, offset?: number): number
    writeUint32BE(value: number, offset?: number): number
    writeFloatLE(value: number, offset?: number): number
    writeFloatBE(value: number, offset?: number): number
    writeDoubleLE(value: number, offset?: number): number
    writeDoubleBE(value: number, offset?: number): number
    writeBigInt64LE(value: bigint, offset?: number): number
    writeBigInt64BE(value: bigint, offset?: number): number
    writeBigUInt64LE(value: bigint, offset?: number): number
    writeBigUInt64BE(value: bigint, offset?: number): number
    writeBigUint64LE(value: bigint, offset?: number): number
    writeBigUint64BE(value: bigint, offset?: number): number
  }

  export const Buffer: {
    new (arg?: number | ArrayLike<number> | ArrayBufferLike): Buffer
    alloc(
      length: number,
      fill?: string | number | Uint8Array,
      encoding?: string,
    ): Buffer
    allocUnsafe(size: number): Buffer
    allocUnsafeSlow(size: number): Buffer
    byteLength(
      value: string | Uint8Array | ArrayBufferLike,
      encoding?: string,
    ): number
    concat(list: Uint8Array[], maxLength?: number): Buffer
    from(
      value: string | ArrayLike<number> | ArrayBufferLike,
      offsetOrEncoding?: number | string,
      length?: number,
    ): Buffer
    isBuffer(value: unknown): boolean
    isEncoding(value: unknown): boolean
  }

  export function atob(data: string): string
  export function btoa(data: string): string

  export const constants: {
    MAX_LENGTH: number
    MAX_STRING_LENGTH: number
  }
}

declare module 'buffer' {
  export * from 'node:buffer'
}

declare module 'node:console' {
  /** The class form. The `console` global carries additional members that this class does not. */
  export class Console {
    constructor()
    log(...args: unknown[]): void
    clear(): void
    debug(...args: unknown[]): void
    info(...args: unknown[]): void
    trace(...args: unknown[]): void
    error(...args: unknown[]): void
    warn(...args: unknown[]): void
    assert(expression: boolean, ...args: unknown[]): void
  }
}

declare module 'console' {
  export * from 'node:console'
}

declare module 'node:timers' {
  /** The callback receives no arguments. Trailing arguments are not forwarded to it. */
  export function setTimeout(callback: () => void, delay?: number): number
  export function clearTimeout(id?: number): void
  export function setInterval(callback: () => void, delay?: number): number
  export function clearInterval(id?: number): void
  export function setImmediate(callback: () => void): number
  export function queueMicrotask(callback: () => void): void
}

declare module 'timers' {
  export * from 'node:timers'
}

declare module 'node:url' {
  export class URLSearchParams {
    constructor(
      init?: string | string[][] | Record<string, string> | URLSearchParams,
    )
    readonly size: number
    append(key: string, value: string): void
    delete(key: string, value?: string): void
    entries(): IterableIterator<string[]>
    forEach(callback: (value: string, key: string) => void): void
    get(key: string): string | null
    getAll(key: string): string[]
    has(key: string, value?: string): boolean
    /** Returns an array, not an iterator. */
    keys(): string[]
    set(key: string, value: string): void
    sort(): void
    toString(): string
    /** Returns an array, not an iterator. */
    values(): string[]
    ;[Symbol.iterator](): IterableIterator<string[]>
  }

  export class URL {
    constructor(input: string | URL, base?: string | URL)
    hash: string
    host: string
    hostname: string
    href: string
    readonly origin: string
    password: string
    pathname: string
    port: string
    protocol: string
    search: string
    readonly searchParams: URLSearchParams
    username: string
    static canParse(input: string | URL, base?: string | URL): boolean
    static parse(input: string | URL, base?: string | URL): URL | null
    toJSON(): string
    toString(): string
  }

  export interface HttpOptions {
    protocol: string
    hostname: string
    hash?: string
    search?: string
    pathname: string
    path: string
    href: string
    auth?: string
    port?: string
  }

  export function urlToHttpOptions(url: URL): HttpOptions
  export function domainToUnicode(domain: string): string
  export function domainToASCII(domain: string): string
  export function fileURLToPath(url: string | URL): string
  export function pathToFileURL(path: string): URL
  export function format(url: URL, options?: unknown): string
}

declare module 'url' {
  export * from 'node:url'
}

declare module 'node:os' {
  export interface UserInfo {
    uid: number
    gid: number
    username: string | null
    homedir: string
    shell: string | null
  }

  export function arch(): string
  export function availableParallelism(): number
  /** A string constant, not a function. */
  export const devNull: string
  export function endianness(): 'LE' | 'BE'
  /** A string constant, not a function. */
  export const EOL: string
  export function getPriority(who?: number): number
  export function homedir(): string
  export function platform(): string
  export function release(): string
  export function setPriority(priority: number): void
  export function setPriority(who: number, priority: number): void
  export function tmpdir(): string
  export function type(): string
  export function userInfo(options?: unknown): UserInfo
  export function version(): string
}

declare module 'os' {
  export * from 'node:os'
}

declare module 'node:stream/web' {
  export interface QueuingStrategy<T = any> {
    highWaterMark?: number
    size?: (chunk: T) => number
  }

  export interface ReadableStreamGetReaderOptions {
    mode?: 'byob'
  }

  export interface StreamPipeOptions {
    preventClose?: boolean
    preventAbort?: boolean
    preventCancel?: boolean
    signal?: unknown
  }

  export interface ReadableWritablePair<R = any, W = any> {
    readable: ReadableStream<R>
    writable: WritableStream<W>
  }

  export interface ReadableStreamReadResult<R> {
    done: boolean
    value?: R
  }

  export class ReadableStream<R = any> {
    constructor(underlyingSource?: object, strategy?: QueuingStrategy<R>)
    static from<T>(
      asyncIterable: AsyncIterable<T> | Iterable<T>,
    ): ReadableStream<T>
    readonly locked: boolean
    readonly disturbed: boolean
    cancel(reason?: unknown): Promise<void>
    getReader(): ReadableStreamDefaultReader<R>
    getReader(options: ReadableStreamGetReaderOptions): ReadableStreamBYOBReader
    pipeThrough<T>(
      transform: ReadableWritablePair<T, R>,
      options?: StreamPipeOptions,
    ): ReadableStream<T>
    pipeTo(
      destination: WritableStream<R>,
      options?: StreamPipeOptions,
    ): Promise<void>
    tee(): [ReadableStream<R>, ReadableStream<R>]
    values(options?: object): AsyncIterableIterator<R>
    ;[Symbol.asyncIterator](): AsyncIterableIterator<R>
  }

  export class ReadableStreamDefaultReader<R = any> {
    constructor(stream: ReadableStream<R>)
    readonly closed: Promise<void>
    read(): Promise<ReadableStreamReadResult<R>>
    releaseLock(): void
    cancel(reason?: unknown): Promise<void>
  }

  export class ReadableStreamBYOBReader {
    constructor(stream: ReadableStream)
    readonly closed: Promise<void>
    read<T extends ArrayBufferView>(
      view: T,
      options?: object,
    ): Promise<ReadableStreamReadResult<T>>
    releaseLock(): void
    cancel(reason?: unknown): Promise<void>
  }

  export class ReadableStreamDefaultController<R = any> {
    private constructor()
    readonly desiredSize: number | null
    close(): void
    enqueue(chunk?: R): void
    error(reason?: unknown): void
  }

  export class ReadableByteStreamController {
    private constructor()
    readonly byobRequest: ReadableStreamBYOBRequest | null
    readonly desiredSize: number | null
    close(): void
    enqueue(chunk: ArrayBufferView): void
    error(reason?: unknown): void
  }

  export class ReadableStreamBYOBRequest {
    private constructor()
    readonly view: ArrayBufferView | null
    respond(bytesWritten: number): void
    respondWithNewView(view: ArrayBufferView): void
  }

  export class WritableStream<W = any> {
    constructor(underlyingSink?: object, strategy?: QueuingStrategy<W>)
    readonly locked: boolean
    abort(reason?: unknown): Promise<void>
    close(): Promise<void>
    getWriter(): WritableStreamDefaultWriter<W>
  }

  export class WritableStreamDefaultWriter<W = any> {
    constructor(stream: WritableStream<W>)
    readonly closed: Promise<void>
    readonly desiredSize: number | null
    readonly ready: Promise<void>
    abort(reason?: unknown): Promise<void>
    close(): Promise<void>
    releaseLock(): void
    write(chunk?: W): Promise<void>
  }

  export class WritableStreamDefaultController {
    private constructor()
    error(reason?: unknown): void
  }

  export class TransformStream<I = any, O = any> {
    constructor(
      transformer?: object,
      writableStrategy?: QueuingStrategy<I>,
      readableStrategy?: QueuingStrategy<O>,
    )
    readonly readable: ReadableStream<O> | null
    readonly writable: WritableStream<I> | null
  }

  export class TransformStreamDefaultController<O = any> {
    private constructor()
    readonly desiredSize: number | null
    enqueue(chunk?: O): void
    error(reason?: unknown): void
    terminate(): void
  }

  export class ByteLengthQueuingStrategy {
    constructor(init: { highWaterMark: number })
    readonly size: (chunk: ArrayBufferView) => number
    readonly highWaterMark: number
  }

  export class CountQueuingStrategy {
    constructor(init: { highWaterMark: number })
    readonly size: (chunk?: unknown) => number
    readonly highWaterMark: number
  }
}

declare module 'stream/web' {
  export * from 'node:stream/web'
}
