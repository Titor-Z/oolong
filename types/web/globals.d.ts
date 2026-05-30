declare class ReadableStream {
  constructor(underlyingSource?: UnderlyingSource)
  readonly locked: boolean
  cancel(reason?: any): Promise<void>
  getReader(): ReadableStreamDefaultReader
  pipeThrough<T>(transform: ReadableWritablePair<any, T>, options?: { preventClose?: boolean; preventAbort?: boolean; preventCancel?: boolean }): ReadableStream<T>
  pipeTo(dest: WritableStream, options?: { preventClose?: boolean; preventAbort?: boolean; preventCancel?: boolean }): Promise<void>
  tee(): [ReadableStream, ReadableStream]
}
interface UnderlyingSource { start?: (controller: ReadableStreamController) => void; pull?: (controller: ReadableStreamController) => Promise<void>; cancel?: (reason?: any) => void; type?: "bytes" }
interface ReadableStreamController { enqueue(chunk: any): void; close(): void; error(e: any): void; readonly desiredSize: number | null }
declare class ReadableStreamDefaultReader { read(): Promise<{ done: boolean; value: any }>; releaseLock(): void; cancel(reason?: any): Promise<void>; readonly closed: Promise<void> }
interface ReadableWritablePair<R, W> { readable: ReadableStream<R>; writable: WritableStream<W> }
declare class WritableStream { constructor(underlyingSink?: UnderlyingSink); readonly locked: boolean; close(): Promise<void>; abort(reason?: any): Promise<void>; getWriter(): WritableStreamDefaultWriter }
interface UnderlyingSink { write?: (chunk: any, controller: WritableStreamController) => Promise<void>; close?: () => Promise<void>; abort?: (reason?: any) => Promise<void> }
interface WritableStreamController { error(e: any): void }
declare class WritableStreamDefaultWriter { write(chunk: any): Promise<void>; close(): Promise<void>; abort(reason?: any): Promise<void>; releaseLock(): void; readonly closed: Promise<void>; readonly desiredSize: number | null }

declare function queueMicrotask(callback: () => void): void
declare function structuredClone<T>(value: T, options?: { transfer?: any[] }): T

declare class TextEncoder {
  constructor()
  readonly encoding: string
  encode(input?: string): Uint8Array
  encodeInto(input: string, dest: Uint8Array): { read: number; written: number }
}
declare class TextDecoder {
  constructor(label?: string, options?: { fatal?: boolean })
  readonly encoding: string
  readonly fatal: boolean
  decode(input?: BufferSource, options?: { stream?: boolean }): string
}

declare class URL {
  constructor(url: string, base?: string | URL)
  readonly href: string
  readonly origin: string
  readonly protocol: string
  readonly username: string
  readonly password: string
  readonly host: string
  readonly hostname: string
  readonly port: string
  readonly pathname: string
  readonly search: string
  readonly searchParams: URLSearchParams
  readonly hash: string
  toJSON(): string
  toString(): string
  static canParse(url: string, base?: string): boolean
}
