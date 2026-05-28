# 讨论记录

## 2026-05-28 — 第一次：项目诞生

### 背景

kossjs 是一个基于 Boa 的 JS/TS 运行时，koss 是基于 Deno 风格的包管理 + CLI。经过多次讨论，决定将两者整合为：OOLONG（引擎）+ CHA（CLI/包管理）。

### 关键决策

**1. 为什么不自接用上游 kossjs？**
- ModuleLoader 必须在 ContextBuilder 构建时注入，外部覆盖不了
- 需要 TS 即时转译、CJS→ESM、koss 缓存解析 → 必须改 ModuleLoader

**2. 为什么标准库放 oolong 不放 cha？**
- 标准库是引擎的核心能力，嵌入式场景也要用

**3. CJS→ESM 怎么做？**
- 静态 AST 转译（OXC 解析 + 源码改写）
- 不做运行时 createRequire 风格

**4. 项目命名？**
- 引擎：oolong（乌龙茶 🍵）
- CLI：cha（茶 🍵）

**5. 为什么从零建仓库而不是原地 rename？**
- 清晰区分上游和我们
- 不保留上游 commit history 的包袱

### 待办事项

- [x] 创建设计文档
- [ ] 创建 oolong 项目骨架
- [ ] CJS→ESM 转译器
- [ ] 迁移 4 个侵入式文件
- [ ] ModuleLoader 集成
- [ ] 创建 cha 项目骨架
- [ ] 迁移包管理器代码
- [ ] tsgo 集成文档
