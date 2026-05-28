# OOLONG 代码规范

## 认知修正（踩坑记录）

| 日期 | 问题 | 修正 |
|------|------|------|

## 构建命令

TBD

## 代码风格

- 遵循 Rust 最新版标准
- 中文注释
- 2 空格缩进
- OOP + 设计模式，每个文件只负责一类
- 错误处理使用 `anyhow::Result`
- 所有代码必须有测试

## 架构

```
oolong/
├── src/
│   ├── lib.rs
│   ├── runtime.rs
│   ├── module_loader.rs（fork: +CJS→ESM + TS 即时转译）
│   ├── resolver.rs（fork: +TS ext + koss cache）
│   ├── cjs_to_esm.rs
│   ├── transpiler.rs
│   ├── typecheck.rs
│   ├── bindings.rs
│   ├── embedded_stdlib.rs
│   ├── worker.rs
│   ├── std/（W3C 标准库）
│   └── node/（Node.js 兼容库）
├── docs/
│   ├── agents.md
│   ├── architecture.md
│   ├── changelog.md
│   └── taolun.md
```

## 自维护的 fork 文件

与上游 kossjs 的差异：

| 文件 | 改动 | 原因 |
|------|------|------|
| `module_loader.rs` | +CJS→ESM 集成 + TS 即时转译 | 必须侵入 ModuleLoader trait |
| `resolver.rs` | +TS 扩展名 + koss 缓存解析 | resolver 内部逻辑 |
| `lib.rs` | +pub mod 暴露 | 必须改模块声明 |
| `Cargo.toml` | +OXC/tsgo deps | 依赖声明 |
