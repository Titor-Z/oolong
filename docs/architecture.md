# OOLONG 乌龙茶 🍵 — 架构设计

## 项目定义

OOLONG 是自研的 JavaScript/TypeScript 运行时引擎。

- **ES6 ~ ES2026 ~ ESNext** 标准语法为第一等公民
- **TypeScript** 运行时，基于 OXC 转译 TS → ESNext
- **tsgo**（Microsoft 官方原生 TypeScript 7.0 检查器）进行类型检查
- **全栈 Rust**：引擎、标准库、Node 兼容层全部 Rust 原生实现
- **W3C 类型约定**：所有模块遵守 W3C 类型规范（Uint8Array, DOMHighResTimeStamp, AbortSignal…）
- **三元标准库体系**：
  - `src/web/` — W3C Web API（Blob、URLSearchParams、fetch 等全局类）
  - `src/std/` — OOLONG 原生模块（`import "fs"`、`import "os"` 等）
  - `src/node/` — Node.js 兼容层（`import "node:fs"`）

## 与 CHA 的关系

```
CHA 茶 🍵（统一 CLI + 包管理器）
  │
  └─ dep: oolong（引擎层）
```

- **CHA** — 负责 CLI 交互和包管理
- **OOLONG** — 负责代码执行、模块加载、类型检查和标准库

## 模块加载管线

```
源文件
  │
  ├─ .ts/.tsx → OXC transpiler（TS→JS）
  ├─ .js（含 CJS）→ 语法检测 → CJS→ESM transform（如需要）
  │
  ▼
Boa Parser（ES Module）
  │
  ▼
Boa ModuleLoader（OOLONG 实现）
  ├─ 解析 import → resolver（路径/node_modules/cha 缓存）
  ├─ 递归加载依赖（同上管线）
  │
  ▼
Boa 执行
```

## 架构分层

```
用户代码
  │
  ├─ import "fs"          import "node:fs"        Blob/fetch/URL（全局）
  │   (OOLONG 原生)         (Node 兼容)            (W3C Web API)
  ▼
┌──────────────────────────────────────────────────────────────┐
│ 三元标准库 — 全部 Rust + W3C 类型约定                          │
│                                                              │
│  src/web/  W3C 全局类（自实现，替换 boa_runtime）              │
│  ├─ Event/EventTarget, AbortController/AbortSignal           │
│  ├─ Blob/File, URL/URLSearchParams                          │
│  ├─ TextEncoder/TextDecoder, queueMicrotask                 │
│  ├─ atob/btoa, Performance                                  │
│  ├─ Headers/Response/Request（自实现 ✅）                     │
│  └─ fetch（自实现，基于 reqwest blocking）                    │
│                                                              │
│  src/std/  OOLONG 原生模块（不受 nodeCompat 影响）              │
│  ├─ path: join/dirname/basename/extname/…                    │
│  ├─ process: cwd/pid/arch/env/argv/stdin/stdout/…             │
│  ├─ fs: readFile/writeFile/mkdir/readdir/stat/…              │
│  ├─ os: platform/arch/cpus/EOL/hostname/memory/…             │
│  └─ http: serve()（🏗️ Phase A）                               │
│                                                              │
│  src/node/ Node 兼容层（nodeCompat 控制输出类型）               │
│  ├─ path, os, events（✅ Phase B）, fs, process, buffer（Rust）│
│  ├─ crypto, child_process, zlib, tty, perf_hooks（Rust）      │
│  ├─ assert, querystring, timers, vm                          │
│  │   （🏗️ Phase B: JS→Rust 迁移）                             │
│  └─ stream, util, url, module                                 │
│      （🏗️ Phase B: JS→Rust 迁移）                             │
├──────────────────────────────────────────────────────────────┤
│  OOLONG — 引擎核心                                             │
│  ├─ runtime.rs（Boa Context 封装）                             │
│  ├─ module_loader.rs（Boa ModuleLoader）                       │
│  ├─ resolver.rs（模块路径解析）                                 │
│  ├─ cjs_to_esm.rs（CJS→ESM 转译）                             │
│  ├─ cjs/（CJS require 运行时）                                 │
│  ├─ transpiler.rs（OXC TS→JS）                                │
│  └─ typecheck.rs（tsgo 调用）                                  │
├──────────────────────────────────────────────────────────────┤
│  Boa 0.21 + OXC 0.133                                         │
│  └─ vendor/oxc_transformer（if let 补丁）                      │
└──────────────────────────────────────────────────────────────┘
```

## nodeCompat 配置机制

`oolong.json` 中的 `nodeCompat` 配置项同时控制两件事：

1. **裸名路由** — 决定 `import "path"` 指向 `node:path` 还是 `@std/path`
2. **`node:*` 输出类型** — 决定 `node:*` 暴露的版本行为

```jsonc
{
  "nodeCompat": "v22"  // npm 项目：裸名→node:*，match Node.js 22
  // "nodeCompat": "v20"  // npm 项目：裸名→node:*，match Node.js 20
  // "nodeCompat": "v18"  // npm 项目：裸名→node:*，match Node.js 18
                        // 无 nodeCompat → 纯 OOLONG 模式，裸名→@std/*
}
```

### 导入路由逻辑

| 用户写 | 有 nodeCompat | 无 nodeCompat |
|--------|:------------:|:-------------:|
| `import "path"` | → `node:path` | → `@std/path` |
| `import "@std/path"` | → `std::path` | → `std::path` |
| `import "node:path"` | → `node::path` | → `node::path` |

- `node:*` 内部 Rust 实现全部使用 W3C 类型约定
- 对外暴露时根据 `nodeCompat` 值进行适配转换
- 用户无需在代码中写任何版本判断
- `std/` 和 `web/` 不受 `nodeCompat` 影响，始终使用 W3C 标准
- 所有路由逻辑集中在 `module_loader.rs` 一个文件

## 标准库设计

### 哲学（2026-05-30 确立）

OOLONG 标准库是 **自定标准**，融合三家之长：

| 来源 | 影响 |
|------|------|
| **W3C Web 标准** | 浏览器已有 API：`Blob`、`fetch`、`URL`、`TextEncoder` |
| **Deno** | 模块式导入（`import "fs"`）、异步优先、`Sync` 后缀 |
| **Node.js** | `node:*` 前缀，完整 Node.js API 面（通过 nodeCompat 适配） |
| **Bun** | 全局对象注册模式、性能优先的 Rust 原生实现 |

**关键认知**：`import "os"` 不存在真正的 W3C 标准（浏览器没有 `os` 模块）。
OOLONG 原生层的 API 面是自定的，参考 Deno `Deno.*` + Bun `Bun.*` + Node.js
三方的最佳设计。

### W3C 类型约定（所有模块必须遵守）

| # | 规则 | 说明 |
|---|------|------|
| 1 | 二进制数据用 `Uint8Array` | Rust 层 `Vec<u8>` / `&[u8]`，暴露给 JS 为 `Uint8Array` |
| 2 | 时间戳用 `DOMHighResTimeStamp` | `f64` 毫秒，非秒；非微秒；非纳秒 |
| 3 | 异步返回 `Promise<T>` | 不暴露 callback 风格 |
| 4 | 取消用 `AbortSignal` | 函数签名接收 `Option<&AbortSignal>` |
| 5 | 字符串用 `USVString` | Rust `String`，语义不含非法代理对 |
| 6 | 错误用标准类型 | `TypeError`、`RangeError`，非自定义 error code |

### 使用风格

- **模块导入**（`import "fs"`），非全局对象（与 Deno/Bun 对齐）
- **异步优先**（`await fs.readFile(path)` 返回 `Promise`）
- **三元路由**（依赖 nodeCompat 配置）：
  - `import "fs"` → 有 nodeCompat → `node::fs` | 无 nodeCompat → `@std/fs`（`src/std/`）
  - `import "@std/fs"` → OOLONG 原生模块（`src/std/`）
  - `import "node:fs"` → Node.js 兼容层（`src/node/`）
  - `Blob` / `fetch` → W3C Web API 全局类（`src/web/`）
- 三种 import 语法全支持：default / named / namespace

### 目录结构

```
src/
├── web/            W3C Web API 全局类（Blob、URLSearchParams…）
│   ├── mod.rs
│   ├── blob.rs
│   └── url_search_params.rs
├── std/            OOLONG 原生模块（import "fs" 等）
│   ├── mod.rs
│   ├── fs.rs
│   ├── path.rs
│   ├── process.rs
│   └── os.rs
└── node/           Node.js 兼容层（import "node:fs"）
    ├── mod.rs
    ├── path.rs
    ├── os.rs
    ├── events.rs
    └── ...（19 模块）
```

### 实现策略

- **全部 Rust 原生**——没有 JS 字符串注入，没有语言边界
- **`node:` 和 `std/` 各自独立实现**——没有包装/继承关系
- **共用 W3C 类型约定**——Uint8Array, DOMHighResTimeStamp, AbortSignal 等
- **node:* 支持 nodeCompat**——根据配置版本适配输出

### 对上游组件的审核原则

kossjs / boa_runtime 的组件**不可盲目使用**，每个必须：
1. 审核上游源码，确认行为是否符合需求
2. 判断：适配使用 vs 自己实现
3. 即使适配使用，也要对齐 OOLONG 代码风格（2 空格、中文注释、测试覆盖）

## 当前代码状态

### 已完成模块

| 模块 | 文件 | 说明 |
|------|------|------|
| 核心引擎 | `src/runtime.rs`, `src/module_loader.rs` | Context + ModuleLoader |
| CJS→ESM | `src/cjs_to_esm.rs` | 静态 AST 转译 |
| TS→JS | `src/transpiler.rs` | OXC transformer |
| 模块解析 | `src/resolver.rs` | Node.js 风格 |
| 类型检查 | `src/typecheck.rs` | tsgo 调用 |
| CJS 运行时 | `src/cjs/mod.rs` | require + module + exports |
| std/fs | `src/std/fs.rs` | Rust 原生 |
| std/os | `src/std/os.rs` | Rust 原生 |
| std/path | `src/std/path.rs` | Rust 原生 |
| std/process | `src/std/process.rs` | Rust 原生 |
| node/* 19 模块 | `src/node/` | 9 Rust + 10 内联 JS（Phase B 待迁移） |
| web/* 6 模块 | `src/web/` | W3C 全局类 |

## 关键决策记录

### 2026-05-30 — 全栈 Rust + W3C 类型 + nodeCompat

**问题**：Node 兼容层的实现策略和长期维护方向。

**决策**：
1. 所有模块全栈 Rust 实现，零 JS 内联
2. W3C 类型为所有模块的一等公民类型约定
3. `nodeCompat` 配置控制 node:* 版本行为，用户无代码侵入
4. `std/` 和 `node/` 各自独立 Rust 实现，无包装依赖

**原因**：
- 全栈 Rust 消除语言边界，性能最优
- W3C 类型统一的未来保障（Node.js 自身也在向 Web 标准靠拢）
- nodeCompat 让用户无需改代码就能切换 Node 版本

### 2026-05-28 — 自维护 fork

**问题**：要不要继续依赖上游 kossjs？

**决策**：自维护 fork，后独立为 oolong 项目。

**原因**：
1. Boa 的 `ModuleLoader` 必须在 `ContextBuilder` 构建时注册，创建后无法替换
2. TS 即时转译、CJS→ESM 包装、koss 缓存解析是必须的功能，都需要侵入 ModuleLoader
3. 已经改了 4 个文件 ~350 行，合并回上游不现实

**同步策略**：上游更新时 git cherry-pick，4 个文件的冲突概率低。

### 2026-05-28 — CJS→ESM 静态转译

**问题**：CJS 和 ESM 混合的 npm 包如何统一执行？

**决策**：在 ModuleLoader 加载阶段做静态 AST 转译。

**方案**：OXC 解析 CJS → 采集 require/exports/__dirname → 源码改写为 ESM。

**原因**：运行时做 CJS↔ESM 互操作（如 Node.js 的 `createRequire`）复杂度高，静态转译更可靠且性能更好。

### 2026-05-28 — 标准库放 oolong 而非 cha

**问题**：W3C 和 Node 标准库应放在哪个项目？

**决策**：放 oolong。

**原因**：标准库是引擎的核心能力。如果放在 cha，那别人用 oolong 做嵌入式引擎时就拿不到标准库。

### 2026-05-29 — vendor oxc_transformer

**问题**：`oxc_transformer 0.133` 在 Rust 1.98 nightly 上编译失败（`if let` guard 语法兼容问题）。

**决策**：本地 vendor 补丁。

**方案**：`vendor/oxc_transformer/` 存放补丁版，`Cargo.toml` 通过 path dep 引用。保留原版未修改。

## 开发规范

- 中文为第一语言（注释、对话、thinking）
- OOP + 设计模式，每个文件只负责一类
- 2 空格缩进，提交前 `cargo fmt`
- Rust 最新版 + 最新语法
- 所有代码必须有测试
- 所有模块遵守 W3C 类型六条硬规则
- 新模块先列 API 清单再动手，在对话中协商确认
