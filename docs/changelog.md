# Changelog

## v0.1.0-dev.6 — Event/EventTarget 全局类 + __dirname/__filename 文件系统感知

- `globalThis.Event` / `globalThis.EventTarget` — Rust `#[boa_class]` 原生实现：构造/type/target/defaultPrevented/cancelable/bubbles/getters、preventDefault/stopPropagation、addEventListener/removeEventListener/dispatchEvent（12 集成测试）
- `__dirname` / `__filename` — 从 `import.meta` 改为文件系统路径注入，每个 CJS 模块根据实际文件路径生成准确的目录名和文件名
- `module_loader` — `Source::from_bytes` 增加 `.with_path()`，使解析器获知文件位置
- 修复 `event.rs` 6 个编译错误：`to_boolean()` 非 Result 用法、`#[boa(rename = "type")]` 缺失、`GcRef` 字段访问改用 `downcast_ref`、dispatch 后 listeners 未恢复
- **当前测试数：57 全过（38 单元 + 19 集成），零 clippy 警告**

## v0.1.0-dev.5 — W3C 全局 API 补齐 (atob/btoa/performance/AbortController)

- `globalThis.atob` / `globalThis.btoa` — Base64 编解码（Rust `base64` crate，2 测试）
- `globalThis.performance` — Rust `#[boa_class]` 原生实现：now/timeOrigin/mark/measure/clearMarks/clearMeasures/getEntries（6 测试）
- `globalThis.PerformanceEntry` / `PerformanceMark` / `PerformanceMeasure`（Rust 原生类）
- `globalThis.AbortController` / `AbortSignal` — Rust `#[boa_class]`，addEventListener/removeEventListener/abort 事件回调（2 测试）
- `node:perf_hooks` 简化：从 `globalThis.performance` 引用，不再自建 JS shim
- Cargo.toml 新增依赖：`base64`
- **当前测试数：284 单元+集成（38 单元 + 246 集成），零 clippy 警告**

## v0.1.0-dev.4 — Node 兼容层 Phase 5.6 完结 + CLI 入口

- `node:querystring` — parse/stringify/escape/unescape（纯 JS，10 测试）
- `node:assert` — ok/equal/strictEqual/deepEqual/throws/AssertionError + strict 命名空间（纯 JS，17 测试）
- `node:timers` — setTimeout/setInterval/setImmediate + timers/promises（纯 JS，7 测试）
- `node:tty` — isatty (libc) + WriteStream/ReadStream（Rust + JS，5 测试）
- `node:perf_hooks` — performance.now/timeOrigin + PerformanceEntry/Mark/Measure（Rust `Instant` + JS，6 测试）
- `node:vm` — runInThisContext/runInNewContext/Script/compileFunction（纯 JS，6 测试）
- `node:zlib` — gzipSync/gunzipSync/deflateSync/inflateSync + deflateRawRaw + unzipSync（Rust `flate2` + JS，6 测试）
- CLI 二进制 `oolong`：`oolong run <file>` / `oolong eval <code>`（clap derive）
- `process.argv` 通过 `set_cli_args()` 自定义参数，`--` 分隔脚本参数
- `process` 全局对象：不再需要 `import "process"`，可直接用 `process.argv`/`process.pid`/`process.env` 等
- `import "os"` 新增 4 API：`cpus()` / `uptime()` / `loadavg()` / `endianness()`
- Cargo.toml 新增依赖：`libc`、`flate2`、`clap`
- **当前测试数：273 单元+集成（38 单元 + 235 集成），零 clippy 警告（dev.4）**

## v0.1.0-dev.1 — 项目诞生

- 从 kossjs fork 独立，创建 oolong 项目
- 确定架构：引擎 + W3C 标准库 + Node 兼容层
- CJS→ESM 静态转译器（13 测试）
- 从 kossjs 迁移：transpiler（11 测试）、resolver（10 测试）、typecheck
- ModuleLoader 集成：Boa ModuleLoader trait + 管线 TS→JS→CJS→ESM→Boa
- Runtime 封装：Context + ModuleLoader + Console（7 集成测试）
- vendor oxc_transformer：修复 Rust nightly `if let` guard 兼容问题
- 项目文档体系：architecture.md / agents.md / changelog.md / taolun.md
- **当前测试数：41 单元+集成，零 clippy 警告**

## v0.1.0-dev.2 — 标准库补齐

- `import "path"` 实现（24 测试）：join/dirname/basename/extname/isAbsolute/normalize/relative/resolve/parse/format/sep/delimiter
- `import "process"` 实现（16 测试）：cwd/chdir/pid/ppid/platform/arch/version/versions/execPath/env/argv/exit/stdout/stderr/stdin/uptime/memoryUsage/title/execArgv
- `import "fs"` 实现（15 测试 + 32 API）：readFile/readTextFile/readFileSync/writeFile/writeTextFile/exists/mkdir/remove/readdir/stat/lstat/appendFile/copyFile/rename/realpath/symlink + 11 同步版 + chmod/chown/link/truncate
- `import "os"` 实现（15 测试）：platform/arch/EOL/hostname/type/release/homedir/tmpdir/totalmem/freemem
- `readFileSync` 返回 `ArrayBuffer`（与 Deno 对齐）
- `writeFile` 支持 `string | ArrayBuffer | Uint8Array`
- `stdin.read()` / `stdin.readAsBytes()` 异步标准输入
- 修复 clippy 警告（collapsible_if、needless_borrows_for_generic_args）
- **当前测试数：84 单元+集成，零 clippy 警告**

## v0.1.0-dev.3 — Web API （fetch + Blob + URL）

- `Blob` / `File` 全局类：构造/text/arrayBuffer/slice/size/type（WHATWG 规范）
- `URLSearchParams` 全局类：get/set/append/delete/has/sort/toString/forEach/entries/keys/values
- `URL` 全局类：从 boa_runtime 注册（支持构造函数/属性/相对 URL 解析）
- `TextEncoder` / `TextDecoder`：从 boa_runtime 注册（utf-8/utf-16 编解码）
- `queueMicrotask` / `structuredClone`：从 boa_runtime 注册
- `fetch` / `Request` / `Response` / `Headers`：从 boa_runtime 注册 + `BlockingReqwestFetcher` HTTP 后端
- Cargo.toml 加 `reqwest-blocking` feature（依赖 reqwest + rustls）
- **当前测试数：98 单元+集成，零 clippy 警告**
