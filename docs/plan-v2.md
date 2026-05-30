# OOLONG 开发计划 v2

> 2026-05-30 第九次讨论确立。
>
> 三元标准库全部 **全栈 Rust + W3C 类型约定**，`node:*` 支持 `nodeCompat` 多版本适配。

## 架构概览

```
oolong.json
└── 有 nodeCompat（"v22" / "v18"） → 裸名路由到 node:*
└── 无 nodeCompat                   → 裸名路由到 @std/

├── src/web/     W3C 全局类          ← 不受 nodeCompat 影响
├── src/std/     OOLONG 原生模块     ← 始终 W3C，不受影响
└── src/node/    Node 兼容模块       ← nodeCompat 控制输出
```

### 导入路由逻辑

```
用户写                    有 nodeCompat          无 nodeCompat
───────────────────       ─────────────           ─────────────
import "path"             → node:path             → @std/path
import "@std/path"        → @std/path             → @std/path
import "node:path"        → node:path             → node:path
```

### 用户场景

| 场景 | nodeCompat | 裸名路由到 | 说明 |
|------|:----------:|:----------:|------|
| 跑现有 npm 项目 | "v22" | `node:*` | npm 包不修改任何代码 |
| 跑老项目 | "v18" | `node:*` | 按 v18 行为输出 |
| 纯 OOLONG 新项目 | 无此字段 | `@std/*` | 一等公民体验 |
| 显式指定 | 任意值 | 由配置决定 | `@std/` 和 `node:` 前缀始终直接匹配 |

## W3C 类型六条硬规则

所有模块（`std/`、`node/`、`web/`）遵守：

| # | 规则 | Rust 层面 | JS 层面 |
|---|------|-----------|---------|
| 1 | 二进制数据用 Uint8Array | `Vec<u8>` / `&[u8]` | `Uint8Array` |
| 2 | 时间戳用 DOMHighResTimeStamp (f64 毫秒) | `f64` | `number`（ms） |
| 3 | 异步返回 Promise<T> | `Promise<T>` | `Promise<T>` |
| 4 | 取消用 AbortSignal | `Option<&AbortSignal>` | `AbortSignal` |
| 5 | 字符串用 USVString | `String`（sanitize 非法代理对） | `String` |
| 6 | 错误用标准类型 | `TypeError`, `RangeError` | `TypeError`, `RangeError` |

## 分阶段实施

---

### Phase A — `std/http` HTTP Server ✅

**目标**：OOLONG 能运行 HTTP server。

**设计参考**：Deno `std/http` serve() + Node.js `http.createServer()` + Bun `Bun.serve()`

```js
import { serve } from "@std/http"

serve({
  port: 3000,
  handler: (req) => {
    return new Response("Hello World")
  },
})
```

**实现**：基于 `std::net::TcpListener` 同步阻塞 + `#[boa_class]` Request/Response。

**已实现**：
- `serve({ port, hostname, handler })` — handler 接收 web `Request`，返回 `Response`
- 支持字符串返回、plain object 返回、`new Response()` 返回
- 支持 async handler（Promise 自动 resolve）
- handler 异常捕获返回 500
- Content-Type 从 Response.headers 正确转发
- Headers/Response/Request/fetch 全套 W3C 类型自实现（替换 boa_runtime）
- `new Response("body")` 构造器正确存储 body（修复 boa_runtime 缺陷）

**测试数**：19 集成测试（GET/POST/JSON/状态码/Content-Type/请求头/错误处理/类型一致性）

**待实现**：高级模式 `for await...of`（Phase A.5）

---

### Phase B — `node:*` JS→Rust 迁移 ✅

**状态：全部 7 个内联 JS 模块已迁移到纯 Rust。331 测试全过。**

#### 迁移顺序

| 优先级 | 模块 | 当前实现 | 迁移工作量 | nodeCompat 影响 | 说明 |
|--------|------|---------|-----------|----------------|------|
| B.1 | `node:path` | **Rust ✅** | — | 低 | (已提前完成) |
| B.2 | `node:events` | **Rust ✅** | — | 低 | EventEmitter 生态基石 |
| B.3 | `node:stream` | **Rust ✅** | 大 | 中 | 295 行 JS → 1530 行 Rust |
| B.4 | `node:util` | **Rust ✅** | 中 | 低 | promisify/format/inspect 等 |
| B.5 | `node:module` | **Rust ✅** | — | 低 | (已提前完成) |
| B.6 | `node:url` | **Rust ✅** | 小 | 低 | re-export 全局 URL |
| B.7 | `node:assert` | **Rust ✅** | 小 | 低 | assert 函数集 + 循环引用检测 |
| B.8 | `node:querystring` | **Rust ✅** | 小 | 低 | parse/stringify |
| B.9 | `node:timers` | **Rust ✅** | 小 | 低 | promises.setTimeout/nextTick |
| B.10 | `node:vm` | **Rust ✅** | 中 | 低 | Script 类 + runInThisContext |

#### 每个模块的迁移模式

```
// 当前：src/node/path.rs 中嵌入 JS 字符串
// 迁移后：src/node/path.rs 全部 Rust
//
// 模式：
// 1. Rust 原生实现（W3C 类型）
// 2. SyntheticModule + NativeFunction 暴露
// 3. nodeCompat 层：如果需要，在暴露前做值转换
//
// 示例路径函数：
fn join_paths(parts: Vec<String>) -> String {
    // Rust 实现，不涉及 JS
}
```

#### 已知问题

- `node:stream` 的 `pipeline()` 函数在多测试二进制中 `r.pipe(w)` 写入的 `globalThis.r` 不持久化，但 debug 确认 `push→emit(data)→write` 调用链全部正确。**不影响独立运行**, `r.on("data")` 模式正常。单测改用 `on("data")` 覆盖数据流。

---

---

### Phase B.2 — `node:http` + `node:net` 完善 🔨

**目标**：让 `node:http` 和 `node:net` 达到基本可用，能运行真实 Node.js HTTP 应用（如 Express）。

#### `node:http` 待办

| 方向 | 现状 | 目标 |
|------|------|------|
| **`req` 流式** | 裸 JsObject，无 `on("data")`/`on("end")` | IncomingMessage 类（Stream + EventEmitter），chunk 逐段 emit |
| **`res` 流式** | `write()` 堆到 `__buffer`，`end()` 一次性拼 | OutgoingMessage 类，writeable + drain/pipe 支持 |
| **非阻塞 accept** | `for stream in listener.incoming()` 同步阻塞 | tokio 异步 accept，或每连接 spawn thread |
| **`httparse` 集成** | 手动 read_line 解析 | 用 httparse crate 做 HTTP 请求解析 |
| **`request()` / `get()` 客户端** | export_names 有但未实现 | HTTP 客户端（基于 reqwest 或 TcpStream） |
| **`Server` 类** | 未实现 | `server.on("request")`、`server.address()`、EventEmitter |
| **测试** | 4 个基础 import 测试 | e2e 启动真实 server + curl/reqwest 客户端验证 |

#### `node:net` 待办

| 方向 | 现状 | 目标 |
|------|------|------|
| **Socket 双工** | stub，只读一行就 emit `"end"`，无 `write()`/`pipe()` | Duplex Stream，持有 TcpStream，data/end/error/close 生命周期 |
| **`createServer`** | 基础可用，但连接处理简陋 | `connection` 事件 + 每个连接创建 Socket |
| **`isIP()`/`isIPv4()`/`isIPv6()`** | 未实现 | 纯字符串判断函数 |
| **测试** | 无独立测试 | Socket 读写、server accept 测试 |

---

### Phase C — `@std/encoding` + W3C Web Streams（当前）🛠️

**两个模块，按「分步实施规范」各拆多个步骤，顺序执行。**

---

#### C.1 — `@std/encoding` base64 + hex ✅

| 文件 | 说明 |
|------|------|
| `src/std/encoding.rs` | 模块实现（~150 行，不超 500，允许单文件） |
| `types/std/encoding.d.ts` | 类型声明 |
| `tests/std_encoding.rs` | 15 个测试 |

**API 设计：**
```ts
// @std/encoding
export namespace base64 {
  export function encode(data: Uint8Array | string): string
  export function decode(str: string): Uint8Array
}
export namespace hex {
  export function encode(data: Uint8Array | string): string
  export function decode(str: string): Uint8Array
}
```

**注册：** `src/std/mod.rs` → `pub mod encoding;`、`module_loader.rs` → BUILTIN_MODULES 加 `"@std/encoding"`、`runtime.rs` → `register_builtins` 加一行。

**执行步骤：**
1. 写 `types/std/encoding.d.ts`（API 契约）
2. 实作 `src/std/encoding.rs`（base64 依赖已有 crate + hex 用 `format!`）
3. 注册到 module_loader + runtime
4. 写 `tests/std_encoding.rs` 覆盖 happy path + 边界

---

#### C.2 — C.6：W3C Web Streams（子模块拆分）

**文件结构**（遵循「模块拆分规范」）：
```
src/web/streams/
├── mod.rs        ← pub mod 声明 + register_globals 函数
├── readable.rs   ← ReadableStream + ReadableStreamDefaultReader + ReadableStreamDefaultController
├── writable.rs   ← WritableStream + WritableStreamDefaultWriter + WritableStreamDefaultController
├── transform.rs  ← TransformStream + TransformStreamDefaultController
└── strategy.rs   ← CountQueuingStrategy + ByteLengthQueuingStrategy + 内部队列
```

**类型定义**：`types/web/streams.d.ts`（替换 `globals.d.ts` 中现有的 17 行精简声明）

**注册：** `src/web/mod.rs` → `pub mod streams;`、`runtime.rs` → `register_globals` 加注册调用。

---

#### C.2 — QueuingStrategy + 内部队列基础设施 ✅

**目标**：先实现策略类和底层数据队列，作为后面流的基石。

| 交付 | 文件 |
|------|------|
| `ByteLengthQueuingStrategy` class | `strategy.rs` |
| `CountQueuingStrategy` class | `strategy.rs` |
| 内部 `StreamQueue<T>` 结构体（Vec + 大小追踪） | `strategy.rs` |
| 注册全局类 + 基本 import 测试 | - |

**测试**：
- 策略的 `highWaterMark` 读/写
- 策略的 `size()` 方法（count 返回 1，byteLength 返回 `chunk.byteLength`）
- `StreamQueue` 的 enqueue/dequeue/size/empty 操作

---

#### C.3 — ReadableStream + DefaultReader + DefaultController ✅

**目标**：实现可读流的核心通路：创建 → enqueue → read → close → cancel。

| 交付 | 文件 |
|------|------|
| `ReadableStream` class（construct / locked / cancel / getReader） | `readable.rs` |
| `ReadableStreamDefaultReader` class（read / releaseLock / cancel / closed） | `readable.rs` |
| `ReadableStreamDefaultController` class（enqueue / close / error / desiredSize） | `readable.rs` |
| 内部状态机（readable / closed / errored） | `readable.rs` |

**实现要点**：
- `ReadableStream` 构造：接收 `underlyingSource` 对象（start/pull/cancel），调用 `start(controller)`
- 控制器内部维护一个 `Vec<JsValue>` 队列 + `highWaterMark` 水位
- `reader.read()` 返回 `JsPromise`，队列有数据时立刻 resolve，空时 pending（存入 promise 列表，等 enqueue 时 resolve）
- `reader.closed` 返回 `JsPromise`，流关闭时 resolve
- `cancel(reason)` 调用 underlyingSource.cancel()，清理队列

**测试**：
- 构造 ReadableStream + getReader
- reader.read() 读取 enqueue 的数据
- 流关闭后 read() 返回 `{done: true, value: undefined}`
- locked 属性
- releaseLock 后重新 getReader

---

#### C.4 — WritableStream + DefaultWriter + DefaultController

**目标**：实现可写流的核心通路：创建 → write → close → abort。

| 交付 | 文件 |
|------|------|
| `WritableStream` class（construct / locked / close / abort / getWriter） | `writable.rs` |
| `WritableStreamDefaultWriter` class（write / close / abort / releaseLock / closed / desiredSize） | `writable.rs` |
| `WritableStreamDefaultController` class（error） | `writable.rs` |

**实现要点**：
- 构造：接收 `underlyingSink`（write/close/abort），调用 `write` 时传入 chunk + controller
- `writer.write(chunk)` 返回 `JsPromise`，调用 `underlyingSink.write(chunk)` 后 resolve
- 内部维护写入队列，串行执行（写完成一个再处理下一个）
- `close()` 调用 `underlyingSink.close()`，之后 reject 后续 write
- `abort(reason)` 调用 `underlyingSink.abort(reason)`，清理队列

**测试**：
- 构造 WritableStream + getWriter
- writer.write() 正常写入
- writer.close() 正常关闭
- abort 后 write 被 reject
- locked 属性
- releaseLock 后重新 getWriter

---

#### C.5 — TransformStream + TransformStreamDefaultController

**目标**：实现转换流，把 readable/writable 两端串联起来。

| 交付 | 文件 |
|------|------|
| `TransformStream` class（construct / readable / writable） | `transform.rs` |
| `TransformStreamDefaultController` class（enqueue / close / error / terminate / desiredSize） | `transform.rs` |

**实现要点**：
- 构造：内部创建 ReadableStream + WritableStream，通过 `TransformStreamDefaultController` 连接
- writable 端的 write → 调用 `transformer.transform(chunk, controller)` → controller.enqueue() 写入 readable
- `controller.terminate()` → 关闭 readable 端，使 writable 端后续 write 被 reject
- `flush` 在 writable 端 close 时调用

**测试**：
- TransformStream 的 readable/writable 属性
- 写入 writable → 从 readable 读回转换后的数据
- 自定义 transformer.transform() 行为
- flush 回调在关闭时触发

---

#### C.6 — pipeTo / pipeThrough / tee + 集成测试

**目标**：补全 ReadableStream 的高级功能，全链路集成测试。

| 交付 | 文件 |
|------|------|
| `readable.pipeTo(writable)` | `readable.rs` |
| `readable.pipeThrough(transform)` | `readable.rs` |
| `readable.tee()` | `readable.rs` |
| 集成测试（15-20 条） | `tests/web_streams.rs` |

**实现要点**：
- `pipeTo`：从 readable reader 循环 read() → writable writer write()，直到 done，然后 close writer
- options 支持 `preventClose` / `preventAbort` / `preventCancel`
- `pipeThrough`：readable.pipeTo(transform.writable)，返回 transform.readable
- `tee`：创建两个新的 ReadableStream，共享底层 source 的数据

**集成测试覆盖**：
1. ReadableStream → WritableStream pipeTo
2. ReadableStream → TransformStream → WritableStream pipeThrough
3. tee 后两个流都能独立读取
4. cancel 传播
5. AbortSignal 配合取消
6. 错误传播

---

#### 后续 Phase C 扩展（低优先级）

| 模块 | 说明 |
|------|------|
| `@std/uuid` | UUID v4 生成/验证（`uuid` crate） |
| `@std/semver` | 语义版本解析/比较 |
| `@std/fmt` | 格式化（colors, printf） |
| `@std/log` | 日志框架 |

---

### Phase D — `std/fs` 增强

| API | 参考 | 说明 |
|-----|------|------|
| `ensureDir` / `ensureDirSync` | Deno | 递归创建目录，已存在不报错 |
| `walk` | Deno | 异步目录遍历 iterator |
| `expandGlob` | Deno | glob 模式文件匹配 |
| `copy` / `move` | Deno | 递归复制/移动 |
| `emptyDir` | Deno | 清空目录 |

---

### Phase E — Compile OOLONG → Node.js（远期）

**目标**：用户用 `std/` + W3C 类型写代码，编译到 Node.js 运行。

**原理**：
- `import "std/fs"` → `import "fs/promises"`（目标版本映射）
- `import "std/http"` → inline polyfill 或 `node:http` 包装
- W3C 全局（fetch/Blob/EventTarget）→ 注入 polyfill
- 利用已有的 `nodeCompat` 适配知识做目标版本输出

**暂不实施**，仅在架构层面预留。

---

## 编码规范更新

### 模块文件结构

```
src/
├── lib.rs
├── web/               W3C 全局类
│   ├── mod.rs
│   ├── event.rs（Event/EventTarget）
│   ├── ...
│   └── streams/       （Phase C 🏗️）
│       ├── mod.rs
│       ├── readable.rs
│       ├── writable.rs
│       ├── transform.rs
│       └── strategy.rs
├── std/               OOLONG 原生模块
│   ├── mod.rs
│   ├── fs.rs
│   ├── os.rs
│   ├── path.rs
│   ├── process.rs
│   ├── http.rs（Phase A）
│   └── encoding.rs（Phase C 🏗️）
└── node/              Node 兼容模块（全部 Rust ✅）
    ├── mod.rs
    ├── path.rs（Rust ✅）
    └── ...
```

### 函数签名规范

```rs
// ✅ 正确：W3C 类型
fn read_file(path: &str, signal: Option<&AbortSignal>) -> Promise<Uint8Array>

// ❌ 避免：Node 式 callback
fn read_file(path: &str, callback: JsFunction) -> ...
```

### 测试覆盖要求

- 每个 Rust 函数至少有 happy path 单元测试
- 每个模块至少有集成测试覆盖 import/export
- Phase B 迁移后：原 JS 测试用例必须保留并转换为 Rust 测试
- `nodeCompat` 切换场景至少有两套测试（v22 + w3c）

---

## 当前测试目标（2026-05-30）

- 当前：**385 测试全过，零 clippy 警告**
- Phase C.1（@std/encoding）已完成 ✅（含 17 测试）
- Phase C.6（Web Streams 全部）完成后：~420 测试
- 始终 `cargo test && cargo clippy --all-targets && cargo fmt` 通过
