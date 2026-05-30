declare module "@std/semver" {
  export interface SemVer {
    major: number
    minor: number
    patch: number
    prerelease: string[]
    build: string[]
    toString(): string
  }

  export function parse(str: string): SemVer
  export function format(sv: SemVer): string
  export function compare(a: string | SemVer, b: string | SemVer): -1 | 0 | 1
  export function greaterThan(a: string | SemVer, b: string | SemVer): boolean
  export function lessThan(a: string | SemVer, b: string | SemVer): boolean
  export function equals(a: string | SemVer, b: string | SemVer): boolean
  export function satisfies(version: string, range: string): boolean

  const _default: {
    parse: typeof parse
    format: typeof format
    compare: typeof compare
    greaterThan: typeof greaterThan
    lessThan: typeof lessThan
    equals: typeof equals
    satisfies: typeof satisfies
  }
  export default _default
}
