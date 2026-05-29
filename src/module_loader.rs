use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use boa_engine::module::{ModuleLoader, Referrer};
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsString, Module, Source};
use boa_gc::GcRefCell;
use rustc_hash::FxHashMap;

use crate::resolver::ModuleResolver;

/// 内置模块白名单（不触发 "cha install" 提示）
const BUILTIN_MODULES: &[&str] = &["path", "process", "fs", "os"];

pub struct OolongModuleLoader {
    root: PathBuf,
    resolver: ModuleResolver,
    module_map: GcRefCell<FxHashMap<PathBuf, Module>>,
    /// 内置模块（"path" → Module）
    builtins: GcRefCell<HashMap<String, Module>>,
}

impl OolongModuleLoader {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            resolver: ModuleResolver::new(),
            module_map: GcRefCell::default(),
            builtins: GcRefCell::default(),
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

    fn referrer_file(&self, referrer: &Referrer) -> PathBuf {
        referrer
            .path()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.root.join("__entry__.js"))
    }
}

impl ModuleLoader for OolongModuleLoader {
    fn load_imported_module(
        self: Rc<Self>,
        referrer: Referrer,
        specifier: JsString,
        context: &RefCell<&mut Context>,
    ) -> impl Future<Output = JsResult<Module>> {
        let result = (|| {
            let spec = specifier.to_std_string_escaped();

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

            let source_bytes = std::fs::read(&resolved).map_err(|err| {
                JsError::from(JsNativeError::typ().with_message(format!(
                    "cannot read module '{}': {}",
                    resolved.display(),
                    err
                )))
            })?;

            let source_bytes = {
                let ext = resolved.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "ts" | "tsx" | "mts") {
                    let source_str = String::from_utf8(source_bytes).map_err(|_| {
                        JsError::from(
                            JsNativeError::typ().with_message("invalid UTF-8 in TypeScript file"),
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
                    source_bytes
                }
            };

            let source_bytes = {
                let source_str = String::from_utf8(source_bytes).map_err(|_| {
                    JsError::from(JsNativeError::typ().with_message("invalid UTF-8 in source file"))
                })?;
                match crate::cjs_to_esm::transform(&source_str, Some(&resolved)) {
                    Ok(code) => code.into_bytes(),
                    Err(_) => source_str.into_bytes(),
                }
            };

            let source = Source::from_bytes(&source_bytes);

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

fn is_bare_specifier(spec: &str) -> bool {
    !spec.starts_with("./")
        && !spec.starts_with("../")
        && !spec.starts_with('/')
        && !spec.starts_with("node:")
        && !spec.starts_with("file:")
        && !BUILTIN_MODULES.contains(&spec)
}
