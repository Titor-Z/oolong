declare class URLSearchParams {
  constructor(init?: string | URLSearchParams | Record<string, string> | [string, string][])
  append(name: string, value: string): void
  delete(name: string): void
  get(name: string): string | null
  getAll(name: string): string[]
  has(name: string): boolean
  set(name: string, value: string): void
  sort(): void
  toString(): string
  forEach(callback: (value: string, name: string) => void): void
  entries(): IterableIterator<[string, string]>
  keys(): IterableIterator<string>
  values(): IterableIterator<string>
  [Symbol.iterator](): IterableIterator<[string, string]>
  size: number
}
