# Changelog

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
