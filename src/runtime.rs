use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;

use boa_engine::context::ContextBuilder;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsError, JsValue, Module, Source};
use boa_runtime::Console;

use crate::module_loader::OolongModuleLoader;

/// CLI 参数缓存，由二进制入口设置，用于 process.argv
static CLI_ARGS: OnceLock<Vec<String>> = OnceLock::new();

/// 设置 CLI 参数（仅在二进制模式调用）
pub fn set_cli_args(args: Vec<String>) {
    let _ = CLI_ARGS.set(args);
}

/// 获取当前进程参数，优先使用 set_cli_args 设置的值
pub fn get_argv() -> Vec<String> {
    CLI_ARGS
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::args().collect())
}

/// OOLONG 运行时 — 一个 JS/TS 虚拟机实例
pub struct OolongRuntime {
    pub context: Context,
    pub loader: Rc<OolongModuleLoader>,
}

impl OolongRuntime {
    /// 创建新的运行时，绑定模块加载器
    pub fn new(root: &Path) -> Result<Self, String> {
        let loader = Rc::new(OolongModuleLoader::new(root));
        let context = ContextBuilder::default()
            .module_loader(loader.clone())
            .build()
            .map_err(|e| format!("创建 Boa Context 失败: {e}"))?;
        let mut rt = Self { context, loader };
        rt.register_console();
        rt.register_timers();
        rt.register_web_apis();
        rt.register_node_globals();
        rt.register_builtins();
        Ok(rt)
    }

    /// 注册 console 对象
    fn register_console(&mut self) {
        let console = Console::init(&mut self.context);
        let _ = self.context.register_global_property(
            boa_engine::js_string!("console"),
            console,
            Attribute::all(),
        );
    }

    /// 注册 Node.js 全局对象（process / Buffer / global / setImmediate）
    fn register_node_globals(&mut self) {
        // Buffer 全局类
        crate::node::buffer::register_buffer_global(&mut self.context)
            .expect("注册 Buffer 全局失败");

        // global — globalThis 别名
        let global_this = self.context.global_object().clone();
        let _ = self.context.register_global_property(
            boa_engine::js_string!("global"),
            global_this,
            boa_engine::property::Attribute::all(),
        );

        // setImmediate / clearImmediate
        let set_immediate_fn: boa_engine::NativeFunction = {
            use boa_engine::IntoJsFunctionCopied;
            (|cb: JsValue, ctx: &mut Context| -> boa_engine::JsResult<JsValue> {
                if let Some(obj) = cb.as_object() {
                    let func = boa_engine::object::builtins::JsFunction::from_object(obj.clone());
                    if let Some(f) = func {
                        let realm = ctx.realm().clone();
                        ctx.enqueue_job(
                            boa_engine::job::GenericJob::new(
                                move |job_ctx| {
                                    let _ = f.call(&JsValue::undefined(), &[], job_ctx);
                                    Ok(JsValue::undefined())
                                },
                                realm,
                            )
                            .into(),
                        );
                    }
                }
                Ok(JsValue::undefined())
            })
            .into_js_function_copied(&mut self.context)
        };
        let set_immediate =
            boa_engine::object::FunctionObjectBuilder::new(self.context.realm(), set_immediate_fn)
                .name(boa_engine::js_string!("setImmediate"))
                .length(1)
                .build();
        let _ = self.context.register_global_property(
            boa_engine::js_string!("setImmediate"),
            set_immediate,
            boa_engine::property::Attribute::all(),
        );

        let clear_immediate_fn: boa_engine::NativeFunction = {
            use boa_engine::IntoJsFunctionCopied;
            (|_: &mut Context| -> boa_engine::JsResult<JsValue> { Ok(JsValue::undefined()) })
                .into_js_function_copied(&mut self.context)
        };
        let clear_immediate = boa_engine::object::FunctionObjectBuilder::new(
            self.context.realm(),
            clear_immediate_fn,
        )
        .name(boa_engine::js_string!("clearImmediate"))
        .length(1)
        .build();
        let _ = self.context.register_global_property(
            boa_engine::js_string!("clearImmediate"),
            clear_immediate,
            boa_engine::property::Attribute::all(),
        );
    }

    /// 注册内置模块（import "path" / import "process" 等）
    fn register_builtins(&mut self) {
        let path_mod =
            crate::std::path::create_path_module(&mut self.context).expect("创建 path 模块失败");
        self.loader.register_builtin("path", path_mod);

        let process_mod = crate::std::process::create_process_module(&mut self.context)
            .expect("创建 process 模块失败");
        self.loader.register_builtin("process", process_mod);

        let fs_mod = crate::std::fs::create_fs_module(&mut self.context).expect("创建 fs 模块失败");
        self.loader.register_builtin("fs", fs_mod);

        let os_mod = crate::std::os::create_os_module(&mut self.context).expect("创建 os 模块失败");
        self.loader.register_builtin("os", os_mod);

        // ── Node.js 兼容模块 ─────────────────────────────────────────────
        let node_process_mod = crate::node::process::create_node_process_module(&mut self.context)
            .expect("创建 node:process 模块失败");
        self.loader
            .register_builtin("node:process", node_process_mod);

        let node_buffer_mod = crate::node::buffer::create_node_buffer_module(&mut self.context)
            .expect("创建 node:buffer 模块失败");
        self.loader.register_builtin("node:buffer", node_buffer_mod);

        let node_path_mod = crate::node::path::create_node_path_module(&mut self.context)
            .expect("创建 node:path 模块失败");
        self.loader.register_builtin("node:path", node_path_mod);

        let node_os_mod = crate::node::os::create_node_os_module(&mut self.context)
            .expect("创建 node:os 模块失败");
        self.loader.register_builtin("node:os", node_os_mod);

        let node_events_mod = crate::node::events::create_node_events_module(&mut self.context)
            .expect("创建 node:events 模块失败");
        self.loader.register_builtin("node:events", node_events_mod);

        let node_fs_mod = crate::node::fs::create_node_fs_module(&mut self.context)
            .expect("创建 node:fs 模块失败");
        self.loader.register_builtin("node:fs", node_fs_mod);

        let node_util_mod = crate::node::util::create_node_util_module(&mut self.context)
            .expect("创建 node:util 模块失败");
        self.loader.register_builtin("node:util", node_util_mod);

        let node_stream_mod = crate::node::stream::create_node_stream_module(&mut self.context)
            .expect("创建 node:stream 模块失败");
        self.loader.register_builtin("node:stream", node_stream_mod);

        let node_url_mod = crate::node::url::create_node_url_module(&mut self.context)
            .expect("创建 node:url 模块失败");
        self.loader.register_builtin("node:url", node_url_mod);

        let node_crypto_mod = crate::node::crypto::create_node_crypto_module(&mut self.context)
            .expect("创建 node:crypto 模块失败");
        self.loader.register_builtin("node:crypto", node_crypto_mod);

        let node_child_process_mod =
            crate::node::child_process::create_node_child_process_module(&mut self.context)
                .expect("创建 node:child_process 模块失败");
        self.loader
            .register_builtin("node:child_process", node_child_process_mod);

        let node_module_mod = crate::node::module::create_node_module_module(&mut self.context)
            .expect("创建 node:module 模块失败");
        self.loader.register_builtin("node:module", node_module_mod);

        let node_querystring_mod =
            crate::node::querystring::create_node_querystring_module(&mut self.context)
                .expect("创建 node:querystring 模块失败");
        self.loader
            .register_builtin("node:querystring", node_querystring_mod);

        let node_assert_mod = crate::node::assert::create_node_assert_module(&mut self.context)
            .expect("创建 node:assert 模块失败");
        self.loader.register_builtin("node:assert", node_assert_mod);

        let node_timers_mod = crate::node::timers::create_node_timers_module(&mut self.context)
            .expect("创建 node:timers 模块失败");
        self.loader.register_builtin("node:timers", node_timers_mod);

        let node_tty_mod = crate::node::tty::create_node_tty_module(&mut self.context)
            .expect("创建 node:tty 模块失败");
        self.loader.register_builtin("node:tty", node_tty_mod);

        let node_perf_hooks_mod =
            crate::node::perf_hooks::create_node_perf_hooks_module(&mut self.context)
                .expect("创建 node:perf_hooks 模块失败");
        self.loader
            .register_builtin("node:perf_hooks", node_perf_hooks_mod);

        let node_vm_mod = crate::node::vm::create_node_vm_module(&mut self.context)
            .expect("创建 node:vm 模块失败");
        self.loader.register_builtin("node:vm", node_vm_mod);

        let node_zlib_mod = crate::node::zlib::create_node_zlib_module(&mut self.context)
            .expect("创建 node:zlib 模块失败");
        self.loader.register_builtin("node:zlib", node_zlib_mod);
    }

    /// 注册 setTimeout/setInterval/clearTimeout/clearInterval
    fn register_timers(&mut self) {
        boa_runtime::interval::register(&mut self.context).expect("注册 timers 失败");
    }

    /// 注册 Web API 全局对象（Blob / File / URL / TextEncoder / fetch 等）
    fn register_web_apis(&mut self) {
        // Blob + File
        crate::web::blob::register_globals(&mut self.context).expect("注册 Blob/File 失败");

        // URLSearchParams
        crate::web::url_search_params::register_globals(&mut self.context)
            .expect("注册 URLSearchParams 失败");

        // URL (来自 boa_runtime)
        boa_runtime::url::Url::register(None, &mut self.context).expect("注册 URL 失败");

        // TextEncoder + TextDecoder
        boa_runtime::text::register(None, &mut self.context)
            .expect("注册 TextEncoder/TextDecoder 失败");

        // queueMicrotask
        boa_runtime::microtask::register(None, &mut self.context)
            .expect("注册 queueMicrotask 失败");

        // structuredClone
        boa_runtime::clone::register(None, &mut self.context).expect("注册 structuredClone 失败");

        // fetch + Request + Response + Headers
        let fetcher = boa_runtime::fetch::BlockingReqwestFetcher::default();
        boa_runtime::fetch::register(fetcher, None, &mut self.context).expect("注册 fetch 失败");
    }

    /// 执行 JS 脚本（非模块模式，不支持 import）
    pub fn eval_script(&mut self, code: &str) -> Result<String, String> {
        let source = Source::from_bytes(code.as_bytes());
        match self.context.eval(source) {
            Ok(val) => Ok(js_value_to_string(&val, &mut self.context)),
            Err(err) => Err(js_error_to_string(&err, &mut self.context)),
        }
    }

    /// 从字符串执行 ES Module（支持 import/export）
    pub fn eval_module_str(
        &mut self,
        code: &str,
        path_hint: Option<&Path>,
    ) -> Result<String, String> {
        let source = match path_hint {
            Some(p) => Source::from_bytes(code.as_bytes()).with_path(p),
            None => Source::from_bytes(code.as_bytes()),
        };
        let module = Module::parse(source, None, &mut self.context)
            .map_err(|e| format!("parse error: {}", js_error_to_string(&e, &mut self.context)))?;
        let promise = module.load_link_evaluate(&mut self.context);
        let _ = self.context.run_jobs();
        Ok(match promise.state() {
            boa_engine::builtins::promise::PromiseState::Fulfilled(val) => {
                js_value_to_string(&val, &mut self.context)
            }
            boa_engine::builtins::promise::PromiseState::Rejected(err) => {
                return Err(format!(
                    "module error: {}",
                    js_value_to_string(&err, &mut self.context)
                ));
            }
            boa_engine::builtins::promise::PromiseState::Pending => {
                return Err("module evaluation pending".to_string());
            }
        })
    }

    /// 从文件执行 ES Module（支持 .ts/.tsx 转译）
    pub fn eval_module_file(&mut self, path: &Path) -> Result<String, String> {
        // 如果是 TS 文件，先转译再作为 Module 加载
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "ts" | "tsx" | "mts") {
            let source_str = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
            let transpiled = crate::transpiler::transpile(&source_str, path)
                .map_err(|e| format!("transpile error in {}: {}", path.display(), e))?;
            // 对转译后的 JS 再做 CJS→ESM 转换
            let code = crate::cjs_to_esm::transform(&transpiled.code, Some(path))
                .unwrap_or(transpiled.code);
            return self.eval_module_str(&code, Some(path));
        }

        // JS 文件：直接用 Boa 读取
        let source = boa_parser::Source::from_filepath(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let module = Module::parse(source, None, &mut self.context).map_err(|e| {
            format!(
                "parse error in {}: {}",
                path.display(),
                js_error_to_string(&e, &mut self.context)
            )
        })?;
        let promise = module.load_link_evaluate(&mut self.context);
        let _ = self.context.run_jobs();
        Ok(match promise.state() {
            boa_engine::builtins::promise::PromiseState::Fulfilled(val) => {
                js_value_to_string(&val, &mut self.context)
            }
            boa_engine::builtins::promise::PromiseState::Rejected(err) => {
                return Err(format!(
                    "module error in {}: {}",
                    path.display(),
                    js_value_to_string(&err, &mut self.context)
                ));
            }
            boa_engine::builtins::promise::PromiseState::Pending => {
                return Err(format!("module '{}' evaluation pending", path.display()));
            }
        })
    }
}

// ── Helper ──────────────────────────────────────────────────────────────────

fn js_value_to_string(val: &JsValue, ctx: &mut Context) -> String {
    match val.to_string(ctx) {
        Ok(s) => s.to_std_string_escaped(),
        Err(_) => format!("{val:?}"),
    }
}

fn js_error_to_string(err: &JsError, ctx: &mut Context) -> String {
    match err.try_native(ctx) {
        Ok(native) => native.message().to_string(),
        Err(_) => format!("{err:?}"),
    }
}
