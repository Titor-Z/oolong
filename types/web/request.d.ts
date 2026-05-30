declare class Request {
  constructor(input: RequestInfo, init?: RequestInit)
  readonly method: string
  readonly url: string
  readonly headers: Headers
  readonly body: ReadableStream | null
  readonly bodyUsed: boolean
  readonly cache: RequestCache
  readonly credentials: RequestCredentials
  readonly destination: RequestDestination
  readonly integrity: string
  readonly keepalive: boolean
  readonly mode: RequestMode
  readonly redirect: RequestRedirect
  readonly referrer: string
  readonly referrerPolicy: ReferrerPolicy
  readonly signal: AbortSignal
  clone(): Request
  arrayBuffer(): Promise<ArrayBuffer>
  blob(): Promise<Blob>
  json(): Promise<any>
  text(): Promise<string>
}
type RequestInfo = string | Request
interface RequestInit {
  method?: string; headers?: HeadersInit; body?: BodyInit | null
  signal?: AbortSignal; redirect?: RequestRedirect; cache?: RequestCache
  credentials?: RequestCredentials; integrity?: string; keepalive?: boolean
  mode?: RequestMode; referrer?: string; referrerPolicy?: ReferrerPolicy
}
type BodyInit = string | Blob | BufferSource | URLSearchParams | ReadableStream
type RequestCache = "default" | "no-store" | "reload" | "no-cache" | "force-cache" | "only-if-cached"
type RequestCredentials = "omit" | "same-origin" | "include"
type RequestDestination = "" | "document" | "sharedworker" | "subresource" | "unknown" | "worker"
type RequestMode = "navigate" | "same-origin" | "no-cors" | "cors"
type RequestRedirect = "follow" | "error" | "manual"
type ReferrerPolicy = "" | "no-referrer" | "no-referrer-when-downgrade" | "origin" | "origin-when-cross-origin" | "same-origin" | "strict-origin" | "strict-origin-when-cross-origin" | "unsafe-url"
