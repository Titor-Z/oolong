declare module "node:url" {
  export function parse(url: string, parseQueryString?: boolean, slashesDenoteHost?: boolean): Url
  export function format(url: Url, options?: { fragment?: boolean; unicode?: boolean; auth?: boolean }): string
  export function resolve(from: string, to: string): string
  export function fileURLToPath(url: string | URL): string
  export function pathToFileURL(path: string): URL
  export interface Url {
    protocol: string | null
    slashes: boolean | null
    auth: string | null
    host: string | null
    port: string | null
    hostname: string | null
    hash: string | null
    search: string | null
    query: string | Record<string, string> | null
    pathname: string | null
    path: string | null
    href: string
  }
  const _default: { parse: typeof parse; format: typeof format; resolve: typeof resolve }
  export default _default
}
