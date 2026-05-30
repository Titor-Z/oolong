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
