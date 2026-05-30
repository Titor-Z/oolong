declare module "node:stream" {
  import { EventEmitter } from "node:events"

  export class Stream extends EventEmitter {
    pipe<T extends Writable>(destination: T, options?: { end?: boolean }): T
    destroy(error?: Error): this
  }

  export class Readable extends Stream {
    constructor(opts?: ReadableOptions)
    read(size?: number): any
    push(chunk: any, encoding?: string): boolean
    unshift(chunk: any, encoding?: string): void
    resume(): this
    pause(): this
    isPaused(): boolean
    readableEnded: boolean
    readableFlowing: boolean | null
    readableHighWaterMark: number
    readableLength: number
    readableObjectMode: boolean
    destroy(error?: Error): this
    _read(size: number): void
    _destroy(error: Error | null, callback: (error?: Error | null) => void): void
    static from(data: Iterable<any> | AsyncIterable<any>, opts?: ReadableOptions): Readable
  }

  export class Writable extends Stream {
    constructor(opts?: WritableOptions)
    write(chunk: any, encoding?: string, cb?: (error?: Error | null) => void): boolean
    end(cb?: () => void): this
    end(chunk: any, cb?: () => void): this
    end(chunk: any, encoding?: string, cb?: () => void): this
    cork(): void
    uncork(): void
    writableEnded: boolean
    writableFinished: boolean
    writableHighWaterMark: number
    writableLength: number
    writableObjectMode: boolean
    writableCorked: number
    destroy(error?: Error): this
    _write(chunk: any, encoding: string, callback: (error?: Error | null) => void): void
    _final(callback: (error?: Error | null) => void): void
    _destroy(error: Error | null, callback: (error?: Error | null) => void): void
  }

  export class Duplex extends Readable {
    constructor(opts?: DuplexOptions)
    write(chunk: any, encoding?: string, cb?: (error?: Error | null) => void): boolean
    end(cb?: () => void): this
    end(chunk: any, cb?: () => void): this
    end(chunk: any, encoding?: string, cb?: () => void): this
    cork(): void
    uncork(): void
    _write(chunk: any, encoding: string, callback: (error?: Error | null) => void): void
    _final(callback: (error?: Error | null) => void): void
  }

  export class Transform extends Duplex {
    constructor(opts?: TransformOptions)
    _transform(chunk: any, encoding: string, callback: (error?: Error | null) => void): void
    _flush(callback: (error?: Error | null) => void): void
  }

  export class PassThrough extends Transform {}

  export function pipeline<T extends Writable>(source: Readable, destination: T, callback?: (err?: Error | null) => void): T
  export function pipeline<T extends Writable>(...streams: (Readable | Writable | ((err?: Error | null) => void))[]): Writable
  export function finished(stream: Readable | Writable, callback: (err?: Error | null) => void): () => void

  interface ReadableOptions {
    highWaterMark?: number
    encoding?: string
    objectMode?: boolean
    read?: (this: Readable, size: number) => void
    destroy?: (this: Readable, error: Error | null, callback: (error?: Error | null) => void) => void
  }

  interface WritableOptions {
    highWaterMark?: number
    objectMode?: boolean
    decodeStrings?: boolean
    write?: (this: Writable, chunk: any, encoding: string, callback: (error?: Error | null) => void) => void
    final?: (this: Writable, callback: (error?: Error | null) => void) => void
    destroy?: (this: Writable, error: Error | null, callback: (error?: Error | null) => void) => void
  }

  interface DuplexOptions extends ReadableOptions, WritableOptions {}

  interface TransformOptions extends DuplexOptions {
    transform?: (this: Transform, chunk: any, encoding: string, callback: (error?: Error | null) => void) => void
    flush?: (this: Transform, callback: (error?: Error | null) => void) => void
  }
}
