declare class Blob {
  constructor(parts?: (Blob | BufferSource | string)[], options?: { type?: string; endings?: "transparent" | "native" })
  readonly size: number
  readonly type: string
  slice(start?: number, end?: number, type?: string): Blob
  stream(): ReadableStream
  text(): Promise<string>
  arrayBuffer(): Promise<ArrayBuffer>
}
declare class File extends Blob {
  constructor(parts: (Blob | BufferSource | string)[], name: string, options?: { type?: string; lastModified?: number })
  readonly name: string
  readonly lastModified: number
}
type BufferSource = ArrayBuffer | ArrayBufferView
