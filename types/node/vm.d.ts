declare module "node:vm" {
  export function runInThisContext(code: string, options?: { filename?: string }): any
  export function runInNewContext(code: string, sandbox?: Record<string, any>, options?: { filename?: string }): any
  export function compileFunction(code: string, params?: string[], options?: { filename?: string }): Function
  export class Script {
    constructor(code: string, options?: { filename?: string })
    runInThisContext(): any
    runInNewContext(sandbox?: Record<string, any>): any
  }
  const _default: { runInThisContext: typeof runInThisContext; runInNewContext: typeof runInNewContext; Script: typeof Script }
  export default _default
}
