# OOLONG 标准库 API 规范 v2

> 每模块对照 Node.js / Deno / Bun 三家的 API 梳理，标记实现优先级。
>
> 三元体系：
> - `web/` — W3C Web API（全局类）
> - `std/` — OOLONG 原生模块（自定 API 面，参考三家设计）
> - `node/` — Node.js 兼容层（`node:` 前缀，完整 Node API）
>
> - ✅ 已实现
> - 🔜 待实现（已排期）
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

> **设计说明**：OOLONG 原生 `os` 模块不是 W3C 标准（浏览器无此模块）。
> 自定 API 面，参考 Deno/Bun/Node 三家。Node 有但此处暂缺的（如 `cpus()`），
> 后续会统一评估加入，而不是只藏在 `node:os` 里。

参考：[Node os](https://nodejs.org/api/os.html)

| API | Node | Deno | Bun | OOLONG | 优先级 |
|-----|------|------|-----|--------|--------|
| `platform()` | ✅ | `Deno.build.os` | ✅ | ✅ | P0 |
| `arch()` | ✅ | `Deno.build.arch` | ✅ | ✅ | P0 |
| `hostname()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `type()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `release()` | ✅ | ❌ | ✅ | ✅ | P1 |
| `cpus()` | ✅ | ❌ | ✅ | ✅ | P2 |
| `totalmem()` / `freemem()` | ✅ | ❌ | ✅ | ✅ | P2 |
| `homedir()` | ✅ | `Deno.env.get("HOME")` | ✅ | ✅ | P1 |
| `tmpdir()` | ✅ | `Deno.env.get("TMPDIR")` | ✅ | ✅ | P1 |
| `uptime()` | ✅ | `Deno.uptime()` | ✅ | ✅ | P2 |
| `loadavg()` | ✅ | ❌ | ✅ | ✅ | P2 |
| `networkInterfaces()` | ✅ | ❌ | ✅ | 🔜 P3 | — |
| `userInfo()` | ✅ | ❌ | ✅ | 🔜 P3 | — |
| `EOL` | ✅ | ✅ | ✅ | ✅ | P0 |
| `endianness()` | ✅ | ❌ | ✅ | ✅ | P0 |
| `devNull` | ✅ | ❌ | ✅ | 🔜 P3 | — |

**实现状态**：✅ P0+P1+P2 已完成（platform/arch/EOL/hostname/type/release/homedir/tmpdir/totalmem/freemem/cpus/uptime/loadavg/endianness）

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
| `performance` | ❌ | ✅ | ✅ | ✅ | P2 |
| `structuredClone` | boa_runtime | ✅ | ✅ | ✅ 已注册 | P2 |
| `Blob` | ❌（自实现） | ✅ | ✅ | ✅ 已注册 | P1 |
| `File` | ❌（自实现） | ✅ | ✅ | ✅ 已注册 | P2 |
| `ReadableStream` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `crypto.subtle` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `atob` / `btoa` | ❌ | ✅ | ✅ | ✅ | P1 |
| `AbortController` / `AbortSignal` | ❌ | ✅ | ✅ | ✅ | P3 |
| `Event` / `EventTarget` | ❌ | ✅ | ✅ | ❌ P3 | — |
| `WebSocket` | ❌ | ✅ | ✅ | ❌ P3 | — |

**实现状态**：✅ atob/btoa/performance/AbortController 已完成，Event/EventTarget/ReadableStream 待实现

---

## 6. Node 兼容层（`node:` 前缀）

> 参考 Deno/Bun，完整 Node.js 内置模块 API 面。
> 与 W3C 标准库独立共存：`import "fs"` → W3C, `import "node:fs"` → Node

### 实施分阶段

| 阶段 | 模块 | 策略 | 状态 |
|------|------|------|------|
| 5.0 | **基础设施** — CJS require + Buffer 全局 + node:process + node:buffer | Rust + Boa global property | ✅ |
| 5.1 | `node:path` + `node:os` | Rust synthetic module | ✅ |
| 5.2 | `node:events` (EventEmitter) | 纯 JS 实现 | ✅ |
| 5.3 | `node:fs` (callback + sync + promises + constants) | Rust synthetic module | ✅ |
| 5.4 | `node:util` + `node:stream` + `node:url` | 纯 JS / Rust hybrid | ✅ |
| 5.5 | `node:crypto` + `node:child_process` + `node:module` | Rust 原生 | ✅ |
| 5.6 | `node:assert` + `node:tty` + `node:vm` + `node:zlib` + `node:querystring` + `node:perf_hooks` + `node:timers` | 纯 JS / Rust hybrid | ✅ |

### 全局对象注册

| 全局 | 来源 | 状态 |
|------|------|------|
| `Buffer` | Rust 原生 class Boa global class | ✅ 5.0 |
| `global` | globalThis 别名 | ✅ 5.0 |
| `setImmediate` / `clearImmediate` | Boa global property | ✅ 5.0 |
| `process` | Boa global property + node:process module | 🔜 后续（目前通过 `node:process` 导入） |
| `__dirname` / `__filename` | CJS 模块作用域注入 | 🔜 后续 |

### CJS 支持

| 功能 | 策略 | 状态 |
|------|------|------|
| `require()` | ModuleLoader 中检测 CJS → 函数作用域包装 | ✅ 5.0 |
| `module` / `exports` | CJS 模块运行时注入 | ✅ 5.0 |
| `.cjs` 文件支持 | ModuleLoader 按 CJS 模式加载 | ✅ 5.0 |

### Phase 5.1 API 清单

#### `node:path`

| API | 说明 | 实现方式 |
|-----|------|---------|
| `basename(p, ext?)` | 同 W3C path | JS 源码注入 |
| `delimiter` | `:` / `;` | 平台常量 |
| `dirname(p)` | 同 W3C path | JS 源码注入 |
| `extname(p)` | 同 W3C path | JS 源码注入 |
| `format(obj)` | 同 W3C path | JS 源码注入 |
| `isAbsolute(p)` | 同 W3C path | JS 源码注入 |
| `join(...parts)` | 同 W3C path | JS 源码注入 |
| `normalize(p)` | 同 W3C path | JS 源码注入 |
| `parse(path)` | 同 W3C path | JS 源码注入 |
| `relative(from, to)` | 同 W3C path | JS 源码注入 |
| `resolve(...parts)` | 同 W3C path | JS 源码注入 |
| `sep` | `/` / `\` | 平台常量 |
| `toNamespacedPath(p)` | 非 Win 直接返回 | JS 源码注入 |
| `posix` | POSIX 命名空间（sep=`/`, delimiter=`:`） | JS 对象 |
| `win32` | Win32 命名空间（sep=`\`, delimiter=`;`） | JS 对象 |
| `default` | 整个 path 对象 | 命名空间对象 |

#### `node:os`

| API | 说明 | 级别 |
|-----|------|------|
| `EOL` | `\n` / `\r\n` | P0 |
| `arch()` | CPU 架构 | P0 |
| `platform()` | OS 平台 | P0 |
| `type()` | OS 类型（Darwin/Linux/Windows_NT） | P1 |
| `release()` | 内核版本号 | P1 |
| `hostname()` | 主机名 | P1 |
| `homedir()` | 用户 home 目录 | P1 |
| `tmpdir()` | 临时目录 | P1 |
| `endianness()` | `'LE'` / `'BE'` | P0 |
| `uptime()` | 系统开机时间 | **新增** |
| `loadavg()` | 1/5/15 分钟负载 | **新增** |
| `cpus()` | CPU 信息数组[{model,speed,times}] | **新增** |
| `userInfo(opts?)` | 用户信息（username/uid/gid/shell/homedir） | **新增** |
| `version()` | 内核版本字符串 | **新增** |
| `machine()` | CPU 架构字符串（`'arm64'`、`'x86_64'`） | **新增** |
| `totalmem()` | 总内存（字节） | P2 |
| `freemem()` | 空闲内存（字节） | P2 |
| `networkInterfaces()` | 网络接口信息 | P3 暂缓 |
| `default` | 整个 os 对象 | — |

**实现状态**：✅ Phase 5.1 已完成

---

### Phase 5.2 API 清单

#### `node:events` (EventEmitter)

| API | 说明 | 级别 |
|-----|------|------|
| `EventEmitter` (default export) | 事件发射器类 | P0 |
| `emitter.on(event, listener)` | 添加监听器 | P0 |
| `emitter.addListener(event, listener)` | `on` 别名 | P0 |
| `emitter.off(event, listener)` | 移除监听器 | P0 |
| `emitter.removeListener(event, listener)` | `off` 别名 | P0 |
| `emitter.once(event, listener)` | 单次监听器 | P0 |
| `emitter.emit(event, ...args)` | 触发事件（返回 boolean） | P0 |
| `emitter.listeners(event)` | 返回监听器副本 | P0 |
| `emitter.rawListeners(event)` | 返回含 wrapper 的监听器 | P1 |
| `emitter.listenerCount(event)` | 监听器数量 | P0 |
| `emitter.eventNames()` | 返回已注册事件名数组 | P0 |
| `emitter.removeAllListeners(event?)` | 移除全部/指定事件监听器 | P0 |
| `emitter.prependListener(event, listener)` | 在队列头部添加 | P0 |
| `emitter.prependOnceListener(event, listener)` | 头部添加单次监听器 | P1 |
| `emitter.getMaxListeners()` | 获取最大监听器数 | P0 |
| `emitter.setMaxListeners(n)` | 设置最大监听器数 | P0 |
| `EventEmitter.defaultMaxListeners` | 类级默认最大值（静态） | P0 |
| `EventEmitter.listenerCount(emitter, event)` | 静态方法 | P0 |
| `EventEmitter.once(emitter, event)` | 返回 Promise | P1 |
| `newListener` / `removeListener` 事件 | 添加/移除时自动触发 | P0 |
| 超过 maxListeners 警告 | console.warn 提示 | P0 |

**实现状态**：✅ Phase 5.2 已完成

---

### Phase 5.3 API 清单

#### `node:fs`

| API | 说明 | 级别 |
|-----|------|------|
| `readFile(path, cb?)` | 异步读文件 | P0 |
| `readFileSync(path)` | 同步读文件 | P0 |
| `writeFile(path, data, cb?)` | 异步写文件 | P0 |
| `writeFileSync(path, data)` | 同步写文件 | P0 |
| `appendFile(path, data, cb?)` | 异步追加 | P0 |
| `appendFileSync(path, data)` | 同步追加 | P0 |
| `mkdir(path, opts?, cb?)` | 异步创建目录 | P0 |
| `mkdirSync(path, opts?)` | 同步创建目录 | P0 |
| `readdir(path, cb?)` | 异步读目录 | P0 |
| `readdirSync(path)` | 同步读目录 | P0 |
| `rmdir(path, cb?)` | 异步删目录 | P0 |
| `rmdirSync(path)` | 同步删目录 | P0 |
| `rm(path, opts?, cb?)` | 异步删除（递归） | P1 |
| `rmSync(path, opts?)` | 同步删除（递归） | P1 |
| `unlink(path, cb?)` | 异步删文件 | P0 |
| `unlinkSync(path)` | 同步删文件 | P0 |
| `stat(path, cb?)` | 异步 stat | P0 |
| `statSync(path)` | 同步 stat | P0 |
| `lstat(path, cb?)` | 异步 lstat | P0 |
| `lstatSync(path)` | 同步 lstat | P0 |
| `access(path, mode?, cb?)` | 异步访问检查 | P0 |
| `accessSync(path, mode?)` | 同步访问检查 | P0 |
| `chmod(path, mode, cb?)` | 异步改权限 | P1 |
| `chmodSync(path, mode)` | 同步改权限 | P1 |
| `rename(old, new, cb?)` | 异步重命名 | P0 |
| `renameSync(old, new)` | 同步重命名 | P0 |
| `copyFile(src, dst, flags?, cb?)` | 异步复制 | P0 |
| `copyFileSync(src, dst, flags?)` | 同步复制 | P0 |
| `existsSync(path)` | 同步存在检查 | P0 |
| `realpath(path, opts?, cb?)` | 异步真实路径 | P1 |
| `realpathSync(path, opts?)` | 同步真实路径 | P1 |
| `symlink(target, path, cb?)` | 异步软链接 | P1 |
| `symlinkSync(target, path)` | 同步软链接 | P1 |
| `link(existing, new, cb?)` | 异步硬链接 | P1 |
| `linkSync(existing, new)` | 同步硬链接 | P1 |
| `truncate(path, len, cb?)` | 异步截断 | P1 |
| `truncateSync(path, len)` | 同步截断 | P1 |
| `constants` | 文件系统常量 | P0 |
| `promises` | Promise 版本 API | P0 |
| `default` | 整个 fs 命名空间对象 | P0 |

**实现状态**：✅ Phase 5.3 已完成

---

### Phase 5.4 API 清单

#### `node:util`

| API | 说明 | 级别 |
|-----|------|------|
| `promisify(original)` | callback 风格 → Promise | P0 |
| `callbackify(original)` | Promise → callback 风格 | P1 |
| `format(fmt, ...args)` | printf 风格字符串格式化 | P0 |
| `inspect(obj, opts?)` | 对象序列化（深度/颜色/循环引用） | P0 |
| `deprecate(fn, msg)` | 废弃函数包装警告 | P1 |
| `types.isDate(v)` | 类型判断 | P1 |
| `types.isRegExp(v)` | 类型判断 | P1 |
| `types.isArrayBuffer(v)` | 类型判断 | P1 |
| `types.isMap(v)` | 类型判断 | P1 |
| `types.isSet(v)` | 类型判断 | P1 |
| `types.isPromise(v)` | 类型判断 | P1 |
| `types.isNativeError(v)` | 类型判断 | P1 |
| `types.isTypedArray(v)` | 类型判断 | P1 |
| `inherits(ctor, superCtor)` | 原型链继承 | P1 |
| `debuglog(section)` | 调试日志 | P2 |
| `default` | 整个 util 命名空间对象 | P0 |

#### `node:stream`

| API | 说明 | 级别 |
|-----|------|------|
| `Stream` (default) | 流基类（继承 EventEmitter） | P0 |
| `Readable` | 可读流 | P0 |
| `Writable` | 可写流 | P0 |
| `Transform` | 转换流 | P1 |
| `Duplex` | 双工流 | P1 |
| `PassThrough` | 直通流 | P2 |
| `pipeline(...streams, cb?)` | 管道连接 | P0 |
| `finished(stream, cb)` | 流结束监听 | P1 |

#### `node:url`

| API | 说明 | 级别 |
|-----|------|------|
| `URL` | 同全局 URL 类 | P0 |
| `URLSearchParams` | 同全局 URLSearchParams | P0 |
| `fileURLToPath(url)` | `file://` URL → 文件路径 | P0 |
| `pathToFileURL(path)` | 文件路径 → `file://` URL | P0 |
| `url.format(urlObj)` | URL 对象格式化 | P2 |
| `url.parse(urlStr)` | URL 字符串解析 | P2 |
| `default` | 整个 url 命名空间对象 | P0 |

**实现方式**：纯 JS + Rust 混合（`node:util` 含 Rust `inspect` 增强，`node:stream` 纯 JS，`node:url` 纯 JS 包装全局类）

**实现状态**：✅ Phase 5.4 已完成

---

### Phase 5.5 API 清单

#### `node:crypto`

| API | 说明 | 级别 |
|-----|------|------|
| `createHash(algo)` | 创建哈希对象 | P0 |
| `hash.update(data)` | 更新哈希数据 | P0 |
| `hash.digest(encoding)` | 计算摘要 (hex/base64) | P0 |
| `randomBytes(size, cb?)` | 生成随机字节 | P0 |
| `randomUUID()` | 生成随机 UUID v4 | P0 |
| `createHmac(algo, key)` | 创建 HMAC 对象 | P1 |
| `default` | 整个 crypto 命名空间对象 | P0 |

**支持算法**: sha256, sha384, sha512, sha1, md5

#### `node:child_process`

| API | 说明 | 级别 |
|-----|------|------|
| `execSync(command, opts?)` | 同步执行 shell 命令 | P0 |
| `exec(command, opts?, cb?)` | 异步执行 shell 命令 | P0 |
| `spawnSync(command, args?, opts?)` | 同步 spawn 进程 | P0 |
| `spawn(command, args?, opts?)` | 异步 spawn 进程（阻塞等待） | P0 |
| `execFile(file, args?, opts?, cb?)` | 直接执行文件（无 shell） | P1 |
| `default` | 整个 child_process 命名空间对象 | P0 |

#### `node:module`

| API | 说明 | 级别 |
|-----|------|------|
| `builtinModules` | 内置模块名称列表 | P0 |
| `isBuiltin(name)` | 判断是否为内置模块 | P0 |
| `createRequire(filename)` | 创建 require 函数 | P0 |
| `Module._resolveFilename(req, parent)` | 模块路径解析 | P1 |
| `default` | 整个 module 命名空间对象 | P0 |

**实现方式**：Rust synthetic module + 少量辅助函数

**实现状态**：✅ Phase 5.5 已完成
