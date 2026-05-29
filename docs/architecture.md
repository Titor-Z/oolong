# OOLONG 乌龙茶 🍵 — 架构设计

## 项目定义

OOLONG 是自研的 JavaScript/TypeScript 运行时引擎。

- **ES6 ~ ES2026 ~ ESNext** 标准语法为第一等公民
- **TypeScript** 运行时，基于 OXC 转译 TS → ESNext
- **tsgo**（Microsoft 官方原生 TypeScript 7.0 检查器）进行类型检查
- **W3C 标准库**为一等公民（`import "fs"` → 走 W3C 标准）
- **Node.js 兼容层**（`import "node:fs"` → 走 Node 兼容）

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
  ├─ import "fs"              import "node:fs"
  │   (W3C 默认)               (Node 兼容)
  ▼
┌──────────────────────────────────────┐
│  OOLONG — 标准库注入层                │
│  ├─ std/（W3C 标准库，默认）✅        │
│  └─ node/（Node.js 兼容层）🏗️ 5.0-5.6│
├──────────────────────────────────────┤
│  OOLONG — 引擎核心                    │
│  ├─ runtime.rs（Boa Context 封装）    │
│  ├─ module_loader.rs（Boa ModuleLoader）│
│  ├─ resolver.rs（模块路径解析）       │
│  ├─ cjs_to_esm.rs（CJS→ESM 转译）   │
│  ├─ cjs/（CJS require 运行时）🏗️ 5.0 │
│  ├─ transpiler.rs（OXC TS→JS）      │
│  └─ typecheck.rs（tsgo 调用）         │
├──────────────────────────────────────┤
│  Boa 0.21 + OXC 0.133                │
│  └─ vendor/oxc_transformer（if let 补丁）│
└──────────────────────────────────────┘
```

## 标准库设计

### 使用风格

- **模块导入**（`import "fs"`），非全局对象（与 Deno/Bun 对齐）
- **异步优先**（`await fs.readFile(path)` 返回 `Promise`）
- **W3C 风格**为第一等公民，`node:` 前缀做兼容
- 三种 import 语法全支持：`import readFile from "fs"` / `import {readFile} from "fs"` / `import * as fs from "fs"`

### 前缀路由

| 前缀 | 走哪个标准库 | 文件位置 |
|------|-------------|----------|
| `import "fs"` | W3C 标准库（一等公民） | `src/std/fs.rs` |
| `import "node:fs"` | Node.js 兼容层 | `src/node/fs.rs` |

两套标准库完全独立，未来编译单二进制时可选择剔除 Node 兼容层。

### Node 兼容层设计约束

参考 Deno/Bun 的模式，两套标准库独立共存：

| 导入方式 | 走哪个标准库 | 文件位置 |
|----------|-------------|----------|
| `import "fs"` | W3C 标准库（一等公民） | `src/std/fs.rs` |
| `import "node:fs"` | Node.js 兼容层 | `src/node/fs.rs` |

实现策略：
- **Rust 层**：Buffer 的二进制操作、全局对象注册、模块注册管线
- **JS 层**：API 适配、callback 包装、面向用户的接口（通过 SyntheticModule 的 JS 字符串注入）

### 对上游组件的审核原则

kossjs / boa_runtime 的组件**不可盲目使用**，每个必须：
1. 审核上游源码，确认行为是否符合需求
2. 判断：适配使用 vs 自己实现
3. 即使适配使用，也要对齐 OOLONG 代码风格（2 空格、中文注释、测试覆盖）

### Blob / File / FileReader 策略

| API | 是否需要 | 原因 |
|-----|---------|------|
| `Blob` | ✅ 需要 | `fetch` Response body、`new Blob()`、`Response` 构造函数都依赖，Phase 6 实现 |
| `File` | ⚠️ 次优先 | `new File(parts, name)` W3C spec 定义，偶有用到 |
| `FileReader` | ❌ 跳过 | 浏览器 DOM API，服务端运行时无 `<input type="file">` 场景 |

Boa 0.21 不提供 Blob/File/FileReader，需自实现。

**当前状态**：标准库四个模块（path/fs/process/os）已全部实现。

## 关键决策记录

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

## 当前代码状态

### 已完成模块（98 测试，零 clippy 警告）

| 模块 | 文件 | 测试数 | 说明 |
|------|------|--------|------|
| CJS→ESM 转译器 | `src/cjs_to_esm.rs` | 13 | OXC AST → 源码改写 |
| TS→JS 转译器 | `src/transpiler.rs` | 11 | OXC parser + codegen + transformer |
| 模块解析器 | `src/resolver.rs` | 10 | Node.js 风格路径解析 |
| 类型检查 | `src/typecheck.rs` | 0 | 调用外部 tsgo 二进制 |
| ModuleLoader | `src/module_loader.rs` | 0 | Boa ModuleLoader trait 实现 |
| Runtime | `src/runtime.rs` | 7 | Context + ModuleLoader + Console |
| `import "path"` | `src/std/path.rs` | 24 | W3C 路径操作 |
| `import "process"` | `src/std/process.rs` | 16 | 进程信息 + stdin/stdout/stderr |
| `import "fs"` | `src/std/fs.rs` | 15 | 文件系统（含 e2e 集成测试） |
| `import "os"` | `src/std/os.rs` | 15 | 操作系统信息（单元+集成） |
| Blob / File | `src/std/blob.rs` | 6 | 全局类（构造/text/arrayBuffer/slice） |
| URLSearchParams | `src/std/url_search_params.rs` | 5 | 全局类（get/set/append/delete/sort） |
| URL / TextEncoder / fetch | boa_runtime 提供 | 3 | 全局类，通过 boa_runtime 注册 |

### 🏗️ 构建中 — Node 兼容层（Phase 5）

| 阶段 | 模块 | 状态 |
|------|------|------|
| 5.0 | 基础设施：CJS require + 全局对象 + module loader | 🔜 当前阶段 |
| 5.1 | `node:path` / `node:os` / `node:process` | ⏳ |
| 5.2 | `node:buffer` (Buffer) + `node:events` (EventEmitter) | ⏳ |
| 5.3 | `node:fs` (完整 callback + sync + promises) | ⏳ |
| 5.4 | `node:util` + `node:stream` + `node:url` | ⏳ |
| 5.5 | `node:crypto` + `node:child_process` + `node:module` | ⏳ |
| 5.6 | 剩余模块 (assert/tty/vm/zlib/querystring 等) | ⏳ |

## 开发规范

- 中文为第一语言（注释、对话、thinking）
- OOP + 设计模式，每个文件只负责一类
- 2 空格缩进，提交前 `cargo fmt`
- Rust 最新版 + 最新语法
- 所有代码必须有测试
