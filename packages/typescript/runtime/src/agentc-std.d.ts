declare module 'node:buffer' {
  /**
   * A byte buffer extending `Uint8Array`, with encoding and numeric read/write
   * operations. Use the static creation methods to construct buffers.
   */
  export interface Buffer extends Uint8Array {
    /**
     * Copies a source byte range into the target buffer.
     * @returns The number of bytes copied.
     */
    copy(
      target: Uint8Array,
      targetStart?: number,
      sourceStart?: number,
      sourceEnd?: number,
    ): number
    /**
     * Decodes the selected byte range as a string. The default encoding is
     * UTF-8.
     */
    toString(encoding?: string, start?: number, end?: number): string
    /**
     * Encodes a string into this buffer. Additional arguments select the
     * offset, maximum byte length, and encoding.
     * @returns The number of bytes written.
     */
    write(value: string, ...args: unknown[]): number

    /**
     * Reads a 8-bit signed integer at the byte offset.
     * @returns The decoded value.
     */
    readInt8(offset?: number): number
    /**
     * Reads a 8-bit unsigned integer at the byte offset.
     * @returns The decoded value.
     */
    readUInt8(offset?: number): number
    /**
     * Reads a 8-bit unsigned integer at the byte offset.
     * @returns The decoded value.
     */
    readUint8(offset?: number): number
    /**
     * Reads a 16-bit signed integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readInt16LE(offset?: number): number
    /**
     * Reads a 16-bit signed integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readInt16BE(offset?: number): number
    /**
     * Reads a 16-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUInt16LE(offset?: number): number
    /**
     * Reads a 16-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUInt16BE(offset?: number): number
    /**
     * Reads a 16-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUint16LE(offset?: number): number
    /**
     * Reads a 16-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUint16BE(offset?: number): number
    /**
     * Reads a 32-bit signed integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readInt32LE(offset?: number): number
    /**
     * Reads a 32-bit signed integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readInt32BE(offset?: number): number
    /**
     * Reads a 32-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUInt32LE(offset?: number): number
    /**
     * Reads a 32-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUInt32BE(offset?: number): number
    /**
     * Reads a 32-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUint32LE(offset?: number): number
    /**
     * Reads a 32-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readUint32BE(offset?: number): number
    /**
     * Reads a 32-bit floating-point in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readFloatLE(offset?: number): number
    /**
     * Reads a 32-bit floating-point in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readFloatBE(offset?: number): number
    /**
     * Reads a 64-bit floating-point in little-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readDoubleLE(offset?: number): number
    /**
     * Reads a 64-bit floating-point in big-endian byte order at the byte
     * offset.
     * @returns The decoded value.
     */
    readDoubleBE(offset?: number): number
    /**
     * Reads a 64-bit bigint in little-endian byte order at the byte offset.
     * @returns The decoded value.
     */
    readBigInt64LE(offset?: number): bigint
    /**
     * Reads a 64-bit bigint in big-endian byte order at the byte offset.
     * @returns The decoded value.
     */
    readBigInt64BE(offset?: number): bigint
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    readBigUInt64LE(offset?: number): bigint
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    readBigUInt64BE(offset?: number): bigint
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    readBigUint64LE(offset?: number): bigint
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    readBigUint64BE(offset?: number): bigint

    /**
     * Writes a 8-bit signed integer at the byte offset.
     * @returns The offset immediately after the written value.
     */
    writeInt8(value: number, offset?: number): number
    /**
     * Writes a 8-bit unsigned integer at the byte offset.
     * @returns The offset immediately after the written value.
     */
    writeUInt8(value: number, offset?: number): number
    /**
     * Writes a 8-bit unsigned integer at the byte offset.
     * @returns The offset immediately after the written value.
     */
    writeUint8(value: number, offset?: number): number
    /**
     * Writes a 16-bit signed integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeInt16LE(value: number, offset?: number): number
    /**
     * Writes a 16-bit signed integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeInt16BE(value: number, offset?: number): number
    /**
     * Writes a 16-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUInt16LE(value: number, offset?: number): number
    /**
     * Writes a 16-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUInt16BE(value: number, offset?: number): number
    /**
     * Writes a 16-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUint16LE(value: number, offset?: number): number
    /**
     * Writes a 16-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUint16BE(value: number, offset?: number): number
    /**
     * Writes a 32-bit signed integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeInt32LE(value: number, offset?: number): number
    /**
     * Writes a 32-bit signed integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeInt32BE(value: number, offset?: number): number
    /**
     * Writes a 32-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUInt32LE(value: number, offset?: number): number
    /**
     * Writes a 32-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUInt32BE(value: number, offset?: number): number
    /**
     * Writes a 32-bit unsigned integer in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUint32LE(value: number, offset?: number): number
    /**
     * Writes a 32-bit unsigned integer in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeUint32BE(value: number, offset?: number): number
    /**
     * Writes a 32-bit floating-point in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeFloatLE(value: number, offset?: number): number
    /**
     * Writes a 32-bit floating-point in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeFloatBE(value: number, offset?: number): number
    /**
     * Writes a 64-bit floating-point in little-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeDoubleLE(value: number, offset?: number): number
    /**
     * Writes a 64-bit floating-point in big-endian byte order at the byte
     * offset.
     * @returns The offset immediately after the written value.
     */
    writeDoubleBE(value: number, offset?: number): number
    /**
     * Writes a 64-bit bigint in little-endian byte order at the byte offset.
     * @returns The offset immediately after the written value.
     */
    writeBigInt64LE(value: bigint, offset?: number): number
    /**
     * Writes a 64-bit bigint in big-endian byte order at the byte offset.
     * @returns The offset immediately after the written value.
     */
    writeBigInt64BE(value: bigint, offset?: number): number
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    writeBigUInt64LE(value: bigint, offset?: number): number
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    writeBigUInt64BE(value: bigint, offset?: number): number
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    writeBigUint64LE(value: bigint, offset?: number): number
    /**
     * Unsigned 64-bit buffer access entry point.
     * @throws {TypeError} The runtime rejects unsigned 64-bit buffer access.
     */
    writeBigUint64BE(value: bigint, offset?: number): number
  }

  /** Constructs and inspects byte buffers. */
  export const Buffer: {
    /**
     * Creates a byte buffer from a length or byte source. Prefer the named
     * static creation methods.
     */
    new (arg?: number | ArrayLike<number> | ArrayBufferLike): Buffer
    /**
     * Allocates a buffer of the requested length, initialized with the supplied
     * fill or zero.
     */
    alloc(
      length: number,
      fill?: string | number | Uint8Array,
      encoding?: string,
    ): Buffer
    /** Allocates a buffer whose contents must be initialized before use. */
    allocUnsafe(size: number): Buffer
    /** Allocates an uninitialized buffer using the slow allocation path. */
    allocUnsafeSlow(size: number): Buffer
    /**
     * Returns the encoded byte length of a string or the byte length of binary
     * data.
     */
    byteLength(
      value: string | Uint8Array | ArrayBufferLike,
      encoding?: string,
    ): number
    /**
     * Concatenates byte arrays into one buffer, optionally limiting the result
     * length.
     */
    concat(list: Uint8Array[], maxLength?: number): Buffer
    /**
     * Creates a buffer from a string or byte source. `offsetOrEncoding` selects
     * a byte offset for an array buffer or an encoding for a string.
     */
    from(
      value: string | ArrayLike<number> | ArrayBufferLike,
      offsetOrEncoding?: number | string,
      length?: number,
    ): Buffer
    /** Returns whether the value is a Buffer. */
    isBuffer(value: unknown): boolean
    /** Returns whether the value names a supported encoding. */
    isEncoding(value: unknown): boolean
  }

  /** Decodes Base64 into a binary string. */
  export function atob(data: string): string
  /** Encodes a binary string as Base64. */
  export function btoa(data: string): string

  /** Buffer and string size limits. */
  export const constants: {
    /** The maximum supported buffer length. */
    MAX_LENGTH: number
    /** The maximum supported string length. */
    MAX_STRING_LENGTH: number
  }
}

declare module 'buffer' {
  export * from 'node:buffer'
}

declare module 'node:console' {
  /**
   * A console with logging and assertion methods. The global console also
   * provides counters and timers.
   */
  export class Console {
    /** Creates a console instance. */
    constructor()

    /** Writes formatted values to the log output. */
    log(...args: unknown[]): void
    /** Clears the console display. */
    clear(): void
    /** Writes formatted debugging values. */
    debug(...args: unknown[]): void
    /** Writes formatted informational values. */
    info(...args: unknown[]): void
    /** Writes formatted diagnostic values. */
    trace(...args: unknown[]): void
    /** Writes formatted error values. */
    error(...args: unknown[]): void
    /** Writes formatted warning values. */
    warn(...args: unknown[]): void
    /** Writes an assertion message when the expression is false. */
    assert(expression: boolean, ...args: unknown[]): void
  }
}

declare module 'console' {
  export * from 'node:console'
}

declare module 'node:timers' {
  /**
   * Schedules a callback once after a delay.
   * @param callback - The callback, invoked without arguments.
   * @param delay - The delay in milliseconds, defaulting to zero.
   * @returns A numeric timer identifier.
   */
  export function setTimeout(callback: () => void, delay?: number): number
  /** Cancels the timeout identified by `id`. */
  export function clearTimeout(id?: number): void
  /**
   * Schedules a callback repeatedly at the requested interval.
   * @param callback - The callback, invoked without arguments.
   * @param delay - The interval in milliseconds, defaulting to zero.
   * @returns A numeric timer identifier.
   */
  export function setInterval(callback: () => void, delay?: number): number
  /** Cancels the interval identified by `id`. */
  export function clearInterval(id?: number): void
  /**
   * Schedules a callback for immediate asynchronous execution.
   * @returns A numeric timer identifier.
   */
  export function setImmediate(callback: () => void): number
  /** Queues a callback for execution as a microtask. */
  export function queueMicrotask(callback: () => void): void
}

declare module 'timers' {
  export * from 'node:timers'
}

declare module 'node:url' {
  /** An ordered collection of URL query parameter names and values. */
  export class URLSearchParams {
    /**
     * Creates query parameters from a string, pairs, record, or another
     * parameter collection.
     */
    constructor(
      init?: string | string[][] | Record<string, string> | URLSearchParams,
    )

    /** The number of query parameter pairs. */
    readonly size: number

    /**
     * Appends a query parameter pair, preserving existing values for the key.
     */
    append(key: string, value: string): void
    /** Removes parameters matching the key and optional value. */
    delete(key: string, value?: string): void
    /** Returns an iterator over query parameter name/value pairs. */
    entries(): IterableIterator<string[]>
    /** Calls the callback with each parameter's value and key. */
    forEach(callback: (value: string, key: string) => void): void
    /** Returns the first value for a key, or `null` when absent. */
    get(key: string): string | null
    /** Returns all values for a key. */
    getAll(key: string): string[]
    /** Returns whether the key and optional value occur. */
    has(key: string, value?: string): boolean
    /** Returns parameter names as an array. */
    keys(): string[]
    /** Sets one value for a key, replacing its existing values. */
    set(key: string, value: string): void
    /** Sorts the parameters by key. */
    sort(): void
    /** Returns the serialized string representation. */
    toString(): string
    /** Returns parameter values as an array. */
    values(): string[];
    /** Returns an iterator over query parameter name/value pairs. */
    [Symbol.iterator](): IterableIterator<string[]>
  }

  /** A parsed URL with readable and writable components. */
  export class URL {
    /** Parses a URL, resolving relative input against the optional base URL. */
    constructor(input: string | URL, base?: string | URL)

    /** The fragment, including its leading `#` when present. */
    hash: string
    /** The hostname and optional port. */
    host: string
    /** The hostname without the port. */
    hostname: string
    /** The serialized URL. */
    href: string
    /** The URL origin. */
    readonly origin: string
    /** The password in the URL credentials. */
    password: string
    /** The path component. */
    pathname: string
    /** The port component. */
    port: string
    /** The scheme, including its trailing colon. */
    protocol: string
    /** The query string, including its leading `?` when present. */
    search: string
    /** The query parameters associated with this URL. */
    readonly searchParams: URLSearchParams
    /** The username in the URL credentials. */
    username: string
    /** Returns whether the input can be parsed using the optional base URL. */
    static canParse(input: string | URL, base?: string | URL): boolean
    /**
     * Parses the input using the optional base URL, returning `null` on
     * failure.
     */
    static parse(input: string | URL, base?: string | URL): URL | null
    /** Returns the serialized URL for JSON conversion. */
    toJSON(): string
    /** Returns the serialized string representation. */
    toString(): string
  }

  /** URL components returned by {@link urlToHttpOptions}. */
  export interface HttpOptions {
    /** The scheme, including its trailing colon. */
    protocol: string
    /** The hostname without the port. */
    hostname: string
    /** The URL fragment. */
    hash?: string
    /** The URL query string. */
    search?: string
    /** The path component. */
    pathname: string
    /** The path and query string combined. */
    path: string
    /** The serialized URL. */
    href: string
    /** The URL credentials as a username/password string. */
    auth?: string
    /** The URL port. */
    port?: string
  }

  /** Extracts HTTP request options from a parsed URL. */
  export function urlToHttpOptions(url: URL): HttpOptions
  /** Converts an ASCII domain name to its Unicode representation. */
  export function domainToUnicode(domain: string): string
  /** Converts a Unicode domain name to its ASCII representation. */
  export function domainToASCII(domain: string): string
  /** Converts a file URL to a filesystem path. */
  export function fileURLToPath(url: string | URL): string
  /** Converts a filesystem path to a file URL. */
  export function pathToFileURL(path: string): URL
  /** Formats a URL as a string using the supplied options. */
  export function format(url: URL, options?: unknown): string
}

declare module 'url' {
  export * from 'node:url'
}

declare module 'node:os' {
  /** Information about the current operating-system user. */
  export interface UserInfo {
    /** The user identifier. */
    uid: number
    /** The primary group identifier. */
    gid: number
    /** The user name, or `null` when unavailable. */
    username: string | null
    /** The user's home directory. */
    homedir: string
    /** The user's shell, or `null` when unavailable. */
    shell: string | null
  }

  /** Returns the host CPU architecture. */
  export function arch(): string
  /** Returns the estimated parallelism available to the process. */
  export function availableParallelism(): number
  /** The platform's null-device path. */
  export const devNull: string
  /** Returns the host byte order as `LE` or `BE`. */
  export function endianness(): 'LE' | 'BE'
  /** The platform's line-ending string. */
  export const EOL: string
  /** Returns the scheduling priority for the selected process. */
  export function getPriority(who?: number): number
  /** Returns the current user's home directory. */
  export function homedir(): string
  /** Returns the host platform identifier. */
  export function platform(): string
  /** Returns the operating-system release. */
  export function release(): string
  /** Sets the current process's scheduling priority. */
  export function setPriority(priority: number): void
  /** Sets the selected process's scheduling priority. */
  export function setPriority(who: number, priority: number): void
  /** Returns the operating system's temporary directory. */
  export function tmpdir(): string
  /** Returns the operating-system name. */
  export function type(): string
  /** Returns information about the current operating-system user. */
  export function userInfo(options?: unknown): UserInfo
  /** Returns the operating-system version. */
  export function version(): string
}

declare module 'os' {
  export * from 'node:os'
}

declare module 'node:stream/web' {
  /** Controls queue capacity and chunk sizing for a stream. */
  export interface QueuingStrategy<T = any> {
    /** The queue size threshold used to apply backpressure. */
    highWaterMark?: number
    /** Computes the queue size contribution of a chunk. */
    size?: (chunk: T) => number
  }

  /** Selects the kind of reader to acquire. */
  export interface ReadableStreamGetReaderOptions {
    /** Selects a bring-your-own-buffer reader with `byob`. */
    mode?: 'byob'
  }

  /**
   * Controls how closure, errors, cancellation, and abort signals propagate
   * during piping.
   */
  export interface StreamPipeOptions {
    /** Whether to leave the destination open when the source closes. */
    preventClose?: boolean
    /**
     * Whether to keep the destination from being aborted when the source
     * errors.
     */
    preventAbort?: boolean
    /**
     * Whether to keep the source from being cancelled when the destination
     * errors.
     */
    preventCancel?: boolean
    /** An abort signal controlling the pipe operation. */
    signal?: unknown
  }

  /** A readable output and writable input used by a stream transform. */
  export interface ReadableWritablePair<R = any, W = any> {
    /** The readable output side. */
    readable: ReadableStream<R>
    /** The writable input side. */
    writable: WritableStream<W>
  }

  /** A chunk or completion result from a stream reader. */
  export interface ReadableStreamReadResult<R> {
    /** Whether the stream has finished. */
    done: boolean
    /** The returned chunk, when available. */
    value?: R
  }

  /** A stream of readable chunks. */
  export class ReadableStream<R = any> {
    /**
     * Creates a readable stream from an optional source and queuing strategy.
     */
    constructor(underlyingSource?: object, strategy?: QueuingStrategy<R>)

    /**
     * Creates a readable stream from a synchronous or asynchronous iterable.
     */
    static from<T>(
      asyncIterable: AsyncIterable<T> | Iterable<T>,
    ): ReadableStream<T>
    /** Whether a reader or writer currently holds the stream lock. */
    readonly locked: boolean
    /** Whether reading or cancellation has begun. */
    readonly disturbed: boolean

    /** Cancels reading and passes the reason to the stream. */
    cancel(reason?: unknown): Promise<void>
    /** Acquires a default reader and locks this stream. */
    getReader(): ReadableStreamDefaultReader<R>
    /**
     * Acquires the reader selected by the supplied options and locks this
     * stream.
     */
    getReader(options: ReadableStreamGetReaderOptions): ReadableStreamBYOBReader
    /**
     * Pipes this stream through a readable/writable pair and returns its
     * readable output.
     */
    pipeThrough<T>(
      transform: ReadableWritablePair<T, R>,
      options?: StreamPipeOptions,
    ): ReadableStream<T>
    /** Pipes chunks into the destination until completion or failure. */
    pipeTo(
      destination: WritableStream<R>,
      options?: StreamPipeOptions,
    ): Promise<void>
    /** Splits the stream into two readable branches. */
    tee(): [ReadableStream<R>, ReadableStream<R>]
    /** Returns parameter values as an array. */
    values(options?: object): AsyncIterableIterator<R>;
    /** Returns an asynchronous iterator over stream chunks. */
    [Symbol.asyncIterator](): AsyncIterableIterator<R>
  }

  /** A reader that acquires complete chunks from a readable stream. */
  export class ReadableStreamDefaultReader<R = any> {
    /** Acquires a default reader for the supplied stream. */
    constructor(stream: ReadableStream<R>)

    /** A promise that resolves on closure and rejects on a stream error. */
    readonly closed: Promise<void>

    /** Reads the next chunk or a completion result. */
    read(): Promise<ReadableStreamReadResult<R>>
    /** Releases this reader's or writer's lock on its stream. */
    releaseLock(): void
    /** Cancels reading and passes the reason to the stream. */
    cancel(reason?: unknown): Promise<void>
  }

  /** A byte-stream reader that reads into a caller-supplied buffer view. */
  export class ReadableStreamBYOBReader {
    /** Acquires a bring-your-own-buffer reader for the supplied byte stream. */
    constructor(stream: ReadableStream)

    /** A promise that resolves on closure and rejects on a stream error. */
    readonly closed: Promise<void>

    /**
     * Reads bytes into the supplied view and returns the populated view or
     * completion result.
     */
    read<T extends ArrayBufferView>(
      view: T,
      options?: object,
    ): Promise<ReadableStreamReadResult<T>>
    /** Releases this reader's or writer's lock on its stream. */
    releaseLock(): void
    /** Cancels reading and passes the reason to the stream. */
    cancel(reason?: unknown): Promise<void>
  }

  /** Controls the chunks, closure, and errors of a readable stream. */
  export class ReadableStreamDefaultController<R = any> {
    private constructor()

    /** The additional queue capacity, or `null` when the stream is errored. */
    readonly desiredSize: number | null

    /** Closes the stream after queued work completes. */
    close(): void
    /** Adds a chunk to the readable queue. */
    enqueue(chunk?: R): void
    /** Writes formatted error values. */
    error(reason?: unknown): void
  }

  /** Controls a readable byte stream and its pending buffer requests. */
  export class ReadableByteStreamController {
    private constructor()

    /** The pending bring-your-own-buffer request, or `null`. */
    readonly byobRequest: ReadableStreamBYOBRequest | null
    /** The additional queue capacity, or `null` when the stream is errored. */
    readonly desiredSize: number | null

    /** Closes the stream after queued work completes. */
    close(): void
    /** Adds a chunk to the readable queue. */
    enqueue(chunk: ArrayBufferView): void
    /** Writes formatted error values. */
    error(reason?: unknown): void
  }

  /** A pending request to fill a reader-supplied byte buffer. */
  export class ReadableStreamBYOBRequest {
    private constructor()

    /** The view awaiting bytes, or `null` after the request is invalidated. */
    readonly view: ArrayBufferView | null

    /** Completes the request with the number of bytes written into its view. */
    respond(bytesWritten: number): void
    /** Completes the request using a view over the supplied bytes. */
    respondWithNewView(view: ArrayBufferView): void
  }

  /** A stream that accepts chunks for a writable destination. */
  export class WritableStream<W = any> {
    /** Creates a writable stream from an optional sink and queuing strategy. */
    constructor(underlyingSink?: object, strategy?: QueuingStrategy<W>)

    /** Whether a reader or writer currently holds the stream lock. */
    readonly locked: boolean

    /** Aborts the writable stream and passes the reason to its destination. */
    abort(reason?: unknown): Promise<void>
    /** Closes the stream after queued work completes. */
    close(): Promise<void>
    /** Acquires a writer and locks this stream. */
    getWriter(): WritableStreamDefaultWriter<W>
  }

  /** A writer that owns a writable stream's lock. */
  export class WritableStreamDefaultWriter<W = any> {
    /** Acquires a writer for the supplied stream. */
    constructor(stream: WritableStream<W>)

    /** A promise that resolves on closure and rejects on a stream error. */
    readonly closed: Promise<void>
    /** The additional queue capacity, or `null` when the stream is errored. */
    readonly desiredSize: number | null
    /** A promise that resolves when backpressure permits writing. */
    readonly ready: Promise<void>

    /** Aborts the writable stream and passes the reason to its destination. */
    abort(reason?: unknown): Promise<void>
    /** Closes the stream after queued work completes. */
    close(): Promise<void>
    /** Releases this reader's or writer's lock on its stream. */
    releaseLock(): void
    /** Writes a chunk to the stream. */
    write(chunk?: W): Promise<void>
  }

  /** Controls errors for a writable stream. */
  export class WritableStreamDefaultController {
    private constructor()

    /** Writes formatted error values. */
    error(reason?: unknown): void
  }

  /** A paired writable input and readable output that transforms chunks. */
  export class TransformStream<I = any, O = any> {
    /**
     * Creates a transform with optional input and output queuing strategies.
     */
    constructor(
      transformer?: object,
      writableStrategy?: QueuingStrategy<I>,
      readableStrategy?: QueuingStrategy<O>,
    )

    /** The transform's readable output side. */
    readonly readable: ReadableStream<O> | null
    /** The transform's writable input side. */
    readonly writable: WritableStream<I> | null
  }

  /** Controls output, errors, and termination of a transform stream. */
  export class TransformStreamDefaultController<O = any> {
    private constructor()

    /** The additional capacity in the transform's readable queue. */
    readonly desiredSize: number | null

    /** Adds a chunk to the readable queue. */
    enqueue(chunk?: O): void
    /** Writes formatted error values. */
    error(reason?: unknown): void
    /** Closes the readable side and errors the writable side. */
    terminate(): void
  }

  /** A stream queuing strategy that measures chunks by byte length. */
  export class ByteLengthQueuingStrategy {
    /** Creates a byte-length strategy with the supplied high-water mark. */
    constructor(init: { highWaterMark: number })

    /** The function that measures each chunk's queue size. */
    readonly size: (chunk: ArrayBufferView) => number
    /** The configured queue capacity threshold. */
    readonly highWaterMark: number
  }

  /** A stream queuing strategy that assigns size one to each chunk. */
  export class CountQueuingStrategy {
    /** Creates a chunk-count strategy with the supplied high-water mark. */
    constructor(init: { highWaterMark: number })

    /** The function that measures each chunk's queue size. */
    readonly size: (chunk?: unknown) => number
    /** The configured queue capacity threshold. */
    readonly highWaterMark: number
  }
}

declare module 'stream/web' {
  export * from 'node:stream/web'
}
