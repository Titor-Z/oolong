// ── Headers ───────────────────────────────────────────────────────

interface Headers {
  append(name: string, value: string): void
  delete(name: string): void
  get(name: string): string | null
  getSetCookie(): string[]
  has(name: string): boolean
  set(name: string, value: string): void
  forEach(callbackfn: (value: string, key: string, parent: Headers) => void, thisArg?: unknown): void
}

declare var Headers: {
  prototype: Headers
  new(init?: HeadersInit): Headers
}

// ── Response ──────────────────────────────────────────────────────

interface Response {
  readonly headers: Headers
  readonly ok: boolean
  readonly status: number
  readonly statusText: string
  readonly type: ResponseType
  readonly url: string
  readonly bodyUsed: boolean
  readonly body: ReadableStream<Uint8Array> | null
  clone(): Response
  text(): Promise<string>
  json(): Promise<unknown>
  bytes(): Promise<Uint8Array>
}

declare var Response: {
  prototype: Response
  new(body?: BodyInit | null, init?: ResponseInit): Response
  error(): Response
  redirect(url: string, status?: number): Response
}

// ── Request ───────────────────────────────────────────────────────

interface Request {
  readonly method: string
  readonly url: string
  readonly headers: Headers
  readonly bodyUsed: boolean
  readonly body: ReadableStream<Uint8Array> | null
  clone(): Request
  text(): Promise<string>
  json(): Promise<unknown>
  bytes(): Promise<Uint8Array>
}

declare var Request: {
  prototype: Request
  new(input: RequestInfo | URL, init?: RequestInit): Request
}

// ── fetch ─────────────────────────────────────────────────────────

declare function fetch(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response>

// ── Helper types (not exported to JS) ─────────────────────────────

type BodyInit = string | Blob | Uint8Array | null
type HeadersInit = Record<string, string> | [string, string][] | Headers
type RequestInfo = string | Request
type RequestInit = {
  method?: string
  headers?: HeadersInit
  body?: BodyInit
}
type ResponseInit = {
  status?: number
  statusText?: string
  headers?: HeadersInit
}
type ResponseType = "basic" | "cors" | "default" | "error" | "opaque" | "opaqueredirect"
