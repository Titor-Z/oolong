declare module "@std/path" {
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

  interface PathParts {
    root: string
    dir: string
    base: string
    ext: string
    name: string
  }

  const _default: {
    sep: string
    delimiter: string
    join: typeof join
    dirname: typeof dirname
    basename: typeof basename
    extname: typeof extname
    isAbsolute: typeof isAbsolute
    normalize: typeof normalize
    relative: typeof relative
    resolve: typeof resolve
    parse: typeof parse
    format: typeof format
  }
  export default _default
}
