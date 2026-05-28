# OOLONG 乌龙茶 🍵 — 架构设计

## 项目定义

OOLONG 是自研的 JavaScript/TypeScript 运行时引擎，基于 kossjs 自维护 fork 发展而来。

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

## 架构分层

```
用户代码
  │
  ├─ import "fs"              import "node:fs"
  ▼
┌──────────────────────────────────────┐
│  OOLONG — 标准库注入层                │
│  ├─ std/（W3C 标准库，默认）          │
│  └─ node/（Node.js 兼容层）          │
├──────────────────────────────────────┤
│  OOLONG — 引擎核心                    │
│  ├─ runtime.rs                       │
│  ├─ module_loader.rs（含 CJS→ESM）   │
│  ├─ resolver.rs                      │
│  ├─ cjs_to_esm.rs（CJS 静态转译）    │
│  ├─ transpiler.rs（OXC TS→JS）      │
│  ├─ typecheck.rs（tsgo 调用）        │
│  └─ bindings.rs                      │
├──────────────────────────────────────┤
│  kossjs 上游                          │
│  └─ Boa 0.21 + OXC 0.133            │
└──────────────────────────────────────┘
```

## 标准库设计

| 前缀 | 走哪个标准库 | 文件位置 |
|------|-------------|----------|
| `import "fs"` | W3C 标准库（一等公民） | `src/std/fs.rs` |
| `import "node:fs"` | Node.js 兼容层 | `src/node/fs.rs` |

两套标准库完全独立，未来编译单二进制时可选择剔除 Node 兼容层。

## 关键决策记录

### 2026-05-28 — 自维护 fork

**问题**：要不要继续依赖上游 kossjs？

**决策**：自维护 fork。OOLONG 基于 kossjs 但全权维护。

**原因**：
1. Boa 的 `ModuleLoader` 必须在 `ContextBuilder` 构建时注册，创建后无法替换
2. TS 即时转译、CJS→ESM 包装、koss 缓存解析是必须的功能，都需要侵入 ModuleLoader
3. 我们已经改了 4 个文件 ~350 行，合并回上游不现实

**同步策略**：上游更新时 git cherry-pick，4 个文件的冲突概率低。

### 2026-05-28 — CJS→ESM 静态转译

**问题**：CJS 和 ESM 混合的 npm 包如何统一执行？

**决策**：在 ModuleLoader 加载阶段做静态 AST 转译。

**方案**：OXC 解析 CJS → 采集 require/exports/__dirname → 源码改写为 ESM。

**原因**：运行时做 CJS↔ESM 互操作（如 Node.js 的 `createRequire`）复杂度高，静态转译更可靠且性能更好。

### 2026-05-28 — 标准库放 oolong 而非 cha

**问题**：W3C 和 Node 标准库应放在哪个项目？

**决策**：放 oolong。

**原因**：标准库是引擎的核心能力。如果放在 cha，那别人用 oolang 做嵌入式引擎时就拿不到标准库。

## 自维护的 4 个侵入式文件

从上游 kossjs fork 衍生的改动：

| 文件 | 改动内容 |
|------|----------|
| `resolver.rs` | TS 扩展名（.ts/.tsx/.mts）、koss 缓存解析、package.json entry 发现 |
| `module_loader.rs` | CJS→ESM 集成、TS 即时转译、require/import 混合处理 |
| `lib.rs` | 暴露 transpiler、typecheck、cjs_to_esm 模块 |
| `Cargo.toml` | 加 OXC/tsgo 依赖 |

## 开发规范

- 中文为第一语言（注释、对话、thinking）
- OOP + 设计模式，每个文件只负责一类
- 2 空格缩进，提交前 `cargo fmt`
- Rust 最新版 + 最新语法
- 所有代码必须有测试
- 使用 `taolun.md` / `agents.md` / `changelog.md` 管理项目
