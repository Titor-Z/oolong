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

## 2026-05-30 — 第九次：架构重构 v2 + 长期路线图

### 决策背景

Phase 5（Node 兼容层）全部完成后，重新审视三元标准库体系的设计方向和实现策略。

### 关键决策

1. **全栈 Rust 实现**：
   - `node:*` 中当前 10 个内联 JS 模块（path/events/assert/querystring/timers/vm/url/stream/util/module）全部迁移为 Rust 纯代码实现
   - 零桥接开销，消除 JS↔Rust 类型边界

2. **W3C 类型为一等公民**：
   - `std/`（OOLONG 原生模块）— 全 Rust + W3C 类型 + 自定 API 设计
   - `node:*`（Node 兼容层）— 全 Rust + W3C 类型内核 + Node.js 形状暴露
   - `web/`（全局类）— 已有 W3C 实现
   - W3C 类型六条硬规则：Uint8Array 二进制、DOMHighResTimeStamp 时间戳、Promise<T> 异步、AbortSignal 取消、USVString 字符串、W3C 标准错误类型

3. **`nodeCompat` 配置机制**：
   - `oolong.json` 中有 `nodeCompat` → 裸名路由到 `node:*`
   - `oolong.json` 中无 `nodeCompat` → 裸名路由到 `@std/*`
   - `node:*` 内部 Rust 实现用 W3C 类型，对外暴露时按 nodeCompat 值转换
   - 不需要 `"w3c"` 值——W3C 类型是所有模式的内核基线
   - `std/` 完全不受 `nodeCompat` 约束，始终 W3C
   - 路由逻辑全部在 `module_loader.rs`，未来改路由只改这一个文件

4. **三元标准库最终架构**：
   ```
   std/     → OOLONG 原生（全 Rust + W3C 类型 + 自定 API 设计）
   node:*   → Node 兼容（全 Rust + W3C 类型内核 + nodeCompat 适配输出）
   web/     → W3C 全局类（EventTarget, Blob, fetch…）
   ```
   - 三套独立的 Rust 实现，没有包装/继承关系
   - 共用 W3C 类型约定，各自设计 API 形状

5. **远期目标（Phase 9+）**：
   - "Write OOLONG code, deploy to Node.js" — 利用 `nodeCompat` 适配层 + ModuleLoader import 重写 + polyfill 注入，将 OOLONG 代码编译到 Node.js 运行
   - 暂不实施，仅做架构预留

6. **三元标准库文件结构最终确认**：
   ```
   src/
   ├── web/        W3C 全局类（EventTarget, Blob, fetch…）
   ├── std/        OOLONG 原生模块（import "fs"）
   └── node/       Node 兼容模块（import "node:fs"）
   ```
   无 `runtime/` 共享内核层——每个模块完全独立 Rust 实现

### 后续计划

- Phase A: `std/http` HTTP server（第一个）
- Phase B: 10 个 `node:*` JS→Rust 迁移
- Phase C: `std/` 原生层扩充（encoding, streams, uuid, semver, fmt, log）
- Phase D: `std/fs` 增强（ensureDir, walk, expandGlob）
- Phase E: Compile OOLONG → Node.js（远期）

### 测试策略

- 每个模块 Rust 单元测试 + 集成 e2e 测试
- `nodeCompat` 多版本场景测试
- `cargo test && cargo clippy --all-targets && cargo fmt`

## 2026-06-02 — 第？次：module_loader 设计定调 + 三项目架构确认

### 决策背景

讨论了 CJS require 修复、module_loader 是否需要改、以及 oolong/cha/koss 三者的关系。

### 关键决策

1. **三项目架构确认**：
   - **koss**（`/Users/titor/koss/`）— 包管理器前身，代码将来搬入 `~/cha/`
   - **oolong**（`/Users/titor/oolong/`）— 运行时引擎，作为 Rust library 被 cha 依赖
   - **cha**（`/Users/titor/cha/`）— 统一 CLI 二进制，用户只装这一个

2. **oolong 保留独立二进制**：
   - 同时产出 `liboolong`（库）和 `oolong`（二进制）
   - `oolong run/eval/repl` 保留，供团队内部调试使用
   - `cha run` 调用 `oolong::OolongRuntime` 做同一件事

3. **module_loader 设计：两条加载路径**：
   - **CJS IIFE 路径**：`.cjs` 文件 + 来自 `~/.cha/modules/` 或 `node_modules/` 的 `.js` 文件
     - 通过 `load_cjs_file` 包装 IIFE，`require` 运行时可用
     - 默认 require 函数修复：内置模块→`Module::get_namespace`，外部包→`ModuleResolver` 递归加载
   - **CJS→ESM 转译路径**：用户源代码（不来自包缓存的 `.js`/`.mjs`）
     - 静态 AST 分析，转换 `require` 为 `import`
   - **原因**：不是所有 CJS 模式都能被静态转译（动态 require、条件 require、try-catch），npm 包必须走 IIFE 路径。

4. **下一步规划**：
   - 阶段一：oolong 收尾—修复 `cjs/mod.rs` 默认 require、module_loader 区分路径、lib.rs 导出 API
   - 阶段二：`~/cha/` CLI 工程—搬 koss 核心模块、实现 `cha install/run/eval/repl`
   - 阶段三：E2E 验证—`cha install express && cha run app.mjs`

### 技术要点

- `module_loader.rs` 本身不需要大改，只需新增一条判断规则（是否来自包缓存）
- 唯一的真正改动在 `cjs/mod.rs:64-83` 的默认 require 函数
- `route_bare_specifier` 逻辑不变：裸名路由到 `node:*` 或 `@std/*`
- resolver 的查找顺序不变：cha cache → node_modules

### 踩坑

- `JsObject` 不是 `Send`，不能放在 `Arc<Mutex<>>` 中，但 `thread_local!` 可以存 `HashMap<PathBuf, JsValue>`
- boa 的 `Module::get_namespace(&self, ctx)` 可以同步获取内置模块的导出对象
- 注意：`load_cjs_file` 的默认 require 使用了 ModuleResolver，每次调用创建新 resolver（stateless，可行但略低效，后续可缓存）
