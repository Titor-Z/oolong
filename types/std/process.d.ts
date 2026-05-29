declare module "@std/process" {
  export function cwd(): string
  export function chdir(dir: string): void
  export const pid: number
  export const ppid: number
  export const platform: string
  export const arch: string
  export const title: string
  export const version: string
  export const versions: Record<string, string>
  export const execPath: string
  export const execArgv: string[]
  export const env: Record<string, string | undefined>
  export const argv: string[]
  export function exit(code?: number): void
  export function uptime(): number
  export function memoryUsage(): { rss: number; heapTotal: number; heapUsed: number }

  // 标准流
  export const stdout: { write(data: string): void }
  export const stderr: { write(data: string): void }
  export const stdin: {
    read(): Promise<string>
    readAsBytes(): Promise<Uint8Array>
  }

  const _default: {
    cwd: typeof cwd
    chdir: typeof chdir
    pid: number
    ppid: number
    platform: string
    arch: string
    title: string
    version: string
    versions: Record<string, string>
    execPath: string
    execArgv: string[]
    env: Record<string, string | undefined>
    argv: string[]
    exit: typeof exit
    uptime: typeof uptime
    memoryUsage: typeof memoryUsage
    stdout: { write(data: string): void }
    stderr: { write(data: string): void }
    stdin: { read(): Promise<string>; readAsBytes(): Promise<Uint8Array> }
  }
  export default _default
}
