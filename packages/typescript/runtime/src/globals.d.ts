/** The global binding for `fetch`. See `fetch` in `agentc:http`. */
declare const fetch: typeof import('agentc:http').fetch

/** The global binding for `Headers`. See `Headers` in `agentc:http`. */
declare const Headers: typeof import('agentc:http').Headers
/** The global instance type for `Headers`. See `Headers` in `agentc:http`. */
type Headers = import('agentc:http').Headers

/** The global binding for `Response`. See `Response` in `agentc:http`. */
declare const Response: typeof import('agentc:http').Response
/** The global instance type for `Response`. See `Response` in `agentc:http`. */
type Response = import('agentc:http').Response

/** The global binding for `Buffer`. See `Buffer` in `node:buffer`. */
declare const Buffer: typeof import('node:buffer').Buffer
/** The global instance type for `Buffer`. See `Buffer` in `node:buffer`. */
type Buffer = import('node:buffer').Buffer

/** Binary data with an optional media type. */
declare class Blob {
  /**
   * Creates binary data from optional parts, media type, and line-ending
   * options.
   * @param parts - An iterable of strings, blobs, or binary data.
   * @param options - The media type and transparent or native line-ending
   * handling.
   */
  constructor(
    parts?: unknown,
    options?: { type?: string; endings?: 'transparent' | 'native' },
  )

  /** The content size in bytes. */
  readonly size: number
  /** The normalized media type. */
  readonly type: string

  /** Decodes the content as UTF-8 text, replacing invalid sequences. */
  text(): Promise<string>
  /** Returns a copy of the content as an `ArrayBuffer`. */
  arrayBuffer(): Promise<ArrayBuffer>
  /** Returns a copy of the content as a `Uint8Array`. */
  bytes(): Promise<Uint8Array>
  /**
   * Returns a Blob containing the selected byte range with the optional content
   * type.
   */
  slice(start?: number, end?: number, contentType?: string): Blob
  /** Returns the content as a readable byte stream. */
  stream(): ReadableStream<Uint8Array>
}

/** Binary file data with a name, media type, and modification timestamp. */
declare class File {
  /**
   * Creates a named file from parts and options.
   * @param parts - An iterable of strings, blobs, or binary data.
   * @param name - The file name.
   * @param options - Media type, modification time, and line-ending handling.
   * `lastModified` defaults to the current time in milliseconds since the Unix
   * epoch.
   */
  constructor(
    parts: unknown,
    name: string,
    options?: {
      type?: string
      lastModified?: number
      endings?: 'transparent' | 'native'
    },
  )

  /** The content size in bytes. */
  readonly size: number
  /** The file name. */
  readonly name: string
  /** The normalized media type. */
  readonly type: string
  /** The last modification time in milliseconds since the Unix epoch. */
  readonly lastModified: number

  /** Decodes the content as UTF-8 text, replacing invalid sequences. */
  text(): Promise<string>
  /** Returns a copy of the content as an `ArrayBuffer`. */
  arrayBuffer(): Promise<ArrayBuffer>
  /** Returns a copy of the content as a `Uint8Array`. */
  bytes(): Promise<Uint8Array>
  /**
   * Returns a Blob containing the selected byte range with the optional content
   * type.
   */
  slice(start?: number, end?: number, contentType?: string): Blob
}

/** Logging, counters, and timers available through the global console. */
declare const console: {
  /** Writes an assertion message when the expression is false. */
  assert(expression: boolean, ...args: unknown[]): void
  /** Clears the console display. */
  clear(): void
  /** Increments and logs the counter for the label. */
  count(label?: string): void
  /** Resets the counter for the label. */
  countReset(label?: string): void
  /** Writes formatted debugging values. */
  debug(...args: unknown[]): void
  /** Writes formatted error values. */
  error(...args: unknown[]): void
  /** Writes formatted informational values. */
  info(...args: unknown[]): void
  /** Writes formatted values to the log output. */
  log(...args: unknown[]): void
  /** Starts a timer for the label. */
  time(label?: string): void
  /** Logs the elapsed time and ends the timer for the label. */
  timeEnd(label?: string): void
  /** Logs the elapsed time for the label with optional values. */
  timeLog(label?: string, ...args: unknown[]): void
  /** Writes formatted diagnostic values. */
  trace(...args: unknown[]): void
  /** Writes formatted warning values. */
  warn(...args: unknown[]): void
}

/** The global binding for `setTimeout`. See `setTimeout` in `node:timers`. */
declare const setTimeout: typeof import('node:timers').setTimeout
/**
 * The global binding for `clearTimeout`. See `clearTimeout` in `node:timers`.
 */
declare const clearTimeout: typeof import('node:timers').clearTimeout
/** The global binding for `setInterval`. See `setInterval` in `node:timers`. */
declare const setInterval: typeof import('node:timers').setInterval
/**
 * The global binding for `clearInterval`. See `clearInterval` in `node:timers`.
 */
declare const clearInterval: typeof import('node:timers').clearInterval
/**
 * The global binding for `setImmediate`. See `setImmediate` in `node:timers`.
 */
declare const setImmediate: typeof import('node:timers').setImmediate
/**
 * The global binding for `queueMicrotask`. See `queueMicrotask` in
 * `node:timers`.
 */
declare const queueMicrotask: typeof import('node:timers').queueMicrotask

/** The global binding for `URL`. See `URL` in `node:url`. */
declare const URL: typeof import('node:url').URL
/** The global instance type for `URL`. See `URL` in `node:url`. */
type URL = import('node:url').URL

/**
 * The global binding for `URLSearchParams`. See `URLSearchParams` in
 * `node:url`.
 */
declare const URLSearchParams: typeof import('node:url').URLSearchParams
/**
 * The global instance type for `URLSearchParams`. See `URLSearchParams` in
 * `node:url`.
 */
type URLSearchParams = import('node:url').URLSearchParams

/** The process environment exposed to guest code. */
declare const process: {
  /** Environment variable names and their current guest-visible values. */
  readonly env: Record<string, string | undefined>
}

/**
 * The global binding for `ReadableStream`. See `ReadableStream` in
 * `node:stream/web`.
 */
declare const ReadableStream: typeof import('node:stream/web').ReadableStream
type ReadableStream<R = any> = import('node:stream/web').ReadableStream<R>

/**
 * The global binding for `ReadableStreamDefaultReader`. See
 * `ReadableStreamDefaultReader` in `node:stream/web`.
 */
declare const ReadableStreamDefaultReader:
  typeof import('node:stream/web').ReadableStreamDefaultReader
type ReadableStreamDefaultReader<R = any> =
  import('node:stream/web').ReadableStreamDefaultReader<R>

/**
 * The global binding for `ReadableStreamBYOBReader`. See
 * `ReadableStreamBYOBReader` in `node:stream/web`.
 */
declare const ReadableStreamBYOBReader:
  typeof import('node:stream/web').ReadableStreamBYOBReader
/**
 * The global instance type for `ReadableStreamBYOBReader`. See
 * `ReadableStreamBYOBReader` in `node:stream/web`.
 */
type ReadableStreamBYOBReader =
  import('node:stream/web').ReadableStreamBYOBReader

/**
 * The global binding for `ReadableStreamDefaultController`. See
 * `ReadableStreamDefaultController` in `node:stream/web`.
 */
declare const ReadableStreamDefaultController:
  typeof import('node:stream/web').ReadableStreamDefaultController
type ReadableStreamDefaultController<R = any> =
  import('node:stream/web').ReadableStreamDefaultController<R>

/**
 * The global binding for `ReadableByteStreamController`. See
 * `ReadableByteStreamController` in `node:stream/web`.
 */
declare const ReadableByteStreamController:
  typeof import('node:stream/web').ReadableByteStreamController
/**
 * The global instance type for `ReadableByteStreamController`. See
 * `ReadableByteStreamController` in `node:stream/web`.
 */
type ReadableByteStreamController =
  import('node:stream/web').ReadableByteStreamController

/**
 * The global binding for `ReadableStreamBYOBRequest`. See
 * `ReadableStreamBYOBRequest` in `node:stream/web`.
 */
declare const ReadableStreamBYOBRequest:
  typeof import('node:stream/web').ReadableStreamBYOBRequest
/**
 * The global instance type for `ReadableStreamBYOBRequest`. See
 * `ReadableStreamBYOBRequest` in `node:stream/web`.
 */
type ReadableStreamBYOBRequest =
  import('node:stream/web').ReadableStreamBYOBRequest

/**
 * The global binding for `WritableStream`. See `WritableStream` in
 * `node:stream/web`.
 */
declare const WritableStream: typeof import('node:stream/web').WritableStream
type WritableStream<W = any> = import('node:stream/web').WritableStream<W>

/**
 * The global binding for `WritableStreamDefaultWriter`. See
 * `WritableStreamDefaultWriter` in `node:stream/web`.
 */
declare const WritableStreamDefaultWriter:
  typeof import('node:stream/web').WritableStreamDefaultWriter
type WritableStreamDefaultWriter<W = any> =
  import('node:stream/web').WritableStreamDefaultWriter<W>

/**
 * The global binding for `WritableStreamDefaultController`. See
 * `WritableStreamDefaultController` in `node:stream/web`.
 */
declare const WritableStreamDefaultController:
  typeof import('node:stream/web').WritableStreamDefaultController
/**
 * The global instance type for `WritableStreamDefaultController`. See
 * `WritableStreamDefaultController` in `node:stream/web`.
 */
type WritableStreamDefaultController =
  import('node:stream/web').WritableStreamDefaultController

/**
 * The global binding for `TransformStream`. See `TransformStream` in
 * `node:stream/web`.
 */
declare const TransformStream: typeof import('node:stream/web').TransformStream
type TransformStream<I = any, O = any> =
  import('node:stream/web').TransformStream<I, O>

/**
 * The global binding for `TransformStreamDefaultController`. See
 * `TransformStreamDefaultController` in `node:stream/web`.
 */
declare const TransformStreamDefaultController:
  typeof import('node:stream/web').TransformStreamDefaultController
type TransformStreamDefaultController<O = any> =
  import('node:stream/web').TransformStreamDefaultController<O>

/**
 * The global binding for `ByteLengthQueuingStrategy`. See
 * `ByteLengthQueuingStrategy` in `node:stream/web`.
 */
declare const ByteLengthQueuingStrategy:
  typeof import('node:stream/web').ByteLengthQueuingStrategy
/**
 * The global instance type for `ByteLengthQueuingStrategy`. See
 * `ByteLengthQueuingStrategy` in `node:stream/web`.
 */
type ByteLengthQueuingStrategy =
  import('node:stream/web').ByteLengthQueuingStrategy

/**
 * The global binding for `CountQueuingStrategy`. See `CountQueuingStrategy` in
 * `node:stream/web`.
 */
declare const CountQueuingStrategy:
  typeof import('node:stream/web').CountQueuingStrategy
/**
 * The global instance type for `CountQueuingStrategy`. See
 * `CountQueuingStrategy` in `node:stream/web`.
 */
type CountQueuingStrategy = import('node:stream/web').CountQueuingStrategy

/**
 * The global binding for `Dirent`. See `Dirent` in `agentc:fs`. Installed when
 * `agentc:fs` is evaluated.
 */
declare const Dirent: typeof import('agentc:fs').Dirent
/** The global instance type for `Dirent`. See `Dirent` in `agentc:fs`. */
type Dirent = import('agentc:fs').Dirent

/**
 * The global binding for `Stats`. See `Stats` in `agentc:fs`. Installed when
 * `agentc:fs` is evaluated.
 */
declare const Stats: typeof import('agentc:fs').Stats
/** The global instance type for `Stats`. See `Stats` in `agentc:fs`. */
type Stats = import('agentc:fs').Stats

/**
 * The global binding for `FileHandle`. See `FileHandle` in `agentc:fs`.
 * Installed when `agentc:fs` is evaluated.
 */
declare const FileHandle: typeof import('agentc:fs').FileHandle
/**
 * The global instance type for `FileHandle`. See `FileHandle` in `agentc:fs`.
 */
type FileHandle = import('agentc:fs').FileHandle
