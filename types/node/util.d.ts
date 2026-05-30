declare module "node:util" {
  export function promisify<F extends Function>(original: F): F
  export function callbackify<F extends Function>(original: F): F
  export function format(format: string, ...args: any[]): string
  export function inspect(obj: any, options?: InspectOptions): string
  export function deprecate<F extends Function>(fn: F, msg: string): F
  export function inherits(constructor: Function, superConstructor: Function): void
  export function debuglog(section: string): (msg: string, ...args: any[]) => void

  export namespace types {
    function isDate(value: any): value is Date
    function isRegExp(value: any): value is RegExp
    function isArrayBuffer(value: any): value is ArrayBuffer
    function isMap(value: any): value is Map<any, any>
    function isSet(value: any): value is Set<any>
    function isNativeError(value: any): value is Error
    function isTypedArray(value: any): value is Uint8Array | Uint16Array | Uint32Array | Int8Array | Int16Array | Int32Array | Float32Array | Float64Array
  }

  interface InspectOptions {
    depth?: number
    showHidden?: boolean
    colors?: boolean
    compact?: boolean
    maxArrayLength?: number
  }

  const _default: {
    promisify: typeof promisify
    callbackify: typeof callbackify
    format: typeof format
    inspect: typeof inspect
    deprecate: typeof deprecate
    inherits: typeof inherits
    debuglog: typeof debuglog
    types: typeof types
  }
  export default _default
}
