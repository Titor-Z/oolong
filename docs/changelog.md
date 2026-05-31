# Changelog

## 2026-05-31 — 路径统一 + http 拆分 + net 增强

- **路径统一**：koss `~/.koss/` → `~/.cha/`，`kos.json` → `cha.json`，`koss-index.json` → `package_meta.json`；oolong `~/.oolong/` → `~/.cha/`
- **拆 `node:http`**：1866 行 → 6 子模块 (mod/common/incoming/outgoing/server/client)，每文件 ≤364 行
- **Express 兼容**：JSON body echo 端到端测试，验证 `req.on('data/end')` 模式
- **`net.isIP/isIPv4/isIPv6`**：纯字符串判断函数
- **`net.Socket`**：on/once/connect/write/end/destroy/setTimeout EventEmitter
- **`req.socket`**：EventEmitter + setTimeout stub
- **486 测试全过**

## 已实现

- CJS→ESM 静态转译器（13 测试）
- TS→JS 转译器 / TypeScript 类型检查（11 测试）
- 模块解析/加载管线（10 测试）
- CJS require 运行时
- 三元标准库体系（web/ + std/ + node/）— 全部 Rust 原生
- **web/** — 全局类 13 个：Event/EventTarget, AbortController/AbortSignal, Blob/File, URL/URLSearchParams, atob/btoa, Performance, TextEncoder/TextDecoder, queueMicrotask/structuredClone, Headers/Response/Request/fetch
- **std/** — 4 模块：path, process, fs, os（全部 Rust 原生 ✅）
- **std/http** — Phase A: HTTP Server 完整实现 ✅
- **node/** — 19 模块覆盖 Phase 5.0~5.6：process, buffer, path, os, events, fs, util, stream, url, crypto, child_process, module, assert, tty, vm, zlib, querystring, perf_hooks, timers
- **node:http + node:net** — Phase B.2 完善 ✅：httparse 集成、IncomingMessage/OutgoingMessage、端到端 server 测试
- CLI 二进制：oolong run / oolong eval
- process 全局对象 / Buffer 全局类
- __dirname / __filename（CJS 文件级感知）
- **std/encoding** — Phase C.1: base64 + hex 编码解码（17 测试）
- **web/streams** — Phase C.2: QueuingStrategy（10 测试）
- **web/streams** — Phase C.3: ReadableStream + DefaultReader + DefaultController（12 测试）
- **web/streams** — Phase C.4: WritableStream + DefaultWriter + DefaultController（9 测试）
- **web/streams** — Phase C.5: TransformStream + DefaultController（7 测试）
- **web/streams** — Phase C.6: pipeTo / pipeThrough / tee（6 测试）
- **W3C console** — Phase C 扩展：全局 `console.log/debug/info/warn/error/trace/assert/time/timeEnd/timeLog/count/countReset/table/group/groupEnd/groupCollapsed`（9 测试）
- **@std/log** — Phase C.10：Logger 全局类 + getLogger + setup + child + 莫兰迪配色 + JSON 模式（20 测试）
- **@std/uuid** — Phase C.7：UUID v4 生成 + validate 校验（8 测试）
- **@std/semver** — Phase C.8：纯 Rust semver 解析/比较/satisfies（25 测试）
- **@std/fmt** — Phase C.9：ANSI colors + sprintf（16 测试）
- **Phase C 全部完成** — C.1–C.10 所有模块已实现 ✅
- **485 测试全过，零 clippy 错误**

## 待实现

### Phase A — `std/http` HTTP Server

- `import { serve } from "std/http"` — 基于 tokio TCP listener
- 简写模式 `serve({ port: 3000 }, handler)` + `for await...of` 迭代模式
- Request/Response 复用 web/ 全局类
- ~20 集成测试

### Phase B — `node:*` JS→Rust 迁移

10 个内联 JS 模块重写为 Rust 纯代码：

| 优先级 | 模块 | 状态 | 说明 |
|--------|------|:----:|------|
| B.1 | `node:path` | ✅ | 高频，打包工具热路径 |
| B.2 | `node:events` | 🔜 | EventEmitter 生态基石 |
| B.3 | `node:stream` | 🔜 | 与 fs/http 绑定 |
| B.4 | `node:util` | 🔜 | format/inspect 热路径 |
| B.5 | `node:module` | 🔜 | createRequire |
| B.6~B.10 | url, assert, querystring, timers, vm | 🔜 | 逐个迁移 |

### Phase C — `std/` 原生层扩充（已完成）

- ✅ `std/encoding` — base64, hex（Rust 原生，17 测试）
- ✅ `web/streams` — W3C Web Streams（44 测试）
- ✅ `web/console` — W3C console 全局对象（9 测试）
- ✅ `std/log` — 结构化日志框架（20 测试）
- ✅ `std/uuid` — UUID v4 生成 + validate（8 测试）
- ✅ `std/semver` — 纯 Rust semver 解析/比较/satisfies（25 测试）
- ✅ `std/fmt` — ANSI colors + sprintf（16 测试）

### Phase D — `std/fs` 增强

- `ensureDir`, `walk`, `expandGlob`, `copy`, `move`, `emptyDir`

### Phase E — Compile OOLONG → Node.js（远期）

- 利用 nodeCompat 适配层 + ModuleLoader import 重写 + polyfill 注入
- 将 `std/` 代码编译输出为 Node.js 可运行代码

## 未来架构

```
三元标准库 — 全部全栈 Rust + W3C 类型约定

web/   — W3C 全局类（EventTarget, Blob, fetch…）
std/   — OOLONG 原生模块（不受 nodeCompat 影响，始终 W3C）
node/  — Node.js 兼容模块（nodeCompat 控制版本输出）

nodeCompat 配置（oolong.json）：
  "v22" → 默认，match Node.js 22
  "v20" → match Node.js 20
  "v18" → match Node.js 18
  "w3c" → 直接输出 W3C 标准类型
```

---

## 版本历史

### v0.1.0-dev.11 — Phase B.2: node:http + node:net 完善（2026-05-30）

**实现了什么：**
- **`node:http` 完整重写** — httparse 集成、IncomingMessage（`req.on("data")`/`req.on("end")`）、OutgoingMessage（`writeHead`/`write`/`end`/`setHeader`/`getHeader`/`removeHeader`）
- **`request()` / `get()` HTTP 客户端** — 基于 reqwest blocking，支持 URL 和 options 对象两种入参
- **`Server` 类方法** — `listen()`/`close()`/`address()`/`on("request"|"listening"|"close")` EventEmitter
- **`STATUS_CODES`** — 完整的 HTTP 状态码映射
- **非阻塞 accept** — 连接处理完成后自动 shutdown write 侧
- **15 集成测试** — 包含 10 个端到端 server 测试（GET/POST/headers/body/status/write+end/req.on data 流式）
- **`module_loader` 注册** — `node:http`/`node:net` 加入 BUILTIN_MODULES 和 BARE_NODE_MODULES

**待实现：**
- `req.socket` 完整 `net.Socket` 支持（当前为简化 socket 对象）
- 非阻塞 accept（tokio/JobQueue 集成，当前仍同步阻塞）
- 分块传输编码（chunked transfer encoding）支持
- `http.ClientRequest` 事件（error, response, connect）
- `http.Server` 的 `keepAlive` 支持
- express 兼容性测试

**测试数：364 全过（315 集成 + 49 单元），零 clippy 错误**

---

**实现了什么：**
- **node:events 全栈 Rust 原生** — 替换 inline JS 实现（170 行 JS → 490 行 Rust）
- **EventEmitter 类**：完整支持 on/once/emit/removeListener/off/removeAllListeners/prependListener/prependOnceListener/listenerCount/eventNames/listeners/getMaxListeners/setMaxListeners
- **静态 API**：`defaultMaxListeners`、`EventEmitter.listenerCount()`、`EventEmitter.once()`
- **`types/node/events.d.ts`** — EventEmitter 完整类型定义
- 13 个集成测试全部通过，行为与原 JS 实现一致

**迁移收益：**
- 不再依赖 inline JS 字符串编译，模块初始化更快
- 类型安全，Rust 编译器捕获错误
- 为 stream/util/module 的 Rust 迁移铺路（它们都依赖 events）

**待办：**
- `symbol` 类型事件名支持（当前只支持 string）
- `rawListeners()` 方法
- `newListener`/`removeListener` 事件触发（部分支持）

---

### v0.1.0-dev.9 — Phase A 完成 + W3C Response/Request 自实现（2026-05-30）

**实现了什么：**
- **W3C Headers/Response/Request 自实现** — 替换 boa_runtime 版本，修复构造器 body 被丢弃的缺陷
- **W3C fetch 自实现** — 基于 reqwest blocking，使用自实现 Response/Request
- **std/http handler 接收 web Request 对象** — 替代旧的 ServeRequest plain object
- **`types/web/response.d.ts`** — Headers/Response/Request/fetch 完整类型定义
- **类型修复** — `types/std/http.d.ts` 移除未定义的 `ServeRequest`，handler 类型更正为 `(req: Request) => Response | Promise<Response>`
- **19 集成测试** — 含 `new Response()` 功能验证

**未实现：**
- `for await...of` 高级模式（Phase A.5）
- Response.body 属性返回 ReadableStream（目前不返回）

**测试拆分：** 原来全部 283 个集成测试在单文件 `tests/runtime_test.rs`，现将：
   - 按模块拆分到 24 个独立 `tests/*.rs` 文件（`tests/common.rs` 共享 `create_runtime()`）
   - 各文件独立编译，`cargo test` 并行运行
   - 文件名与模块对应：`std_*.rs` / `node_*.rs` / `web_globals.rs` / `cjs.rs` / `eval.rs`
- **新增 Rust 单元测试：** `src/web/headers.rs#\[cfg(test)\]` — 10 个测试覆盖 normalize / is_forbidden / from_map + iter

**测试数：331 全过（282 集成 + 49 单元），零 clippy 警告**

---

### v0.1.0-dev.8 — Phase A (std/http) + Phase B.1 (node:path)（2026-05-30）

**实现了什么：**
- `@std/http` — `serve({ port, handler })` 阻塞式 HTTP 服务器
- `node:path` JS→Rust 迁移（180 行 JS → 282 行 Rust）
- `types/` 类型定义体系创建（`.d.ts` 先写再实现的工作流）
- 类型一致性校验测试（4 个，验证 Rust 导出与 `types/` 声明匹配）
- 裸名路由逻辑（`module_loader.rs` 中 `route_bare_specifier`）
- `@std/http` 模块注册 + BUILTIN_MODULES
- `oolong.jsonc` 单文件配置方案定稿

**未实现：**
- node:http 尚未实现（node:* 仅 `node:path` 已迁移）
- 剩余 9 个 node:* 模块仍为内联 JS
- std/ 类型文件尚缺 node/ 模块（nodes 9 个待 Phase B 补充）
- `nodeCompat` 运行时配置尚在 ModuleLoader 中埋点，未实现 oolong.jsonc 解析

**当前测试数：304 全过（38 单元 + 266 集成），零 clippy 警告**

---

### v0.1.0-dev.7 — 架构重构 v2（2026-05-30，文档阶段）

**实现了什么：**
- 全栈 Rust + W3C 类型约定 + nodeCompat 方案定稿
- 裸名路由决定：有 nodeCompat → `node:*`，无 nodeCompat → `@std/*`
- 三元标准库最终架构定型
- 10 个 `node:*` 内联 JS 模块列入 Phase B 迁移计划
- `docs/plan-v2.md` 新建，Phase A~E 完整规划
- 所有现有文档更新反映新架构

**未实现：**
- 尚未开始任何代码修改
- std/http 待开发
- node:* 模块仍为内联 JS，待 Phase B 迁移

---

### v0.1.0-dev.6 — Event/EventTarget + `__dirname`/`__filename`（2026-05-29）

**实现了什么：**
- `globalThis.Event` / `EventTarget` — Rust `#[boa_class]` 原生实现（12 集成测试）
- `__dirname` / `__filename` — CJS 模块文件级感知
- `module_loader` — Source.from_bytes 增加 .with_path()
- 修复 event.rs 6 个编译错误

**未实现：**
- node:events（EventEmitter）仍为内联 JS
- `__dirname`/`__filename` 仅支持 CJS 模块，ESM 模块未覆盖

**当前测试数：57 全过（38 单元 + 19 集成）**

---

### v0.1.0-dev.5 — W3C 全局 API（atob/btoa/performance/AbortController）（2026-05-29）

**实现了什么：**
- `atob`/`btoa` — Base64 编解码（Rust base64 crate）
- `performance` — Rust `#[boa_class]`：now/timeOrigin/mark/measure/getEntries/clear
- `PerformanceEntry`/`PerformanceMark`/`PerformanceMeasure` — Rust 原生类
- `AbortController`/`AbortSignal` — Rust `#[boa_class]`
- `node:perf_hooks` 简化：从 globalThis.performance 直接引用

**未实现：**
- `performance` 全部指标（memory, navigation）
- `AbortSignal.timeout()` 静态方法
- `Event`/`EventTarget` 尚待补齐
- fetch 超时未与 AbortSignal 集成

**新增依赖：** `base64`

**当前测试数：284（38 单元 + 246 集成）**

---

### v0.1.0-dev.4 — Node 兼容层 Phase 5.6 完结 + CLI 入口（2026-05-29）

**实现了什么：**
- 7 个新节点模块：querystring, assert, timers, tty, perf_hooks, vm, zlib
- CLI 二进制：oolong run/eval（clap derive）
- process.argv 自定义 + `--` 分隔
- process 全局对象（无需 import）
- `import "os"` 新增 4 API：cpus/uptime/loadavg/endianness

**未实现：**
- Phase 5.6 中 assert/timers/querystring/vm 为纯 JS，非 Rust 原生
- CLI 暂不支持 `--watch`、`--inspect`
- process.stdout/stderr 写操作未非阻塞

**新增依赖：** libc, flate2, clap

**当前测试数：273（38 单元 + 235 集成）**

---

### v0.1.0-dev.3 — Web API（fetch + Blob + URL）（2026-05-29）

**实现了什么：**
- Blob/File 全局类（WHATWG 规范：构造/text/arrayBuffer/slice）
- URLSearchParams 全局类（get/set/append/delete/sort/entries...）
- URL 全局类（从 boa_runtime 注册）
- TextEncoder/TextDecoder（从 boa_runtime 注册）
- queueMicrotask/structuredClone（从 boa_runtime 注册）
- fetch/Request/Response/Headers（BlockingReqwestFetcher）

**未实现：**
- Blob 不支持 stream()
- FileReader 跳过（无浏览器 DOM 场景）
- fetch 不支持 streaming response body
- Request/Response 不支持 clone()

**当前测试数：98（单元+集成）**

---

### v0.1.0-dev.2 — 标准库补齐（2026-05-29）

**实现了什么：**
- `import "path"` — 12 API（24 测试）
- `import "process"` — 22 API（16 测试）
- `import "fs"` — 32 API（15 测试）
- `import "os"` — 10 API（15 测试）
- readFileSync 返回 ArrayBuffer（与 Deno 对齐）
- writeFile 支持 string/ArrayBuffer/Uint8Array
- stdin.read()/readAsBytes() 异步标准输入

**未实现：**
- fs 缺少 access/watch
- fs 缺少 ensureDir/walk 等高级操作
- os 缺少 networkInterfaces/userInfo/devNull
- process 缺少 kill/abort/umask/getuid/getgid

**当前测试数：84（单元+集成）**

---

### v0.1.0-dev.1 — 项目诞生（2026-05-28）

**实现了什么：**
- 从 kossjs fork 独立，创建 oolong 项目
- 架构初定：引擎 + W3C 标准库 + Node 兼容层
- CJS→ESM 静态转译器（13 测试）
- 迁移 transpiler/resolver/typecheck（21 测试）
- ModuleLoader 集成：Boa ModuleLoader trait + 管线
- Runtime 封装：Context + ModuleLoader + Console（7 集成测试）
- vendor oxc_transformer（if let guard 补丁）
- 文档体系：architecture.md/agents.md/changelog.md/taolun.md

**未实现：**
- 标准库仅有基础架构，无 API 实现
- Node 兼容层未开始
- CLI 未实现
- Web API 仅有 boa_runtime 默认项

**当前测试数：41（单元+集成）**
