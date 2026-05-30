# OOLONG 代码规范

## 认知修正（踩坑记录）

| 日期 | 问题 | 修正 |
|------|------|------|
| 2026-05-29 | `Expression::CallExpression` 不是 `Expression::Call` | OXC 0.133 AST 命名与旧版不同，grep 源码确认 |
| 2026-05-29 | `BindingPattern` 是枚举不是 struct | 需 match 而非 `.kind` |
| 2026-05-29 | `oxc_transformer 0.133` Rust 1.98 nightly `if let` guard 失败 | vendor 补丁：拆分 match arm 条件；Cargo.toml path dep |
| 2026-05-29 | `Boa 0.21.1` 的 `property::Attribute::all()` 是 bitflags 方法 | 直接可用，无需 builder |
| 2026-05-30 | W3C 类型为所有模块的一等公民约定 | std/ 和 node/ 各自独立 Rust 实现，共用类型契约 |
| 2026-05-30 | `nodeCompat` 配置控制 node:* 版本输出 | 用户无需代码侵入，oolong.json 配置即可 |
| 2026-05-30 | 所有模块必须全栈 Rust，禁止内联 JS | 10 个 JS 模块列入 Phase B 迁移计划 |

## 构建命令

```sh
# 构建
cargo build --lib

# 测试（全部 304 测试 ✅）
cargo test

# 仅单元测试
cargo test --lib

# 仅集成测试
cargo test --test runtime_test

# Lint
cargo clippy --all-targets

# 格式化
cargo fmt

# 发布检查
cargo package
```

## 代码风格

- 遵循 Rust 最新版标准
- 中文注释
- 2 空格缩进
- 错误处理使用 `Result<T, String>`（暂未引入 anyhow）
- 所有代码必须有测试
- 所有模块遵守 W3C 类型六条硬规则（见 architecture.md）
- `node:*` 内部 Rust 实现用 W3C 类型，对外按 nodeCompat 配置适配
- `std/` 始终 W3C，不受 nodeCompat 影响

## 类型文件规范

### 工作流

每个模块的开发流程：

1. **先写 `types/`** — API 设计先行，确定导出名、参数签名、返回值类型
2. **再写 Rust 实现** — 照着 `.d.ts` 签名实现
3. **类型校验测试** — 测试中验证运行时导出与 `.d.ts` 一致
4. **提交** — Rust + `.d.ts` + 测试一同提交

### 目录结构

```
types/
├── oolong.d.ts      ← 入口，reference path 指向所有子模块
├── std/             ← @std/* 模块
│   ├── http.d.ts
│   ├── path.d.ts
│   ├── fs.d.ts
│   ├── os.d.ts
│   └── process.d.ts
├── node/            ← node:* 模块（🏗️ Phase B 迁移时同步创建）
│   └── ...
└── web/             ← W3C 全局类
    └── ...
```

### 类型一致性测试

`tests/runtime_test.rs` 中的 `test_type_consistency_*` 测试负责验证：

- 默认导出（`import mod from "@xxx/yyy"`）的 `Object.keys()` 匹配 `.d.ts` 声明
- 每个导出的 `typeof` 匹配声明的类型
- 新增 API 必须同步添加类型校验测试

### 规则

- 类型文件不是文档的替代，而是可执行的 API 契约
- 不写 `.d.ts` 等同于没写 API

## 标准库设计约束

1. **模块导入**（`import "fs"`），**非全局对象**
2. **异步优先**（`await fs.readFile(path)` 返回 `Promise`），除非语义明确要求同步（如 `path.join`）
3. **三元标准库体系，全部全栈 Rust**：
   - `web/` — W3C Web API（Blob、URLSearchParams 等全局类）
   - `std/` — OOLONG 原生模块（自定 API 面，始终 W3C，不受 nodeCompat 影响）
   - `node/` — Node.js 兼容层（`node:` 前缀，nodeCompat 控制输出）
4. **W3C 类型为一等公民**：std/ 和 node/ 各自独立 Rust 实现，共用类型约定（Uint8Array, DOMHighResTimeStamp, AbortSignal…），不共享代码层
5. **裸名路由**：`oolong.json` 中有 `nodeCompat` → 裸名路由到 `node:*`；无 `nodeCompat` → 裸名路由到 `@std/*`
6. **全栈 Rust，禁止内联 JS**：10 个现有 JS 模块（assert/querystring/timers/vm/url/events/path/stream/util/module）列入 Phase B 迁移计划
7. 三种 import 语法全支持：default / named / namespace
8. **上游组件不可盲目使用**：每个先审核源码，判别适配使用 vs 自己实现
9. **每个新模块必须先列 API 清单再动手**：对照 Node/Deno/Bun 三家 API，确定实现范围和优先级。清单**先在对话中协商**，达成一致后保存到 `docs/stdlib-api.md` 再执行
10. **不允许偷懒**：API 清单中约定好的功能，只要技术上可行就必须实现，不能因为「麻烦」跳过。确实有困难的（如依赖缺失、底层限制），先调研可替代方案并向用户说明，由用户决策是否跳过

## 架构

```
oolong/
├── Cargo.toml
├── vendor/oxc_transformer/（补丁版）
├── src/
│   ├── lib.rs（模块声明）
│   ├── runtime.rs（Context + ModuleLoader + Console）
│   ├── module_loader.rs（Boa ModuleLoader trait 实现）
│   ├── resolver.rs（Node.js 风格路径解析）
│   ├── cjs_to_esm.rs（CJS→ESM 静态转译）
│   ├── cjs/（CJS require 运行时 ✅）
│   │   └── mod.rs（require + module + exports 实现）
│   ├── transpiler.rs（OXC TS→JS）
│   ├── typecheck.rs（tsgo 调用）
│   ├── web/（W3C Web API ✅ — 自实现，替换 boa_runtime）
│   │   ├── mod.rs
│   │   ├── blob.rs（Blob + File 全局类）
│   │   ├── event.rs（Event + EventTarget）
│   │   ├── abort.rs（AbortController + AbortSignal）
│   │   ├── base64.rs（atob + btoa）
│   │   ├── performance.rs（Performance API）
│   │   ├── url_search_params.rs（URLSearchParams 全局类）
│   │   ├── headers.rs（Headers 类 — 自实现 ✅）
│   │   ├── response.rs（Response 类 — 自实现 ✅）
│   │   ├── request.rs（Request 类 — 自实现 ✅）
│   │   └── fetch.rs（fetch 函数 — 自实现 ✅）
│   ├── std/（OOLONG 原生模块 ✅ — 始终 W3C，不受 nodeCompat 影响）
│   │   ├── mod.rs
│   │   ├── path.rs
│   │   ├── process.rs
│   │   ├── fs.rs
│   │   ├── os.rs
│   │   └── http.rs（🏗️ Phase A）
│   └── node/（Node 兼容模块 ✅ 19 模块 — nodeCompat 控制输出）
│       ├── mod.rs
│       ├── buffer.rs（Rust ✅）
│       ├── process.rs（Rust ✅）
│       ├── os.rs（Rust ✅）
│       ├── fs.rs（Rust ✅）
│       ├── crypto.rs（Rust ✅）
│       ├── child_process.rs（Rust ✅）
│       ├── zlib.rs（Rust ✅）
│       ├── tty.rs（Rust ✅）
│       ├── perf_hooks.rs（Rust ✅）
│       ├── path.rs（🏗️ Phase B JS→Rust）
│       ├── events.rs（🏗️ Phase B JS→Rust）
│       ├── stream.rs（🏗️ Phase B JS→Rust）
│       ├── util.rs（🏗️ Phase B JS→Rust）
│       ├── module.rs（🏗️ Phase B JS→Rust）
│       ├── url.rs（🏗️ Phase B JS→Rust）
│       ├── assert.rs（🏗️ Phase B JS→Rust）
│       ├── querystring.rs（🏗️ Phase B JS→Rust）
│       ├── timers.rs（🏗️ Phase B JS→Rust）
│       └── vm.rs（🏗️ Phase B JS→Rust）
├── tests/
│   └── runtime_test.rs（e2e 集成测试）
└── docs/
    ├── agents.md
    ├── architecture.md
    ├── changelog.md
    ├── plan-v2.md（架构 v2 计划）
    ├── stdlib-api.md
    └── taolun.md
```

## 测试策略

- 每个模块有单元测试（`#[cfg(test)] mod tests`）
- 集成测试在 `tests/` 目录，涉及文件 I/O
- e2e 测试创建临时目录，写文件，执行，清理
- 提交前必须 `cargo test && cargo clippy --all-targets && cargo fmt`
