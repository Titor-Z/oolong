# OOLONG 开发计划 v3

> 2026-06-04 方向定稿。
>
> **定位变更**：不开源、不自称"运行时" → 自用 TS 执行器 + 现代标准库
> **核心假设**：tsgo + oxc + Boa 三层各自独立，任何一层可替换

---

## 项目重新定位

| 之前 | 现在 |
|------|------|
| 开源 | 自用，不开源 |
| npm CJS 兼容（Express、webpack） | 纯 ESM，不跑 npm 包 |
| 通用 JS runtime | TS-first 执行器 |
| 需兼容 Node API 全貌 | 只实现需要的 `@std/*` |
| 用户写 `.mjs/.cjs` | 用户写 `.ts` |

**CHA** = TS 执行器 + 标准库 + 包框架（简化为项目脚手架/配置）

## 三层 Pipeline

```
.ts 源文件
  │
  ├─ Step 1: tsgo (类型检查)
  │   └── 可选的编译开关 `--check`，默认打开
  │   └── 依赖外部 tsgo 二进制
  │
  ├─ Step 2: oxc (strip types)
  │   └── 纯 Rust，解析 TS AST → 剥离类型注解 → 产出 JS AST/文本
  │   └── 产出干净 ESM（无 CJS wrapper）
  │
  ├─ Step 3: Boa (执行 JS)
  │   └── 执行脱类型后的 ESM 代码
  │   └── 加载 `@std/*` 模块（Rust 原生 FFI）
  │
  └─ Step 4: @std/* (标准库)
      └── 三层架构（见下文）
```

## 三层标准库架构

### L1：W3C 通用层（全局挂载，100% 跨平台）

对标浏览器标准，不增不减：

| 模块 | 状态 |
|------|------|
| `fetch` / `Request` / `Response` | ✅ |
| Web Streams | ✅ |
| `Blob` / `File` | ✅ |
| `Event` / `EventTarget` | ✅ |
| `AbortController` / `AbortSignal` | ✅ |
| `URL` / `URLSearchParams` | 待补 |
| `TextEncoder` / `TextDecoder` | 待补 |
| `console` | ✅ |
| `setTimeout` / `setInterval` / `setImmediate` | 待补 |
| Web Crypto | 待补 |

### L2：`@std/*` 增强层（import 加载，现代 API 面）

借鉴 Node 使用场景，但不照搬 Node 实现：

| 模块 | 说明 | 优先级 |
|------|------|--------|
| `@std/path` | 路径拼接/解析 | P0 |
| `@std/fs` | 文件读写（Promise 异步，无 callback） | P0 |
| `@std/env` | 环境变量/启动参数 | P0 |
| `@std/process` | 进程基本能力（exit、cwd、argv） | P0 |
| `@std/net` | TCP/UDP（基于 Web Streams） | P1 |
| `@std/http` | HTTP 客户端/服务端 | P1 |
| `@std/encoding` | base64/hex | ✅ |
| `@std/uuid` | UUID v4 | ✅ |
| `@std/semver` | 语义版本 | ✅ |
| `@std/log` | 结构化日志 | ✅ |

### L3：`@std/types` 运行时底层（内建，无需 import）

预留层，Phase 3 启用。存放 i32/i64/u32/u64/string 等强类型的 Rust 运算内核。

### 命名

- L1：全局对象（`fetch()`, `new URL()`），无前缀
- L2：`@std/<name>`（`import { readFile } from "@std/fs"`）
- L3：`@std/types` 命名空间下（Phase 3）

## Feature Gates

```toml
[features]
default = []
# npm 生态兼容（Node 模块 + CJS require）
node-compat = ["dep:node_std"]
npm-cjs = ["dep:cjs_loader", "node-compat"]
# 强类型扩展（i32/i64 等，Phase 3）
strict-types = []
```

- **默认二进制**：只有 `@std/*` + Boa + oxc，体积最小
- **`--features node-compat`**：加入 `node:*` 模块（保留现有代码）
- **`--features npm-cjs`**：加入 CJS require + 包加载器
- **`--features strict-types`**：加入强类型系统

## Boa 边界探索任务

Phase 1 执行，用来决定是否继续用 Boa：

| # | 任务 | 判断标准 |
|---|------|---------|
| 1 | ESM 脱类型后能否正确执行 | `cha run app.ts` 跑通简单 TS 文件 |
| 2 | Rust 原生函数（`@std/*`）能否正常挂载和调用 | `@std/fs.readFile` 工作 |
| 3 | 基础 API 性能（循环 10^7 次、字符串操作） | 不比 Node 慢 10x 以上 |
| 4 | 内存泄漏测试 | 连续执行 1000 次脚本后 RSS 不持续增长 |
| 5 | var 声明 bug 是否影响 ESM-only 代码 | 测试确认无影响 |
| 6 | Proxy 基础能力 | `new Proxy({}, { get() {} })` 正常工作 |

**退出条件**：如果 3 项以上失败或用户决定切换 → 替换为 QuickJS-wasm / 其他

## Type Check 策略（tsgo）

tsgo 是微软 typescript-go 的包装，项目已有源码在 `~/tsgo/`。通过 CLI 调用：

```
tsgo --noEmit --lib es2022 app.ts
```

OOLONG 通过 `typecheck.rs` 调用（`std::process::Command`）：

```rust
pub fn check(path: &Path) -> Result<Vec<Diagnostic>> {
    let output = Command::new("tsgo")
        .arg("--noEmit")
        .arg("--lib")
        .arg("es2022")
        .arg(path)
        .output()?;
    // parse stdout → Vec<Diagnostic>
}
```

可选的 `--check` 开关，默认 true。

## Phase 里程碑

### Phase 1：TS 全链路（当前 sprint）

- [ ] `cha run app.ts` 跑通
- [ ] oxc pipeline（parse → strip types → codegen JS）
- [ ] Boa 执行脱类型后的 JS ESM
- [ ] tsgo 类型检查集成（`cha check` / `cha run --check`）
- [ ] Boa 边界验证完成 → 决定是否保留 Boa
- [ ] `@std/path` + `@std/env` 基础模块
- [ ] Feature gate 实现（node-compat, npm-cjs 条件编译）

### Phase 2：标准库成型

- [ ] W3C 通用层补全（URL、TextEncoder、timers、crypto）
- [ ] `@std/fs` 完整实现
- [ ] `@std/net` TCP/UDP
- [ ] `@std/http` HTTP 客户端
- [ ] 缓存机制（`cha build` 生成缓存 .js）

### Phase 3：强类型扩展

- [ ] `@std/types` 模块设计
- [ ] `let x: i32 = 10` 编译识别（oxc 侧）
- [ ] 运行时 `i32` 值包装（Rust FFI）
- [ ] 类型转换规则（`i32(x)` 显式转换）
- [ ] `--features strict-types` gate
- [ ] `i64` / `u32` / `u64` 跟进
- [ ] 字面量自动分类（`let a = 1` → i64）

## 保留代码（不删除，只 feature gate）

```
oolong/src/
├── cjs/              #[cfg(feature = "npm-cjs")]
├── node/             #[cfg(feature = "node-compat")]
├── module_loader.rs  # 部分路径 cfg 隔离
cha/src/
├── install.rs        #[cfg(feature = "npm-cjs")]
└── remove.rs         #[cfg(feature = "npm-cjs")]

vendor/
├── boa_ast/          # 保留 vendored，不出问题就不改
└── boa_engine/       # 同上
```

## 替换策略

任何一层都可替换，不影响其他层：

| 层 | 当前 | 可选替代 |
|----|------|---------|
| Type check | tsgo (Go) | tsc (JS)、自己写轻量检查器 |
| Parser + transform | oxc | swc、esbuild |
| JS 引擎 | Boa | QuickJS、Hermes、V8 (deno_core) |
| 标准库 | 自研 `@std/*` | 借用 deno 或 bun 的部分实现思路 |
