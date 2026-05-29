declare module "node:path" {
  /** 路径分隔符（POSIX: "/", Windows: "\\"） */
  export const sep: string

  /** 路径定界符（POSIX: ":", Windows: ";"） */
  export const delimiter: string

  export function join(...paths: string[]): string
  export function dirname(path: string): string
  export function basename(path: string, ext?: string): string
  export function extname(path: string): string
  export function isAbsolute(path: string): boolean
  export function normalize(path: string): string
  export function relative(from: string, to: string): string
  export function resolve(...paths: string[]): string
  export function parse(path: string): PathParts
  export function format(pathObj: Partial<PathParts>): string
  export function toNamespacedPath(path: string): string

  interface PathParts {
    root: string
    dir: string
    base: string
    ext: string
    name: string
  }

  export const posix: PathPlatform
  export const win32: PathPlatform

  interface PathPlatform {
    sep: string
    delimiter: string
    join(...paths: string[]): string
    dirname(path: string): string
    basename(path: string, ext?: string): string
    extname(path: string): string
    isAbsolute(path: string): boolean
    normalize(path: string): string
    relative(from: string, to: string): string
    resolve(...paths: string[]): string
    parse(path: string): PathParts
    format(pathObj: Partial<PathParts>): string
    toNamespacedPath(path: string): string
  }

  const _default: PathPlatform
  export default _default
}
