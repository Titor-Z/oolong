declare class Headers {
  constructor(init?: HeadersInit)
  append(name: string, value: string): void
  delete(name: string): void
  get(name: string): string | null
  getSetCookie(): string[]
  has(name: string): boolean
  set(name: string, value: string): void
  forEach(callback: (value: string, name: string) => void): void
  entries(): IterableIterator<[string, string]>
  keys(): IterableIterator<string>
  values(): IterableIterator<string>
  [Symbol.iterator](): IterableIterator<[string, string]>
}
type HeadersInit = Headers | Record<string, string> | [string, string][]
