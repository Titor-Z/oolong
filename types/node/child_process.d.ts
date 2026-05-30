declare module "node:child_process" {
  export function execSync(command: string, options?: { cwd?: string; encoding?: string; timeout?: number; killSignal?: string }): Buffer | string
  export function execFileSync(file: string, args?: string[], options?: { cwd?: string; encoding?: string; timeout?: number; killSignal?: string }): Buffer | string
  export function spawnSync(command: string, args?: string[], options?: { cwd?: string; encoding?: string; timeout?: number; killSignal?: string }): SpawnSyncResult
  export interface SpawnSyncResult {
    status: number | null
    signal: string | null
    stdout: Buffer | string
    stderr: Buffer | string
    error?: Error
  }
}
