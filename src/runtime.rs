use std::path::Path;
use std::rc::Rc;

use boa_engine::context::ContextBuilder;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsError, JsValue, Module, Source};
use boa_runtime::Console;

use crate::module_loader::OolongModuleLoader;

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

    /// 注册内置模块（import "path" / import "process" 等）
    fn register_builtins(&mut self) {
        let path_mod =
            crate::std::path::create_path_module(&mut self.context).expect("创建 path 模块失败");
        self.loader.register_builtin("path", path_mod);

        let process_mod = crate::std::process::create_process_module(&mut self.context)
            .expect("创建 process 模块失败");
        self.loader.register_builtin("process", process_mod);

        let fs_mod = crate::std::fs::create_fs_module(&mut self.context)
            .expect("创建 fs 模块失败");
        self.loader.register_builtin("fs", fs_mod);

        let os_mod = crate::std::os::create_os_module(&mut self.context)
            .expect("创建 os 模块失败");
        self.loader.register_builtin("os", os_mod);
    }

    /// 注册 setTimeout/setInterval/clearTimeout/clearInterval
    fn register_timers(&mut self) {
        boa_runtime::interval::register(&mut self.context).expect("注册 timers 失败");
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
