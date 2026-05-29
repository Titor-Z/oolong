declare module "@std/os" {
  // ── P0 ────────────────────────────────────────────────────
  export const EOL: string
  export function platform(): string
  export function arch(): string
  export function endianness(): "LE" | "BE"

  // ── P1 ────────────────────────────────────────────────────
  export function hostname(): string
  export function type(): string
  export function release(): string
  export function homedir(): string
  export function tmpdir(): string

  // ── P2 ────────────────────────────────────────────────────
  export function totalmem(): number
  export function freemem(): number
  export function uptime(): number
  export function loadavg(): [number, number, number]
  export function cpus(): CpuInfo[]

  interface CpuInfo {
    model: string
    speed: number
    times: {
      user: number
      nice: number
      sys: number
      idle: number
      irq: number
    }
  }

  const _default: {
    EOL: string
    platform: typeof platform
    arch: typeof arch
    endianness: typeof endianness
    hostname: typeof hostname
    type: typeof type
    release: typeof release
    homedir: typeof homedir
    tmpdir: typeof tmpdir
    totalmem: typeof totalmem
    freemem: typeof freemem
    uptime: typeof uptime
    loadavg: typeof loadavg
    cpus: typeof cpus
  }
  export default _default
}
