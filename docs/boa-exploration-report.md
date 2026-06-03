# Boa Engine 现阶段探索报告

> 基于 oolong 项目集成 Boa 0.21.1 过程中的实践与踩坑记录
> 2026-06-03

---

## 一、背景

[oolong](https://github.com/anomalyco/oolong) 是一个基于 Rust 的 JavaScript/TypeScript 运行时，选择 [Boa](https://boa-dev.github.io/) 作为 JS 引擎（v0.21.1）。在集成过程中，我们实现了 npm 包管理（CJS 加载）、Node 兼容模块、HTTP 服务器等基础设施。当跑通 E2E Express 应用时，触发了 Boa 引擎深处的一个 `var` binding 编译错误。

本报告系统梳理此次踩坑的根因、已发现的引擎问题、适用边界以及后续方向。

---

## 二、根因分析：`var` binding miscompilation

### 2.1 触发条件

函数体内存在 **5 个以上不被闭包引用的顶级 `var` 声明**。典型的场景是 CJS 模块加载：

```js
// CJS wrapper (5 个参数)
(function(exports, require, module, __filename, __dirname) {
  // 模块源码，包含大量 var 声明
  var app = exports = module.exports = {};
  var slice = Array.prototype.slice;
  var flatten = Array.prototype.flat;
  // ... 更多 var
})
```

当 `var` 声明的 binding 数量加上 `arguments` binding 后超出函数参数数量时，触发越界 panic。

### 2.2 触发路径

```
cha run app.mjs
  → import express from "express"
    → load_cjs_file(express/index.js)
      → load_cjs_file(express/lib/express.js)
        → load_cjs_file(body-parser/index.js)
          → load_cjs_file(merge-descriptors/index.js)
            → load_cjs_file(express/lib/application.js)  ← HERE
```

前 4 个文件的 `var` 声明要么被闭包引用（escaping），要么数量少；`application.js` 有 **43 处 `var` 出现**，其中大量声明后不跨闭包使用 → 触发 bug。

### 2.3 根因链（5 个环节）

#### 环节 1：`create_mutable_binding` 存入原始 vec 位置

`boa_ast/src/scope.rs:357-376`

```rust
let binding_index = bindings.len() as u32;    // ← 绝对位置，如 5
bindings.push(Binding { index: binding_index, ... });
BindingLocator::declarative(..., binding_index, ...) // ← 存入 BindingLocator
```

#### 环节 2：scope analyzer 标记逃逸状态

`BindingEscapeAnalyzer` 遍历 AST，只把被闭包引用、或在 `eval`/`with` 作用域内的 binding 标记为 `ESCAPES`。无跨函数引用的 `var` 保持 non-escaping。

#### 环节 3：`reorder_binding_indices` 只修了 `Binding.index`

`boa_ast/src/scope.rs:225-232`

```rust
for binding in bindings.iter_mut() {
    if !binding.escapes() {
        binding.index = 0;   // ← 只有 Scope 内的 Binding 对象被改
        continue;
    }
    binding.index = index;   // ← escaping 的重排为 0..N
    index += 1;
}
```

但**之前已创建并下发到 `CodeBlock` 的 `BindingLocator` 对象不会被回溯更新**。

#### 环节 4：FunctionEnvironment 只分配 N 个 slot

`boa_engine/src/environments/runtime/mod.rs:210`

```rust
let num_bindings = scope.num_bindings_non_local(); // 只算 escaping binding
FunctionEnvironment::new(num_bindings, ...)
```

比如 5 个参数全部 escaping → runtime 环境有 5 个 slot（索引 0-4）。

#### 环节 5：操作码用旧索引访问 runtime 环境 → OOB

`DefInitVar::operation` (`vm/opcode/define/mod.rs:53-55`)：

```rust
let mut binding_locator = frame.code_block.bindings[index].clone(); // binding_index = 5
context.find_runtime_binding(&mut binding_locator)?;    // 非 poisoned 环境→直接 return
context.set_binding(&binding_locator, value, strict)?;  // env[5] → panic
```

`find_runtime_binding` 只在环境 `poisoned`（`with`/`eval`）时才重新解析 `binding_index`；干净环境直接 return，留下老的 vec 位置索引。

`GetName::operation` (`vm/opcode/get/name.rs:22-25`) 有完全相同的 bug：

```rust
let mut binding_locator = context.vm.frame().code_block.bindings[usize::from(index)].clone();
context.find_runtime_binding(&mut binding_locator)?;
let result = context.get_binding(&binding_locator)??.ok_or_else(|| ...)?;  // OOB
```

### 2.4 为什么说这是 Boa 的 bug

ECMAScript 规范 (10.2.11 `FunctionDeclarationInstantiation`) 要求：函数内**所有** `var` 声明（无论是否被引用）都必须在 VariableEnvironment 中创建 mutable binding 并初始化为 `undefined`。Boa 为了优化把 non-escaping `var` 移出了 runtime 环境，但字节码发射阶段仍为它们生成 `DefInitVar`/`GetName` 等操作码，操作码中的 binding 索引指向 vec 绝对位置，与 runtime 环境的 slot 编号不一致。

**这是编译器前后端（scope analyzer ↔ bytecode emitter ↔ VM）三阶段的索引一致性断裂问题。**

---

## 三、已发现的 Boa 0.21.1 Bug 清单

| 编号 | 位置 | 操作码 | 症状 | 状态 |
|------|------|--------|------|------|
| 1 | `vm/opcode/define/mod.rs:55` | `DefInitVar` | 初始化 non-escaping var 时 OOB | 已定位，已打 runtime 补丁 |
| 2 | `vm/opcode/get/name.rs:25` | `GetName` | 读取 non-escaping var 时 OOB | 已定位，未修补 |
| 3 | `vm/opcode/..` | `SetName` (推测) | 赋值 non-escaping var 时 OOB | 未验证 |
| 4 | `vm/opcode/..` | `DeleteName` (推测) | 删除 non-escaping var 时 OOB | 未验证 |
| 5 | `vm/opcode/define/mod.rs:..` | `DefVar` (推测) | 声明 non-escaping var 时 OOB | 未验证 |

**影响面**：所有在函数作用域内操作 `var` binding 的操作码都可能受此 bug 影响。核心问题不在操作码而在 binding 索引一致性的源头。

---

## 四、适用边界

### ✅ 工作良好

| 场景 | 说明 |
|------|------|
| ES6+ Module (`import`/`export`) | Module 环境使用 `ModuleEnvironment`，走不同路径 |
| `let`/`const` 声明 | 走 `PutLexicalValue`，不触发 `DefInitVar` |
| 箭头函数 | 无独立 `arguments` binding，参数数通常少 |
| 无 `var` 的现代 JS | 直接用 `let`/`const`，完全绕过此 bug |
| 简单 CJS 文件 | 少量 var 且全部 escaping 时偶然而不失 |
| 全局作用域代码 | 使用 GlobalDeclarative 环境，不受函数 scope 影响 |

### ⚠️ 高风险

| 场景 | 说明 |
|------|------|
| npm CJS 模块（大量 `var` 模式） | Express 的 `application.js`（43 var）、`route.js`、`middleware/init.js` 等 |
| legacy JS 库（jQuery、lodash 等） | 大量 `var` 声明 + 内部闭包组合 |
| 手动 CJS wrapper（`function(exports, ...){...}`） | 5+ 参数 + 模块内 var |

### ❌ 不可用

| 场景 | 说明 |
|------|------|
| 任意有 5+ non-escaping var 的函数 | 直接触发 OOB panic |
| `with` 语句 + 大量 var | poisoned 环境可能触发不同代码路径但 OOB 风险仍在 |

---

## 五、方向建议

### 短期（当前 sprint）

**Vendor Boa 源码到 oolong 仓库**，在 `reorder_binding_indices` 或字节码发射阶段修复合入性修复：

1. **方案 A**：修 `reorder_binding_indices`，让它同时更新 `CodeBlock.bindings` 中的 `BindingLocator`
2. **方案 B**：修字节码发射（`function_declaration_instantiation`），对 non-escaping binding 不发射 `DefInitVar`/`GetName`
3. **方案 C**：在 runtime 层（`PoisonableEnvironment::set`）加 bounds-check 静默跳过（当前选择，最轻量）

### 中期（1-2 月）

- 升级 Boa 到已修复版本（关注 Boa 项目 release notes 中关于 `num_bindings_non_local` / `reorder_binding_indices` 的修复）
- 建立 npm 生态兼容性 CI：随机 100+ npm package 加载测试

### 长期（3-6 月）

- **评估引擎替换**：Boa 0.21 仍处于早期阶段，有多个设计层面的问题。替代方案：
  - [QuickJS](https://bellard.org/quickjs/) via Rust binding (`quickjs-wasm-rs`)
  - 自研轻量 VM（基于字节码解释器，对标 boa 但放弃 `var` 优化激进性）
- 如果继续使用 Boa，考虑贡献修复 upstream

---

## 六、实践教训

1. **JavaScript 生态的保守性**：npm 上大量 CJS 模块广泛使用 `var`，即使是 2026 年。引擎必须完整支持 ES5 语法特性。
2. **optimization 不能违反 spec**：Boa 试图优化掉 non-escaping var 的 runtime slot，但字节码没对齐 → 语义错误。编译器优化必须以 spec 正确性为前提。
3. **测试策略**：引擎级测试需要覆盖"大量 var + 嵌套闭包 + 不同参数数"的组合矩阵，不能只靠单元测试 + 简单示例。
4. **Vendor 源码的必要性**：对处于快速迭代期的依赖，vendor + patch 比等 upstream 修复更可控。

---

## 七、附录

### 7.1 相关源码位置

```
# Boa 0.21.1 registry 路径（缓存，`cargo clean` 后需重新 patch）
~/.cargo/registry/src/mirrors.ustc.edu.cn-*/boa_engine-0.21.1/
~/.cargo/registry/src/mirrors.ustc.edu.cn-*/boa_ast-0.21.1/

# 关键文件
boa_ast/src/scope.rs:225            # reorder_binding_indices
boa_ast/src/scope.rs:357            # create_mutable_binding
boa_engine/src/vm/opcode/define/mod.rs:55  # DefInitVar OOB
boa_engine/src/vm/opcode/get/name.rs:25    # GetName OOB
boa_engine/src/environments/runtime/mod.rs:210      # FunctionEnvironment 创建
boa_engine/src/environments/runtime/declarative/mod.rs:310  # PoisonableEnvironment::set
```

### 7.2 复现 Demo

```rust
// 最小复现：5 参数 + 1 个 non-escaping var
let code = r#"
(function(a, b, c, d, e) {
  var x = 42;  // non-escaping var → 触发 DefInitVar OOB
})();
"#;
let source = Source::from_bytes(code);
ctx.eval(source).unwrap();
```

### 7.3 当前补丁

```rust
// PoisonableEnvironment::set — lenient bounds check
pub(crate) fn set(&self, index: u32, value: JsValue) {
    let mut bindings = self.bindings.borrow_mut();
    if (index as usize) < bindings.len() {
        bindings[index as usize] = Some(value);
    }
    // out-of-bounds → non-escaping var → silently skip
}
```

---

## 八、Boa 边界验证结果（2026-06-04）

验证环境：`cha run` + Oolong + Boa 0.21.1（vendored），ESM-only 模式（无 CJS wrapper）。

| # | 任务 | 结果 | 说明 |
|---|------|------|------|
| 1 | ESM 脱类型后正确执行 | ✅ | `cha run app.ts` transpile→eval 全链路通 |
| 2 | Rust 原生函数挂载和调用 | ✅ | `@std/path.join()` 正常工作 |
| 3 | 基础 API 性能 | ⚠️ 慢 | 纯解释器 ~106k `i32` 循环迭代/秒（对比 V8 500M+）。重计算走 Rust |
| 4 | 内存泄漏 | ✅ 一次性场景无问题 | 每个 `cha run` 创建新 Context，进程退出后释放 |
| 5 | `var` bug 是否影响 ESM-only | ✅ 不影响 | 模块内 `var` + 嵌套函数 `var` + 闭包引用全部正常 |
| 6 | Proxy 基础能力 | ✅ | get/set/revocable/prototype 全部正常 |

### 结论：保留 Boa 用于当前阶段

- 对 TS-first 场景（ESM 无 CJS wrapper），Boa 足够稳定
- 性能瓶颈通过 Rust 原生 `@std/*` 模块绕过
- 如果未来需要更高性能的 JS 执行（如热路径代码），可在此层替换为 QuickJS
