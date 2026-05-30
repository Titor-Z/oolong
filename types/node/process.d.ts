declare module "node:process" {
  export const argv: string[]
  export const env: Record<string, string | undefined>
  export const cwd: () => string
  export const chdir: (dir: string) => void
  export const exit: (code?: number) => never
  export const pid: number
  export const ppid: number
  export const platform: string
  export const arch: string
  export const version: string
  export const versions: Record<string, string>
  export const stdout: NodeJS.WriteStream
  export const stderr: NodeJS.WriteStream
  export const stdin: NodeJS.ReadStream
  export function nextTick(cb: (...args: any[]) => void, ...args: any[]): void
  export function hrtime(): [number, number]
  export function hrtime(time: [number, number]): [number, number]
  export function uptime(): number
  export function memoryUsage(): { rss: number; heapTotal: number; heapUsed: number; external: number }
  export function exitCode: number
  export const stderr: any
  export const stdout: any
  export const stdin: any
}
declare namespace NodeJS {
  interface WriteStream { write: (data: any, ...args: any[]) => boolean }
  interface ReadStream { on: (event: string, cb: (...args: any[]) => void) => void }
}
