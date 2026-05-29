declare module "@std/fs" {
  /** 文件信息 */
  interface FileInfo {
    isFile: boolean
    isDirectory: boolean
    isSymlink: boolean
    size: number
    mtimeMs: number
    atimeMs: number
    birthtimeMs: number
    mode: number
  }

  // ── P0 ────────────────────────────────────────────────────
  export function readFile(path: string, signal?: AbortSignal): Promise<ArrayBuffer>
  export function readTextFile(path: string, signal?: AbortSignal): Promise<string>
  export function readFileSync(path: string): ArrayBuffer
  export function writeFile(path: string, data: string | ArrayBuffer | Uint8Array, signal?: AbortSignal): Promise<void>
  export function writeTextFile(path: string, text: string, signal?: AbortSignal): Promise<void>
  export function exists(path: string): Promise<boolean>

  // ── P1 ────────────────────────────────────────────────────
  export function mkdir(path: string, options?: { recursive?: boolean }, signal?: AbortSignal): Promise<void>
  export function remove(path: string, options?: { recursive?: boolean }, signal?: AbortSignal): Promise<void>
  export function readdir(path: string, signal?: AbortSignal): Promise<string[]>
  export function stat(path: string, signal?: AbortSignal): Promise<FileInfo>
  export function lstat(path: string, signal?: AbortSignal): Promise<FileInfo>
  export function appendFile(path: string, data: string | ArrayBuffer | Uint8Array, signal?: AbortSignal): Promise<void>
  export function copyFile(src: string, dst: string, signal?: AbortSignal): Promise<void>
  export function rename(oldPath: string, newPath: string, signal?: AbortSignal): Promise<void>
  export function realpath(path: string, signal?: AbortSignal): Promise<string>
  export function symlink(target: string, path: string, signal?: AbortSignal): Promise<void>

  // ── P2 (Sync) ─────────────────────────────────────────────
  export function existsSync(path: string): boolean
  export function mkdirSync(path: string, options?: { recursive?: boolean }): void
  export function removeSync(path: string, options?: { recursive?: boolean }): void
  export function readdirSync(path: string): string[]
  export function statSync(path: string): FileInfo
  export function lstatSync(path: string): FileInfo
  export function appendFileSync(path: string, data: string | ArrayBuffer | Uint8Array): void
  export function copyFileSync(src: string, dst: string): void
  export function renameSync(oldPath: string, newPath: string): void
  export function realpathSync(path: string): string
  export function symlinkSync(target: string, path: string): void

  // ── P3 ────────────────────────────────────────────────────
  export function chmod(path: string, mode: number, signal?: AbortSignal): Promise<void>
  export function chown(path: string, uid: number, gid: number, signal?: AbortSignal): Promise<void>
  export function link(existingPath: string, newPath: string, signal?: AbortSignal): Promise<void>
  export function truncate(path: string, len?: number, signal?: AbortSignal): Promise<void>
  export function access(path: string, mode?: number, signal?: AbortSignal): Promise<void>
  export function watch(path: string): AsyncIterableIterator<{ kind: string; paths: string[] }>

  const _default: {
    readFile: typeof readFile
    readTextFile: typeof readTextFile
    readFileSync: typeof readFileSync
    writeFile: typeof writeFile
    writeTextFile: typeof writeTextFile
    exists: typeof exists
    mkdir: typeof mkdir
    remove: typeof remove
    readdir: typeof readdir
    stat: typeof stat
    lstat: typeof lstat
    appendFile: typeof appendFile
    copyFile: typeof copyFile
    rename: typeof rename
    realpath: typeof realpath
    symlink: typeof symlink
    existsSync: typeof existsSync
    mkdirSync: typeof mkdirSync
    removeSync: typeof removeSync
    readdirSync: typeof readdirSync
    statSync: typeof statSync
    lstatSync: typeof lstatSync
    appendFileSync: typeof appendFileSync
    copyFileSync: typeof copyFileSync
    renameSync: typeof renameSync
    realpathSync: typeof realpathSync
    symlinkSync: typeof symlinkSync
    chmod: typeof chmod
    chown: typeof chown
    link: typeof link
    truncate: typeof truncate
    access: typeof access
    watch: typeof watch
  }
  export default _default
}
