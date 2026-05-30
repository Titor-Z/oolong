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

### Phase B — `node:*` JS→Rust 迁移

#### 迁移顺序

| 优先级 | 模块 | 当前实现 | 迁移工作量 | nodeCompat 影响 | 说明 |
|--------|------|---------|-----------|----------------|------|
| B.1 | `node:path` | 内联 JS | 中 | 低 | 高频调用，打包工具热路径 |
| B.2 | `node:events` | 内联 JS | 中 | 低 | EventEmitter 生态基石 |
| B.3 | `node:stream` | 内联 JS | 大 | 中 | 和 fs/http 紧密绑定 |
| B.4 | `node:util` | 内联 JS | 中 | 低 | format/inspect 热路径 |
| B.5 | `node:module` | 内联 JS | 小 | 低 | createRequire 关键 |
| B.6 | `node:url` | 内联 JS | 小 | 低 | re-export 全局类 |
| B.7 | `node:assert` | 内联 JS | 小 | 低 | assert 函数集 |
| B.8 | `node:querystring` | 内联 JS | 小 | 低 | parse/stringify |
| B.9 | `node:timers` | 内联 JS | 小 | 低 | re-export 全局 + setImmediate |
| B.10 | `node:vm` | 内联 JS | 中 | 低 | runInThisContext/Script |

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

---

### Phase C — `std/` 原生层扩充

| 模块 | 优先级 | 说明 |
|------|--------|------|
| `std/encoding` | 🔴 高 | base64, hex（Rust 原生） |
| `std/streams` | 🔴 高 | Web Streams 风格读写流 |
| `std/uuid` | 🟡 中 | UUID v4 生成/验证 |
| `std/semver` | 🟡 中 | 语义版本解析/比较 |
| `std/fmt` | 🟡 中 | 格式化（colors, printf） |
| `std/log` | 🟢 低 | 日志框架 |

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
├── web/        W3C 全局类
│   ├── mod.rs
│   ├── event.rs（Event/EventTarget）
│   └── ...
├── std/        OOLONG 原生模块
│   ├── mod.rs
│   ├── fs.rs
│   ├── os.rs
│   ├── path.rs
│   ├── process.rs
│   └── http.rs（Phase A）
└── node/       Node 兼容模块
    ├── mod.rs
    ├── path.rs（Phase B 后全部 Rust）
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

- 当前：**298 测试全过，零 clippy 警告**（38 单元 + 260 集成）
- Phase A 完成后：~320 测试
- Phase B 全部完成后：~400 测试
- 始终 `cargo test && cargo clippy --all-targets && cargo fmt` 通过
