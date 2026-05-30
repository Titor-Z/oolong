// ── Queuing Strategy ─────────────────────────────────────────────────────────

interface QueuingStrategy<T = any> {
  highWaterMark?: number
  size?: (chunk: T) => number
}

declare class CountQueuingStrategy {
  constructor(init: { highWaterMark: number })
  readonly highWaterMark: number
  size(chunk?: any): number
}

declare class ByteLengthQueuingStrategy {
  constructor(init: { highWaterMark: number })
  readonly highWaterMark: number
  size(chunk: ArrayBufferView | ArrayBufferLike): number
}

// ── ReadableStream ───────────────────────────────────────────────────────────

interface ReadableStreamController<T = any> {
  readonly desiredSize: number | null
}

declare class ReadableStream<R = any> {
  constructor(underlyingSource?: UnderlyingSource<R>, strategy?: QueuingStrategy<R>)
  readonly locked: boolean
  cancel(reason?: any): Promise<void>
  getReader(): ReadableStreamDefaultReader<R>
  pipeThrough<T>(transform: ReadableWritablePair<T, R>, options?: StreamPipeOptions): ReadableStream<T>
  pipeTo(dest: WritableStream<R>, options?: StreamPipeOptions): Promise<void>
  tee(): [ReadableStream<R>, ReadableStream<R>]
}

interface UnderlyingSource<R = any> {
  start?: (controller: ReadableStreamController<R>) => void | Promise<void>
  pull?: (controller: ReadableStreamController<R>) => void | Promise<void>
  cancel?: (reason?: any) => void | Promise<void>
  type?: "bytes"
}

interface StreamPipeOptions {
  preventClose?: boolean
  preventAbort?: boolean
  preventCancel?: boolean
  signal?: AbortSignal
}

interface ReadableWritablePair<R = any, W = any> {
  readable: ReadableStream<R>
  writable: WritableStream<W>
}

declare class ReadableStreamDefaultReader<R = any> {
  readonly closed: Promise<void>
  cancel(reason?: any): Promise<void>
  read(): Promise<{ done: boolean; value?: R }>
  releaseLock(): void
}

// ── WritableStream ───────────────────────────────────────────────────────────

interface WritableStreamController<W = any> {
  error(e?: any): void
}

declare class WritableStream<W = any> {
  constructor(underlyingSink?: UnderlyingSink<W>, strategy?: QueuingStrategy<W>)
  readonly locked: boolean
  abort(reason?: any): Promise<void>
  close(): Promise<void>
  getWriter(): WritableStreamDefaultWriter<W>
}

interface UnderlyingSink<W = any> {
  start?: (controller: WritableStreamController<W>) => void | Promise<void>
  write?: (chunk: W, controller: WritableStreamController<W>) => void | Promise<void>
  close?: () => void | Promise<void>
  abort?: (reason?: any) => void | Promise<void>
}

declare class WritableStreamDefaultWriter<W = any> {
  readonly closed: Promise<void>
  readonly desiredSize: number | null
  abort(reason?: any): Promise<void>
  close(): Promise<void>
  releaseLock(): void
  write(chunk?: W): Promise<void>
}

// ── TransformStream ──────────────────────────────────────────────────────────

declare class TransformStream<I = any, O = any> {
  constructor(transformer?: Transformer<I, O>, writableStrategy?: QueuingStrategy<I>, readableStrategy?: QueuingStrategy<O>)
  readonly readable: ReadableStream<O>
  readonly writable: WritableStream<I>
}

interface Transformer<I = any, O = any> {
  start?: (controller: TransformStreamDefaultController<O>) => void | Promise<void>
  transform?: (chunk: I, controller: TransformStreamDefaultController<O>) => void | Promise<void>
  flush?: (controller: TransformStreamDefaultController<O>) => void | Promise<void>
}

declare class TransformStreamDefaultController<O = any> {
  readonly desiredSize: number | null
  enqueue(chunk?: O): void
  error(reason?: any): void
  terminate(): void
}
