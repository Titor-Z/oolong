declare class Response {
  constructor(body?: BodyInit | null, init?: ResponseInit)
  readonly status: number
  readonly statusText: string
  readonly ok: boolean
  readonly headers: Headers
  readonly body: ReadableStream | null
  readonly bodyUsed: boolean
  readonly redirected: boolean
  readonly type: ResponseType
  readonly url: string
  clone(): Response
  arrayBuffer(): Promise<ArrayBuffer>
  blob(): Promise<Blob>
  json(): Promise<any>
  text(): Promise<string>
  static json(data: any, init?: ResponseInit): Response
  static redirect(url: string, status?: number): Response
  static error(): Response
}
interface ResponseInit { status?: number; statusText?: string; headers?: HeadersInit }
type ResponseType = "basic" | "cors" | "default" | "error" | "opaque" | "opaqueredirect"
