# 讨论记录

## 2026-05-28 — 第一次：项目诞生

### 关键决策

1. **为什么不自接用上游 kossjs？**
   - ModuleLoader 必须在 ContextBuilder 构建时注入，外部覆盖不了
   - 需要 TS 即时转译、CJS→ESM、koss 缓存解析 → 必须改 ModuleLoader

2. **为什么标准库放 oolong 不放 cha？**
   - 标准库是引擎的核心能力，嵌入式场景也要用

3. **CJS→ESM 怎么做？**
   - 静态 AST 转译（OXC 解析 + 源码改写）
   - 不做运行时 createRequire 风格

4. **项目命名？**
   - 引擎：oolong（乌龙茶 🍵）
   - CLI：cha（茶 🍵）

5. **为什么从零建仓库而不是原地 rename？**
   - 清晰区分上游和我们
   - 不保留上游 commit history 的包袱

## 2026-05-29 — 第二次：Boa 集成完成

### 已完成

- CJS→ESM 转译器：13 测试
- TS→JS 转译器：11 测试（含 sourcemap）
- 模块解析器：10 测试
- 类型检查：tsgo 外部二进制调用
- ModuleLoader：Boa ModuleLoader trait，管线完整
- Runtime：Context + ModuleLoader + Console，7 e2e 测试
- **41 测试全过，零 clippy 警告**

### 踩坑

- `oxc_transformer 0.133` 在 Rust 1.98 nightly 的 `if let` guard 不兼容 → vendor 补丁
- `Expression::CallExpression` 不是 `Expression::Call`（与旧 OXC 文档不符）
- `BindingPattern` 是枚举不是 struct → 需 match

## 2026-05-29 — 第三次：标准库设计原则

### 标准库风格

- **模块导入**（`import "fs"`），非全局对象（与 Deno/Bun 对齐）
- **异步优先**（`await fs.readFile(path)` 返回 `Promise`）
- **W3C 风格**为第一等公民，`node:` 前缀做兼容
- 三种 import 语法全支持：`import readFile from "fs"` / `import {readFile} from "fs"` / `import * as fs from "fs"`

### 对上游组件（kossjs / boa_runtime）的使用原则

kossjs 当前**很基础不完整**，用之前必须做两件事：
1. **审核上游源码**，确认行为符合我们的需求（格式、颜色、稳定性、测试覆盖）
2. **判断**：适配使用 vs 自己实现
3. 即使使用，也要对齐 OOLONG 代码风格（2 空格、中文注释、测试覆盖）

## 2026-05-29 — 第四次：标准库全线补齐

### 已完成

- **Phase 1-4, 7 全部完成**：path/fs/process/os 四个标准库 + setTimeout/Interval 定时器
- `stdin.read()` / `stdin.readAsBytes()` 异步标准输入
- `readFileSync` 返回 ArrayBuffer（与 Deno 对齐）
- `writeFile` 支持 string | ArrayBuffer | Uint8Array
- os 模块：platform/arch/EOL/hostname/type/release/homedir/tmpdir/totalmem/freemem
- **84 测试全过，零 clippy 警告**

### 踩坑

- Boa 0.21 `JsArrayBuffer::data_mut()` 返回 `Option<GcRefMut<[u8]>>`，必须保持 borrow 活跃直到 copy 完成
- `JsArrayBuffer` `constructor` 私有，只能用 `JsArrayBuffer::new(len, ctx)` 创建空 buffer 再 `data_mut()` 写
- `into_js_function_copied` 要求 `Fn + Copy + 'static`，闭包不能捕获 `&mut Context`，必须作为参数传入

## 2026-05-29 — 第五次：Web API （fetch + Blob + URL）

### 已完成

- **Phase 6 全部完成**：Blob + File + URL + URLSearchParams + TextEncoder/TextDecoder + queueMicrotask + structuredClone + fetch
- Blob：`new Blob(parts)` / `size` / `type` / `text()` / `arrayBuffer()` / `slice()`
- File：`new File(parts, name)` / `name` / `lastModified`（Blob 同级实现）
- URLSearchParams：`get/set/append/delete/has/getAll/sort/toString/forEach/entries/keys/values`
- URL：来自 boa_runtime，支持构造函数 / 属性 / 相对 URL 解析
- TextEncoder + TextDecoder：来自 boa_runtime，utf-8/utf-16 编解码
- fetch：`BlockingReqwestFetcher` HTTP 后端，支持 `Response.text()/json()/bytes()`
- **98 测试全过，零 clippy 警告**

### 踩坑

- Blob 的 `data` 字段需要通过 `borrow().data()`（`Object<T>::data()` 方法）访问，不能直接 `.data` 字段
- `#[boa_class]` 用 `context.register_global_class::<T>()` 注册，不需要手动 module
- `TrustedLen` 编译问题 → 用 `std::iter::TrustedLen` 不可用，改用 `.copied()` 去掉引用

## 2026-05-29 — 第六次：Node 兼容层 Phase 5 规划

### 决策背景

用户指出初始方案（仅 4 个 `node:*` JS 包装模块）不足以支撑目标——Node 兼容层必须能正常运行 webpack/rollup/vite/babel 等 npm 包。

参考 Deno/Bun 的模式：
- `import "path"` → W3C 标准化模块（OOLONG 默认）
- `import "node:path"` → 完整 Node.js API 面
- 两套独立存在，用户按需选择

### 架构方案

```
import "path"          →  W3C 模块（现有）
import "node:path"     →  Node.js 完整 API（新增）
import "node:fs"       →  Node.js 完整 API（含 callback/promises/constants）
import "node:buffer"   →  Buffer 类 + 全局 Buffer
...
```

三个层面：
1. **全局对象** — `process`, `Buffer`, `global`, `setImmediate`, `__dirname`, `__filename`（Boa register_global_property）
2. **CJS 支持** — `require()`, `module`, `exports`（ModuleLoader 中检测 CJS → 函数作用域包装）
3. **`node:*` 模块** — 全量 Node.js 内置模块（Rust 注册 + JS hybrid 实现）

### 分阶段实施

| 阶段 | 内容 | 价值 |
|------|------|------|
| **5.0** | 基础设施：CJS require + 全局 process/Buffer/__dirname + module loader 改造 | 必须先有 |
| **5.1** | `node:path`, `node:os`, `node:process` | 快速验证体系 |
| **5.2** | `node:buffer` (完整 Buffer API) + `node:events` (EventEmitter) | Buffer 是 fs 前置依赖 |
| **5.3** | `node:fs` — callback + sync + promises + constants + Stats + FileHandle | 最核心 |
| **5.4** | `node:util` + `node:stream` + `node:url` | 实用工具层 |
| **5.5** | `node:crypto` + `node:child_process` + `node:module` | 进阶功能 |
| **5.6** | 剩余模块 (assert/tty/vm/zlib/querystring/perf_hooks/timers 等) | 补齐 |

### 当前测试数

- **98 测试全过，零 clippy 警告**
- 确认：Phase 5 之前代码无警告，所有现有模块稳定

## 2026-05-29 — 第七次：标准库哲学定调 + 项目重构

### 决策背景

用户问：`os.cpus()` 这类 Node.js 有但 W3C 没有的 API，OOLONG 的标准库怎么处理？

调研发现：
- **`import "os"` 没有真正的 W3C 标准**——浏览器根本没有 `os` 模块
- **Deno** 三层体系：`Deno.*`（Rust 内置）→ `@std/*`（TS 标准库包）→ `node:*`（Node 兼容）
- **Bun** 两层体系：`Bun.*`（Zig 内置）→ `node:*`（Node 兼容）
- 两者都没有 "W3C `os` 模块" 这个概念——原生模块是自己设计的

### 关键决策

1. **OOLONG 原生层是自定标准**，不是 W3C 标准。API 面参考 Deno/Bun/Node 三家
2. **`cpus()` 等通用 API 最终会加入原生层**（`import "os"`），不只是藏在 `node:os` 里
3. **实现策略**：Phase 5.x 先把 `node:*` 补齐；后续统一对原生层做全量更新
4. **三元设计已确定**：
   - `web/` — W3C Web API 全局类（Blob、URLSearchParams…）
   - `std/` — OOLONG 原生模块（`import "fs"`、`import "os"`…）
   - `node/` — Node 兼容层（`import "node:*"`）

### 项目重构

按三元设计拆分 `src/std/`：

| 移动前 | 移动后 | 说明 |
|--------|--------|------|
| `src/std/blob.rs` | `src/web/blob.rs` | W3C Web API |
| `src/std/url_search_params.rs` | `src/web/url_search_params.rs` | W3C Web API |
| `src/std/{path,fs,process,os}.rs` | 保留在 `src/std/` | OOLONG 原生模块 |

### 后续计划

- **Phase 5.1 立即开始**：实现 `node:path` + `node:os`
- **Phase 5.x 完结后**：对标 Deno/Bun，统一对 `std/` 原生层做全量 API 补充

### 踩坑

- 无
