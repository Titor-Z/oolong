declare module "node:assert" {
  export function assert(value: any, message?: string): asserts value
  export function ok(value: any, message?: string): asserts value
  export function fail(message?: string): never
  export function fail(actual: any, expected: any, message?: string, operator?: string): never
  export function equal(actual: any, expected: any, message?: string): void
  export function notEqual(actual: any, expected: any, message?: string): void
  export function deepEqual(actual: any, expected: any, message?: string): void
  export function notDeepEqual(actual: any, expected: any, message?: string): void
  export function strictEqual(actual: any, expected: any, message?: string): void
  export function notStrictEqual(actual: any, expected: any, message?: string): void
  export function throws(fn: () => void, error?: RegExp | Function | Object, message?: string): void
  export function doesNotThrow(fn: () => void, message?: string): void
  export function ifError(value: any): asserts value is null | undefined
  export function rejects(asyncFn: Promise<any> | (() => Promise<any>), error?: RegExp | Function | Object, message?: string): Promise<void>
  export function doesNotReject(asyncFn: Promise<any> | (() => Promise<any>), message?: string): Promise<void>
  export class AssertionError extends Error {
    constructor(options?: { message?: string; actual?: any; expected?: any; operator?: string })
    actual: any
    expected: any
    operator: string
    generatedMessage: boolean
  }
  const _default: typeof assert
  export default _default
}
