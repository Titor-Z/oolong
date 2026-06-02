use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use boa_engine::module::{ModuleLoader, Referrer, SyntheticModuleInitializer};
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::NativeFunction;
use boa_engine::{
    Context, JsError, JsNativeError, JsResult, JsString, JsValue, Module, Source, js_string,
};
use boa_gc::GcRefCell;
use rustc_hash::FxHashMap;

use crate::resolver::ModuleResolver;

/// 裸名 → Node.js 内置模块名映射
/// 用于路由：有 nodeCompat → 裸名转 node:*，无 nodeCompat → 裸名转 @std/*
const BARE_NODE_MODULES: &[&str] = &[
    "path",
    "process",
    "fs",
    "os",
    "buffer",
    "events",
    "util",
    "stream",
    "url",
    "crypto",
    "child_process",
    "module",
    "assert",
    "timers",
    "tty",
    "perf_hooks",
    "vm",
    "zlib",
    "querystring",
    "http",
    "net",
];

/// 内置模块白名单（不触发 "cha install" 提示）
const BUILTIN_MODULES: &[&str] = &[
    // @std/ — OOLONG 原生模块
    "@std/path",
    "@std/process",
    "@std/fs",
    "@std/os",
    "@std/http",
    "@std/encoding",
    "@std/log",
    "@std/uuid",
    "@std/semver",
    "@std/fmt",
    // node: — Node.js 兼容模块
    "node:path",
    "node:process",
    "node:fs",
    "node:os",
    "node:buffer",
    "node:events",
    "node:util",
    "node:stream",
    "node:url",
    "node:crypto",
    "node:child_process",
    "node:module",
    "node:assert",
    "node:timers",
    "node:tty",
    "node:perf_hooks",
    "node:vm",
    "node:zlib",
    "node:querystring",
    "node:http",
    "node:net",
];

pub struct OolongModuleLoader {
    root: PathBuf,
    resolver: ModuleResolver,
    module_map: GcRefCell<FxHashMap<PathBuf, Module>>,
    /// 内置模块（"@std/fs" / "node:fs" → Module）
    builtins: GcRefCell<HashMap<String, Module>>,
    /// 是否启用 nodeCompat 裸名路由
    node_compat: bool,
}

impl OolongModuleLoader {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self::with_node_compat(root, false)
    }

    pub fn with_node_compat<P: AsRef<Path>>(root: P, node_compat: bool) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            resolver: ModuleResolver::new(),
            module_map: GcRefCell::default(),
            builtins: GcRefCell::default(),
            node_compat,
        }
    }

    pub fn resolver(&self) -> &ModuleResolver {
        &self.resolver
    }

    pub fn insert(&self, path: PathBuf, module: Module) {
        self.module_map.borrow_mut().insert(path, module);
    }

    pub fn get(&self, path: &Path) -> Option<Module> {
        self.module_map.borrow().get(path).cloned()
    }

    /// 注册一个内置模块
    pub fn register_builtin(&self, name: &str, module: Module) {
        self.builtins.borrow_mut().insert(name.to_string(), module);
    }

    /// 获取内置模块
    pub fn get_builtin(&self, name: &str) -> Option<Module> {
        self.builtins.borrow().get(name).cloned()
    }

    fn referrer_file(&self, referrer: &Referrer) -> PathBuf {
        referrer
            .path()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.root.join("__entry__.js"))
    }
}

/// 判断路径是否来自包缓存（应走 CJS IIFE 路径）
fn is_package_cache_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("node_modules") || s.contains(".cha/modules")
}

/// 构建 CJS require 函数
///
/// 返回的 JsValue 是一个 JS Function，可在 CJS IIFE 中作为 `require` 参数使用。
/// 内部捕获 `loader`（用于解析和加载模块）和 `current_file`（用于相对路径解析）。
fn create_cjs_require(
    loader: Rc<OolongModuleLoader>,
    current_file: &Path,
    ctx: &mut Context,
) -> JsValue {
    let current_dir = current_file
        .parent()
        .unwrap_or(Path::new("/"))
        .to_path_buf();

    let f = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let spec = args.first().and_then(|v| v.to_string(ctx).ok()).ok_or_else(
                    || {
                        JsError::from(
                            JsNativeError::typ()
                                .with_message("require() expects a string argument"),
                        )
                    },
                )?;
                let spec = spec.to_std_string_escaped();
                require_inner(&loader, &current_dir, &spec, ctx)
            },
        )
    };

    FunctionObjectBuilder::new(ctx.realm(), f)
        .name("require")
        .length(1)
        .build()
        .into()
}

/// require 内部逻辑：路由 → 内置模块 → 解析 → 缓存 → 递归加载
fn require_inner(
    loader: &Rc<OolongModuleLoader>,
    current_dir: &Path,
    spec: &str,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    // 1. 路由裸名
    let routed = route_bare_specifier(spec, loader.node_compat);

    // 2. 内置模块 → 评估后返回 default export
    if let Some(module) = loader.builtins.borrow().get(&routed).cloned() {
        let _promise = module.load_link_evaluate(ctx);
        let _ = ctx.run_jobs();

        // 优先返回 default export（模块 API 对象）
        if let Ok(default_val) = module.get_value(js_string!("default"), ctx) {
            if !default_val.is_undefined() {
                return Ok(default_val);
            }
        }
        // 回退：返回 namespace 对象
        let ns = module.namespace(ctx);
        return Ok(JsValue::from(ns));
    }

    // 3. 模块解析
    let parent_path = current_dir.join("__require__.js");
    let resolved = loader.resolver.resolve(&routed, &parent_path).map_err(|e| {
        let msg = if is_bare_specifier(&routed) {
            format!("{}\n  Tip: run `cha install {}`", e, routed)
        } else {
            e.to_string()
        };
        JsError::from(JsNativeError::typ().with_message(msg))
    })?;

    // 4. CJS 缓存检查
    if let Some(cached) = crate::cjs::CJS_CACHE.with(|c| c.borrow().get(&resolved).cloned()) {
        return Ok(cached);
    }

    // 5. ESM 模块映射检查（已通过 import 加载）
    if let Some(module) = loader.get(&resolved) {
        let _promise = module.load_link_evaluate(ctx);
        let _ = ctx.run_jobs();
        if let Ok(default_val) = module.get_value(js_string!("default"), ctx) {
            if !default_val.is_undefined() {
                return Ok(default_val);
            }
        }
        let ns = module.namespace(ctx);
        return Ok(JsValue::from(ns));
    }

    // 6. 递归加载 CJS
    let child_require = create_cjs_require(loader.clone(), &resolved, ctx);
    let module_exports = crate::cjs::load_cjs_file(&resolved, child_require, ctx)?;

    // 7. 缓存
    crate::cjs::CJS_CACHE.with(|c| {
        c.borrow_mut().insert(resolved, module_exports.clone());
    });

    Ok(module_exports)
}

impl ModuleLoader for OolongModuleLoader {
    fn load_imported_module(
        self: Rc<Self>,
        referrer: Referrer,
        specifier: JsString,
        context: &RefCell<&mut Context>,
    ) -> impl Future<Output = JsResult<Module>> {
        let result = (|| {
            let raw_spec = specifier.to_std_string_escaped();

            // 裸名路由：nodeCompat → node:*，否则 → @std/*
            let spec = route_bare_specifier(&raw_spec, self.node_compat);

            // 先检查内置模块
            if let Some(module) = self.builtins.borrow().get(&spec).cloned() {
                return Ok(module);
            }

            let parent_path = self.referrer_file(&referrer);

            let resolved = self.resolver.resolve(&spec, &parent_path).map_err(|e| {
                let msg = if is_bare_specifier(&spec) {
                    format!("{}\n  Tip: run `cha install {}`", e, spec)
                } else {
                    e.to_string()
                };
                JsError::from(JsNativeError::typ().with_message(msg))
            })?;

            if let Some(module) = self.get(&resolved) {
                return Ok(module);
            }

            let ext = resolved.extension().and_then(|e| e.to_str()).unwrap_or("");

            // ── CJS 模块（.cjs 扩展名 或 来自包缓存的 .js）──────────────────
            if ext == "cjs" || (ext == "js" && is_package_cache_path(&resolved)) {
                let ctx = &mut *context.borrow_mut();
                let require_fn = create_cjs_require(self.clone(), &resolved, ctx);
                let module_exports = crate::cjs::load_cjs_file(&resolved, require_fn, ctx)?;

                let export_names = &[js_string!("default")];
                let cjs_mod = Module::synthetic(
                    export_names,
                    // SAFETY: The closure captures `module_exports` (JsValue) which is Trace.
                    unsafe {
                        SyntheticModuleInitializer::from_closure(
                            move |m: &boa_engine::module::SyntheticModule,
                                  _export_ctx: &mut Context| {
                                m.set_export(&js_string!("default"), module_exports.clone())?;
                                Ok(())
                            },
                        )
                    },
                    None,
                    None,
                    ctx,
                );

                self.insert(resolved.clone(), cjs_mod.clone());
                return Ok(cjs_mod);
            }

            // ── TS/TSX/MTS 转译 ────────────────────────────────────────────
            let source_bytes = {
                if matches!(ext, "ts" | "tsx" | "mts") {
                    let source_str =
                        String::from_utf8(std::fs::read(&resolved).map_err(|err| {
                            JsError::from(JsNativeError::typ().with_message(format!(
                                "cannot read module '{}': {}",
                                resolved.display(),
                                err
                            )))
                        })?)
                        .map_err(|_| {
                            JsError::from(
                                JsNativeError::typ()
                                    .with_message("invalid UTF-8 in TypeScript file"),
                            )
                        })?;
                    let transpiled =
                        crate::transpiler::transpile(&source_str, &resolved).map_err(|e| {
                            JsError::from(
                                JsNativeError::typ().with_message(format!("transpile error: {e}")),
                            )
                        })?;
                    transpiled.code.into_bytes()
                } else {
                    std::fs::read(&resolved).map_err(|err| {
                        JsError::from(JsNativeError::typ().with_message(format!(
                            "cannot read module '{}': {}",
                            resolved.display(),
                            err
                        )))
                    })?
                }
            };

            // ── CJS→ESM 转换 ────────────────────────────────────────────────
            let source_bytes = {
                let source_str = String::from_utf8(source_bytes).map_err(|_| {
                    JsError::from(JsNativeError::typ().with_message("invalid UTF-8 in source file"))
                })?;
                match crate::cjs_to_esm::transform(&source_str, Some(&resolved)) {
                    Ok(code) => code.into_bytes(),
                    Err(_) => source_str.into_bytes(),
                }
            };

            // ── 解析为 ESM 模块 ──────────────────────────────────────────────
            let source = Source::from_bytes(&source_bytes).with_path(&resolved);

            let module = Module::parse(source, None, &mut context.borrow_mut()).map_err(|err| {
                JsError::from(
                    JsNativeError::syntax()
                        .with_message(format!("could not parse module '{}'", spec))
                        .with_cause(err),
                )
            })?;

            self.insert(resolved, module.clone());
            Ok(module)
        })();

        async { result }
    }
}

/// 路由裸名：有 nodeCompat → node:*，无 nodeCompat → @std/*
fn route_bare_specifier(spec: &str, node_compat: bool) -> String {
    if BARE_NODE_MODULES.contains(&spec) {
        if node_compat {
            format!("node:{}", spec)
        } else {
            format!("@std/{}", spec)
        }
    } else {
        spec.to_string()
    }
}

/// 判断是否为内置模块（含裸名、node:、@std/ 三种形式）
pub fn is_builtin_module(name: &str) -> bool {
    BUILTIN_MODULES.contains(&name) || BARE_NODE_MODULES.contains(&name)
}

fn is_bare_specifier(spec: &str) -> bool {
    !spec.starts_with("./")
        && !spec.starts_with("../")
        && !spec.starts_with('/')
        && !spec.starts_with("node:")
        && !spec.starts_with("file:")
        && !spec.starts_with("@std/")
        && !BUILTIN_MODULES.contains(&spec)
}
