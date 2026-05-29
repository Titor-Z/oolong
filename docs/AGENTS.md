# OOLONG 代码规范

## 认知修正（踩坑记录）

| 日期 | 问题 | 修正 |
|------|------|------|
| 2026-05-29 | `Expression::CallExpression` 不是 `Expression::Call` | OXC 0.133 AST 命名与旧版不同，grep 源码确认 |
| 2026-05-29 | `BindingPattern` 是枚举不是 struct | 需 match 而非 `.kind` |
| 2026-05-29 | `oxc_transformer 0.133` Rust 1.98 nightly `if let` guard 失败 | vendor 补丁：拆分 match arm 条件；Cargo.toml path dep |
| 2026-05-29 | `Boa 0.21.1` 的 `property::Attribute::all()` 是 bitflags 方法 | 直接可用，无需 builder |

## 构建命令

```sh
# 构建
cargo build --lib

# 测试（全部 98 测试）
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

## 标准库设计约束

1. **模块导入**（`import "fs"`），**非全局对象**
2. **异步优先**（`await fs.readFile(path)` 返回 `Promise`），除非语义明确要求同步（如 `path.join`）
3. **三元标准库体系**：
   - `web/` — W3C Web API（Blob、URLSearchParams 等全局类）
   - `std/` — OOLONG 原生模块（不是 W3C 标准，是自定 API 面）
   - `node/` — Node.js 兼容层（`node:` 前缀）
4. **OOLONG 原生层是自定标准**：参考 Deno/Bun/Node 三家设计，不是 W3C 标准（如 `import "os"` 浏览器没有）。互补 API 最终都会加进来
5. 三种 import 语法全支持：default / named / namespace
6. **上游组件不可盲目使用**：每个先审核源码，判别适配使用 vs 自己实现
7. **每个新模块必须先列 API 清单再动手**：对照 Node/Deno/Bun 三家 API，确定实现范围和优先级。清单**先在对话中协商**，达成一致后保存到 `docs/stdlib-api.md` 再执行
8. **不允许偷懒**：API 清单中约定好的功能，只要技术上可行就必须实现，不能因为「麻烦」跳过。确实有困难的（如依赖缺失、底层限制），先调研可替代方案并向用户说明，由用户决策是否跳过

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
│   ├── cjs/（CJS require 运行时 ✅ 5.0）
│   │   └── mod.rs（require + module + exports 实现）
│   ├── transpiler.rs（OXC TS→JS）
│   ├── typecheck.rs（tsgo 调用）
│   ├── web/（W3C Web API ✅）
│   │   ├── mod.rs
│   │   ├── blob.rs（Blob + File 全局类）
│   │   └── url_search_params.rs（URLSearchParams 全局类）
│   ├── std/（OOLONG 原生模块 ✅）
│   │   ├── mod.rs
│   │   ├── path.rs（W3C 路径操作）
│   │   ├── process.rs（进程信息 + stdin/stdout/stderr）
│   │   ├── fs.rs（文件系统）
│   │   └── os.rs（操作系统信息）
│   └── node/（Node.js 兼容库 🏗️ 5.0-5.6）
│       ├── mod.rs
│       ├── path.rs（node:path 🔜 5.1）
│       ├── os.rs（node:os 🔜 5.1）
│       ├── process.rs（node:process ✅ 5.0）
│       ├── buffer.rs（node:buffer + Buffer 全局 ✅ 5.0）
│       ├── events.rs（node:events 🔜 5.2）
│       ├── fs.rs（node:fs + constants + promises 🔜 5.3）
│       ├── util.rs（node:util 🔜 5.4）
│       ├── stream.rs（node:stream 🔜 5.4）
│       └── ...
├── tests/
│   └── runtime_test.rs（e2e 集成测试）
└── docs/
    ├── agents.md
    ├── architecture.md
    ├── changelog.md
    ├── stdlib-api.md
    └── taolun.md
```

## 测试策略

- 每个模块有单元测试（`#[cfg(test)] mod tests`）
- 集成测试在 `tests/` 目录，涉及文件 I/O
- e2e 测试创建临时目录，写文件，执行，清理
- 提交前必须 `cargo test && cargo clippy --all-targets && cargo fmt`
