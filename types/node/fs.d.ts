declare module "node:fs" {
  export function existsSync(path: string): boolean
  export function readFileSync(path: string, encoding?: string): string | Buffer
  export function readFileSync(path: string, options: { encoding?: string; flag?: string }): string | Buffer
  export function writeFileSync(path: string, data: string | Buffer, options?: { encoding?: string; flag?: string }): void
  export function mkdirSync(path: string, options?: { recursive?: boolean }): string | undefined
  export function rmdirSync(path: string, options?: { recursive?: boolean }): void
  export function unlinkSync(path: string): void
  export function readdirSync(path: string, encoding?: string): string[]
  export function readdirSync(path: string, options: { encoding?: string; withFileTypes?: boolean }): string[] | Dirent[]
  export function statSync(path: string): Stats
  export function lstatSync(path: string): Stats
  export function symlinkSync(target: string, path: string): void
  export function readlinkSync(path: string): string
  export function copyFileSync(src: string, dest: string, mode?: number): void
  export function renameSync(oldPath: string, newPath: string): void
  export function chmodSync(path: string, mode: number | string): void
  export function mkdtempSync(prefix: string): string
  export function realpathSync(path: string): string
  export function accessSync(path: string, mode?: number): void
  export function appendFileSync(path: string, data: string | Buffer, options?: { encoding?: string }): void
  export function truncateSync(path: string, len?: number): void
  export function watchFile(path: string, listener: (curr: Stats, prev: Stats) => void): void
  export class Stats {
    isFile(): boolean; isDirectory(): boolean; isSymbolicLink(): boolean
    size: number; mtimeMs: number; atimeMs: number; ctimeMs: number; birthtimeMs: number
  }
  export class Dirent {
    name: string; isFile(): boolean; isDirectory(): boolean; isSymbolicLink(): boolean
  }
  export const constants: { F_OK: number; R_OK: number; W_OK: number; X_OK: number }
}
