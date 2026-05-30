declare module "node:os" {
  export function hostname(): string
  export function platform(): NodeJS.Platform
  export function arch(): string
  export function type(): string
  export function release(): string
  export function homedir(): string
  export function tmpdir(): string
  export function cpus(): CpuInfo[]
  export function totalmem(): number
  export function freemem(): number
  export function uptime(): number
  export function loadavg(): number[]
  export function networkInterfaces(): Record<string, NetworkInterfaceInfo[]>
  export function userInfo(options?: { encoding: string }): UserInfo
  export function version(): string
  export function machine(): string
  export function endianness(): "BE" | "LE"
  export const EOL: string
  export interface CpuInfo { model: string; speed: number; times: { user: number; nice: number; sys: number; idle: number; irq: number } }
  export interface NetworkInterfaceInfo { address: string; netmask: string; family: string; mac: string; internal: boolean; cidr: string | null }
  export interface UserInfo { username: string; uid: number; gid: number; shell: string | null; homedir: string }
  const _default: typeof os
  export default _default
}
declare namespace NodeJS { type Platform = "darwin" | "linux" | "win32" | "freebsd" | "openbsd" | "netbsd" | "aix" | "android" | "cygwin" }
