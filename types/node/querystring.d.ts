declare module "node:querystring" {
  export function parse(str: string, sep?: string, eq?: string, options?: { maxKeys?: number; decodeURIComponent?: (s: string) => string }): Record<string, string | string[]>
  export function stringify(obj: Record<string, any>, sep?: string, eq?: string, options?: { encodeURIComponent?: (s: string) => string }): string
  export function escape(str: string): string
  export function unescape(str: string): string
  export function decode(str: string, sep?: string, eq?: string, options?: { maxKeys?: number; decodeURIComponent?: (s: string) => string }): Record<string, string | string[]>
  export function encode(obj: Record<string, any>, sep?: string, eq?: string, options?: { encodeURIComponent?: (s: string) => string }): string
  const _default: { parse: typeof parse; stringify: typeof stringify }
  export default _default
}
