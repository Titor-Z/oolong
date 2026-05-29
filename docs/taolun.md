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

### 待办（下一阶段）

- [ ] Phase 5：Node 兼容层（`node:fs`/`node:path`/`node:process`/`node:os`）
- [ ] 模块解析完善：package.json `exports`/`imports`/`browser` 字段
- [ ] 类型检查集成测试
- [ ] 从 koss 迁移包管理器代码到 cha
