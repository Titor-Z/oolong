# OOLONG 代码规范

## 语言规范

从本文件被加载后起：

- AI 的所有对话输出必须使用**中文**
- AI 的内部思考过程（`thinking`）也**必须使用中文**
- 代码中的标识符（变量名、函数名、类型名等）保持英文惯例，不需要翻译成拼音
- 此项覆盖任何其他规范中的语言要求
- **原因**：用户不会英语，需要全程掌握 AI 的实时动态

## 模块拆分规范

为控制单文件大小、减少 tokens 消耗、提升 AI 读写效率：

1. **硬限制**：单个 `.rs` 文件不超过 **500 行**（不含测试模块）
2. **超过即拆**：超过 500 行的模块必须拆为子模块
3. **拆分模式**：
   ```
   src/xxx/large_module/
   ├── mod.rs      ← 仅 pub mod 声明 + 注册函数（register_globals / create_xxx_module）
   ├── core.rs     ← 共享数据结构、内部函数、类型别名
   ├── part_a.rs   ← 类 A 的实现
   ├── part_b.rs   ← 类 B 的实现
   └── ...
   ```
4. **拆分标准**：
   - 每个 W3C 类一个文件（如 `readable.rs`、`writable.rs`）
   - `mod.rs` 只做模块声明、`pub use`、注册函数
   - 共享类型放 `core.rs` 或直接放 `mod.rs`
5. **例外**：辅助性小模块（≤300 行纯函数，无复杂状态）允许单文件
6. **`mod.rs` 不自含实现逻辑**，只做路由和声明

## 分步实施规范

每个大模块必须拆成多个独立步骤实施，不可一次性全部实现：

1. **步骤数**：每个模块拆成 **3-5 个步骤**
2. **每步产出**：可编译、可通过 `cargo test && cargo clippy`
3. **步间依赖**：
   - 前一步的可交付物是后一步的基础
   - 每步完成后 commit 或记录 checkpoint
4. **步骤划分原则**：
   - Step 1：类型定义（`.d.ts`）+ 空壳注册（让 `import` 能通）
   - Step 2：基础数据结构 + 最简单的路径（happy path）
   - Step 3：核心功能补全
   - Step 4：高级功能 + 边界情况处理
   - Step 5：集成测试 + clippy 清理
5. **验收条件**：每步结束后执行 `cargo test && cargo clippy --all-targets && cargo fmt`

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
| 2026-05-30 | macOS `grep -P` 不存在，需用 `sed` 替代 | 在 macOS 上使用 `grep -oP` 会报错，应改用 `sed` |
| 2026-05-30 | `stream.pipeline` 在多测试二进制中 globalThis 不持久，但数据流本身正常 | 测试框架 artifact，改用 `on("data")` 覆盖测试，`pipeline` 作为 `r.pipe(w)` 语法糖保留 |
| 2026-05-30 | 同一问题循环 3 次无解必须停止，向用户说明现状和可选路径 | 协作规则已写入 AGENTS.md「三击退出规则」 |
| 2026-05-30 | `node:stream` 的 `pipe/pipeline` 是 Node 公认的历史包袱 | 功能降级：单级 pipe + 基础读写 = MVP，pipeline 做语法糖，不深挖 |

## 协作规则

### 三击退出规则

同一个问题循环 3 次没有解决（debug 验证、方案变更、重构等尝试后仍无效），必须：
1. **强制停止** — 不再继续尝试
2. **说明现状** — 向用户清晰描述：尝试了什么、现象是什么、卡在哪
3. **提供路径** — 列出可选方案（继续深挖、绕过去、标记已知问题等）
4. **等待决策** — 让用户选择方向，不得自行决定

## 构建命令

```sh
# 构建
cargo build --lib

# 测试（全部 331 测试 ✅）
cargo test

# 仅单元测试（49 个 Rust 内联测试）
cargo test --lib

# 仅集成测试（282 个，分布在 tests/*.rs）
# 按模块运行：
cargo test --test std_http    # @std/http
cargo test --test std_fs      # @std/fs
cargo test --test std_process # @std/process
cargo test --test std_path    # @std/path
cargo test --test std_os      # @std/os
cargo test --test node_events # node:events
cargo test --test web_globals # Blob/URL/Event/global APIs

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
│   │   ├── fetch.rs（fetch 函数 — 自实现 ✅）
│   │   └── streams/（Phase C 🏗️ — W3C Web Streams）
│   │       ├── mod.rs      ← pub mod + register_globals
│   │       ├── readable.rs ← ReadableStream + Reader + Controller
│   │       ├── writable.rs ← WritableStream + Writer + Controller
│   │       ├── transform.rs← TransformStream + Controller
│   │       └── strategy.rs ← CountQueuingStrategy + ByteLengthQueuingStrategy
│   ├── std/（OOLONG 原生模块 ✅ — 始终 W3C，不受 nodeCompat 影响）
│   │   ├── mod.rs
│   │   ├── path.rs
│   │   ├── process.rs
│   │   ├── fs.rs
│   │   ├── os.rs
│   │   ├── http.rs（🏗️ Phase A）
│   │   └── encoding.rs（Phase C 🏗️ — base64 + hex）
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
│       ├── events.rs（Rust ✅）
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
