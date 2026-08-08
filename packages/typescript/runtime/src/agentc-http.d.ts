declare module 'agentc:http' {
  /** The second argument to `fetch`. Fields not listed here are ignored by the host. */
  export interface FetchInit {
    method?: string
    headers?: Record<string, string>
    body?: string | Uint8Array
  }

  export class Headers {
    private constructor()
    get(name: string): string | null
    has(name: string): boolean
    keys(): string[]
    entries(): string[][]
    ;[Symbol.iterator](): IterableIterator<string[]>
  }

  export class Response {
    private constructor()
    readonly status: number
    readonly ok: boolean
    readonly url: string
    readonly statusText: string
    readonly bodyUsed: boolean
    readonly headers: Headers
    /** Reading this property consumes the body. Reading it a second time throws. */
    readonly body: ReadableStream<Uint8Array>
    text(): Promise<string>
    json(): Promise<unknown>
    bytes(): Promise<Uint8Array>
    /** Resolves to a `Uint8Array`, not an `ArrayBuffer`. */
    arrayBuffer(): Promise<Uint8Array>
  }

  /** `url` must be a string. A `URL` instance is rejected by the host. */
  export function fetch(url: string, init?: FetchInit): Promise<Response>
}
