declare module 'agentc:http' {
  /**
   * Options for an HTTP request sent by {@link fetch}. Only `method`,
   * `headers`, and `body` are read from this object.
   */
  export interface FetchInit {
    /** The HTTP method. Defaults to `GET`. */
    method?: string
    /** Request header names and values. */
    headers?: Record<string, string>
    /** The request body as UTF-8 text or raw bytes. */
    body?: string | Uint8Array
  }

  /** The response header collection returned by {@link Response.headers}. */
  export class Headers {
    private constructor()

    /**
     * Returns a header's text value, or `null` when no text value is available.
     */
    get(name: string): string | null
    /** Returns whether a text value is available for the header. */
    has(name: string): boolean
    /** Returns the header names as an array. */
    keys(): string[]
    /** Returns header name/value pairs as arrays. */
    entries(): string[][];
    /** Iterates over header name/value pairs. */
    [Symbol.iterator](): IterableIterator<string[]>
  }

  /**
   * The HTTP response returned by {@link fetch}. The body can be consumed once,
   * through `body` or a body-reading method.
   */
  export class Response {
    private constructor()

    /** The HTTP status code. */
    readonly status: number
    /** Whether the status code is in the successful 200 to 299 range. */
    readonly ok: boolean
    /** The response URL. */
    readonly url: string
    /** The standard reason phrase for the status code, or an empty string. */
    readonly statusText: string
    /**
     * Whether a body stream or body-reading method has already taken the body.
     */
    readonly bodyUsed: boolean
    /** The response headers. */
    readonly headers: Headers
    /**
     * Takes the response body as a byte stream.
     * @throws If the body has already been taken or read.
     */
    readonly body: ReadableStream<Uint8Array>

    /**
     * Consumes the body and decodes it as UTF-8 text, replacing invalid
     * sequences.
     * @returns The complete response text.
     */
    text(): Promise<string>
    /**
     * Consumes the body and parses it as JSON.
     * @returns The parsed JSON value.
     * @throws If the body cannot be read or is invalid JSON.
     */
    json(): Promise<unknown>
    /** Consumes the body and returns its complete contents as bytes. */
    bytes(): Promise<Uint8Array>
    /**
     * Consumes the body and returns its complete contents as a `Uint8Array`.
     */
    arrayBuffer(): Promise<Uint8Array>
  }

  /**
   * Sends an HTTP request through the configured client.
   * @param url - The request URL as a string.
   * @param init - Optional method, headers, and body.
   * @returns The response, including its single-use body. HTTP error statuses
   * are returned as responses.
   * @throws If the request is invalid, denied by policy, or fails in transport.
   * @example
   * ```ts
   * await fetch('https://example.com/data')
   *   .then(response => response.json())
   * ```
   */
  export function fetch(url: string, init?: FetchInit): Promise<Response>
}
