declare module "@std/encoding" {
  export namespace base64 {
    export function encode(data: Uint8Array | string): string
    export function decode(str: string): Uint8Array
  }
  export namespace hex {
    export function encode(data: Uint8Array | string): string
    export function decode(str: string): Uint8Array
  }
  const _default: {
    base64: typeof base64
    hex: typeof hex
  }
  export default _default
}
