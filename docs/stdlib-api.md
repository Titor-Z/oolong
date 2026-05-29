# OOLONG 标准库 API 规范 v1

> 每模块对照 Node.js / Deno / Bun 三家的 API 梳理，标记实现优先级。
>
> - ✅ 已实现
> - 🔜 待实现
> - ❌ 不实现（理由）
> - N/A 该运行时无此概念

---

## 1. `import "path"` — 路径操作

参考：[Node path](https://nodejs.org/api/path.html)、Deno std/path、Bun path

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `join(...paths)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `dirname(path)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `basename(path, ext?)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `extname(path)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `isAbsolute(path)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `normalize(path)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `relative(from, to)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `resolve(...paths)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `parse(path)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `format(pathObj)` | ✅ | ✅ | ✅ | ✅ | P0 |
| `sep` | ✅ | ✅ | ✅ | ✅ | P0 |
| `delimiter` | ✅ | ✅ | ✅ | ✅ | P0 |
| `posix` 子模块 | ✅ | ✅ | ✅ | ❌（平台自适应）| — |
| `win32` 子模块 | ✅ | ✅ | ✅ | ❌（平台自适应）| — |
| `toNamespacedPath()` | ✅ | ❌ | ❌ | ❌（仅 Windows）| — |
| `matchesGlob(pattern)` | ⚠️ 实验 | ❌ | ❌ | ❌ | — |

**实现状态**：✅ 全部 P0 已完成（24 测试）

---

## 2. `import "process"` — 进程信息

参考：[Node process](https://nodejs.org/api/process.html)、Deno 全局 `Deno.*`、Bun 全局 `process`

### 2.1 基本信息

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `cwd()` | ✅ global | `Deno.cwd()` | ✅ global | ✅ | P0 |
| `chdir(dir)` | ✅ global | `Deno.chdir()` | ✅ global | ✅ | P1 |
| `pid` | ✅ global | `Deno.pid` | ✅ global | ✅ | P0 |
| `ppid` | ✅ global | ❌ | ✅ global | ✅ | P1 |
| `platform` | ✅ global | `Deno.build.os` | ✅ global | ✅ | P0 |
| `arch` | ✅ global | `Deno.build.arch` | ✅ global | ✅ | P0 |
| `title` | ✅ global | ❌ | ✅ global | ✅ | P2 |
| `version` | ✅ global | `Deno.version.deno` | ✅ global | ✅ | P1 |
| `versions` | ✅ global | `Deno.version` | ✅ global | ✅ | P1 |
| `execPath` | ✅ global | `Deno.execPath()` | ✅ global | ✅ | P1 |
| `execArgv` | ✅ global | ❌ | ✅ global | ✅ | P2 |

### 2.2 环境与参数

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `env` | ✅ global | `Deno.env.toObject()` | ✅ global | ✅ | P0 |
| `argv` | ✅ global | `Deno.args` | ✅ global | ✅ | P0 |
| `exit(code?)` | ✅ global | `Deno.exit()` | ✅ global | ✅ | P0 |
| `kill(pid, signal?)` | ✅ global | `Deno.kill()` | ✅ global | ❌ P3 | — |
| `abort()` | ✅ global | ❌ | ✅ global | ❌ P3 | — |
| `umask(mask?)` | ✅ global | ❌ | ✅ global | ❌ P3 | — |
| `uptime()` | ✅ global | `Deno.uptime()` | ✅ global | ✅ | P2 |
| `hrtime()` | ✅ global | `performance.now()` | ✅ global | ❌（用 `performance`）| — |

### 2.3 标准流

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `stdout` | ✅ global | `Deno.stdout` | ✅ global | ✅（write 接口）| P1 |
| `stderr` | ✅ global | `Deno.stderr` | ✅ global | ✅（write 接口）| P1 |
| `stdin` | ✅ global | `Deno.stdin` | ✅ global | ✅（read/readAsBytes）| P1 |

### 2.4 异步调度

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `nextTick(fn)` | ✅ global | ⚠️ `queueMicrotask` | ✅ global | ❌（用 `queueMicrotask` / `Promise`）| — |
| `on(event, handler)` | ✅ global | ❌ | ✅ global | ❌（EventTarget 替代）| — |
| `emit(event, ...args)` | ✅ global | ❌ | ✅ global | ❌ | — |

### 2.5 内存与资源

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `memoryUsage()` | ✅ global | `Deno.memoryUsage()` | ✅ global | ✅（rss 为占位）| P2 |
| `cpuUsage()` | ✅ global | ❌ | ✅ global | ❌ | — |
| `resourceUsage()` | ✅ global | ❌ | ❌ | ❌ | — |

### 2.6 用户/组

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `getuid()` / `getgid()` | ✅ | ❌ | ✅ | ❌ | — |
| `geteuid()` / `getegid()` | ✅ | ❌ | ✅ | ❌ | — |
| `getgroups()` | ✅ | ❌ | ✅ | ❌ | — |

**实现状态**：✅ P0+P1+P2 已完成（chdir/ppid/version/versions/execPath/stdout/stderr/execArgv/title/uptime/memoryUsage）；P3 暂不实现（umask/kill/abort）

---

## 3. `import "fs"` — 文件系统

参考：[Node fs](https://nodejs.org/api/fs.html)、Deno `Deno.readTextFile` 等、Bun `Bun.file()` + `node:fs`

**设计原则**：
- 异步优先：`readFile(path)` → `Promise<ArrayBuffer>`
- 同步版加 `Sync` 后缀：`readFileSync(path)` → `ArrayBuffer`（与 Deno 对齐）
- 纯函数风格，无 callback（`node:fs` 才提供 callback）

### P0 — 核心读写

| API | Node | Deno | Bun | OOLONG |
|-----|------|------|-----|--------|
| `readFile(path)` | ✅ | `Deno.readFile()` | `Bun.file().arrayBuffer()` | ✅ |
| `readTextFile(path)` | ⚠️ `utf8` | `Deno.readTextFile()` | `Bun.file().text()` | ✅ |
| `readFileSync(path)` | ✅ | ❌ | ❌ | ✅ |
| `writeFile(path, data)` | ✅ | `Deno.writeFile()` | `Bun.write()` | ✅ |
| `writeTextFile(path, text)` | ⚠️ utf8 | `Deno.writeTextFile()` | `Bun.write()` | ✅ |
| `exists(path)` | ❌已弃用 | `Deno.stat()` | `Bun.file().exists()` | ✅ |

### P1 — 目录和元信息

| API | Node | Deno | Bun | OOLONG |
|-----|------|------|-----|--------|
| `mkdir(path, opts?)` | ✅ | `Deno.mkdir()` | ✅ | ✅ |
| `remove(path, opts?)` | ✅ `rm` | `Deno.remove()` | ✅ | ✅ |
| `readdir(path)` | ✅ | `Deno.readDir()` | ✅ | ✅ |
| `stat(path)` | ✅ | `Deno.stat()` | `Bun.file().stat()` | ✅ |
| `lstat(path)` | ✅ | `Deno.lstat()` | ✅ | ✅ |
| `appendFile(path, data)` | ✅ | ❌ | ✅ | ✅ |
| `copyFile(src, dst)` | ✅ | `Deno.copyFile()` | ✅ | ✅ |
| `rename(old, new)` | ✅ | `Deno.rename()` | ✅ | ✅ |
| `realpath(path)` | ✅ | `Deno.realPath()` | ✅ | ✅ |
| `symlink(target, path)` | ✅ | `Deno.symlink()` | ✅ | ✅ |

### P2 — 同步版补齐

| API | Node | Deno | Bun | OOLONG |
|-----|------|------|-----|--------|
| `existsSync(path)` | ✅ | ❌ | ✅ | ✅ |
| `mkdirSync(path, opts?)` | ✅ | ❌ | ✅ | ✅ |
| `removeSync(path, opts?)` | ✅ | ❌ | ✅ | ✅ |
| `readdirSync(path)` | ✅ | ❌ | ✅ | ✅ |
| `statSync(path)` | ✅ | ❌ | ✅ | ✅ |
| `lstatSync(path)` | ✅ | ❌ | ✅ | ✅ |
| `appendFileSync(path, data)` | ✅ | ❌ | ✅ | ✅ |
| `copyFileSync(src, dst)` | ✅ | ❌ | ✅ | ✅ |
| `renameSync(old, new)` | ✅ | ❌ | ✅ | ✅ |
| `realpathSync(path)` | ✅ | ❌ | ✅ | ✅ |
| `symlinkSync(target, path)` | ✅ | ❌ | ✅ | ✅ |

### P3 — 进阶操作

| API | Node | Deno | Bun | OOLONG |
|-----|------|------|-----|--------|
| `chmod(path, mode)` | ✅ | `Deno.chmod()` | ✅ | ✅ |
| `chown(path, uid, gid)` | ✅ | `Deno.chown()` | ✅ | ✅（Unix only） |
| `link(existing, new)` | ✅ | `Deno.link()` | ✅ | ✅ |
| `truncate(path, len)` | ✅ | `Deno.truncate()` | ✅ | ✅ |
| `access(path, mode?)` | ✅ | `Deno.lstat()` | ✅ | ❌ |
| `watch(path)` | ✅ | `Deno.watchFs()` | ✅ | ❌ |

**实现状态**：✅ P0+P1+P2+P3 已完成（共 32 API）；`access` 和 `watch` 暂不实现

### 不实现

`open` / `close` / `read` / `write`（fd 模型）、`createReadStream` / `createWriteStream`（Streams API 替代）

---

## 4. `import "os"` — 操作系统信息

参考：[Node os](https://nodejs.org/api/os.html)

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `platform()` | ✅ | `Deno.build.os` | ✅ | ✅ | P0 |
| `arch()` | ✅ | `Deno.build.arch` | ✅ | ✅ | P0 |
| `hostname()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `type()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `release()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `cpus()` | ✅ | ❌ | ✅ | ❌ P3 | — |
| `totalmem()` / `freemem()` | ✅ | ❌ | ✅ | ✅ | P2 |
| `homedir()` | ✅ | `Deno.env.get("HOME")` | ✅ | ✅ | P1 |
| `tmpdir()` | ✅ | `Deno.env.get("TMPDIR")` | ✅ | ✅ | P1 |
| `uptime()` | ✅ | `Deno.uptime()` | ✅ | ❌（放 process） | — |
| `loadavg()` | ✅ | ❌ | ✅ | ❌ P3 | — |
| `networkInterfaces()` | ✅ | ❌ | ✅ | ❌ P3 | — |
| `userInfo()` | ✅ | ❌ | ✅ | ❌ P3 | — |
| `EOL` | ✅ | ✅ | ✅ | ✅ | P0 |
| `endianness()` | ✅ | ❌ | ✅ | ❌ P3 | — |
| `devNull` | ✅ | ❌ | ✅ | ❌ P3 | — |

**实现状态**：✅ P0+P1+P2 已完成（platform/arch/EOL/hostname/type/release/homedir/tmpdir/totalmem/freemem）；P3 暂不实现（cpus/loadavg/networkInterfaces/userInfo/endianness/devNull）

---

## 5. W3C 全局 API（`globalThis`）

参考：Boa 内置 + Deno web API + 浏览器

| API | Boa 内置 | Deno | Bun | OOLONG | 优先级 |
|-----|----------|------|-----|--------|--------|
| `console` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P0 |
| `setTimeout` / `clearTimeout` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P0 |
| `setInterval` / `clearInterval` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P0 |
| `queueMicrotask` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P0 |
| `URL` / `URLSearchParams` | boa_runtime | ✅ | ✅ | ✅ 已注册（URLSearchParams 自实现） | P1 |
| `TextEncoder` / `TextDecoder` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P1 |
| `fetch` / `Request` / `Response` | boa_runtime（BlockingReqwestFetcher） | ✅ | ✅ | ✅ 已注册 | P1 |
| `performance` | ❌ | ✅ | ✅ | 🔜 | P2 |
| `structuredClone` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P2 |
| `Blob` | ❌（自实现） | ✅ | ✅ | ✅ 已注册 | P1 |
| `File` | ❌（自实现） | ✅ | ✅ | ✅ 已注册 | P2 |
| `ReadableStream` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `crypto.subtle` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `atob` / `btoa` | ❌ | ✅ | ✅ | 🔜 | P1 |
| `AbortController` / `AbortSignal` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `Event` / `EventTarget` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `WebSocket` | ❌ | ✅ | ✅ | ❌ P3 | — |

**实现状态**：基础定时器已完成，其余待注册/实现

---

## 6. Node 兼容层（`node:` 前缀）

| 模块 | 策略 |
|------|------|
| `node:fs` | 包装 W3C `fs`，加 callback 风格 |
| `node:path` | 重新导出 W3C `path`，加 `win32` / `posix` |
| `node:process` | 包装全局 `process`（Node 风格） |
| `node:os` | 同步版本，与 W3C `os` 对齐 |
| `node:buffer` | Buffer 实现 |
| `node:events` | EventEmitter |
| `node:stream` | Stream 兼容 |
| `node:util` | util 工具函数 |

**实现状态**：🔜 待 W3C 模块稳定后开始
