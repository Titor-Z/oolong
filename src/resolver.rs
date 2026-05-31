use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Extensions to try when auto-completing ──────────────────────────────────
const FILE_EXTENSIONS: &[&str] = &[".js", ".mjs", ".cjs", ".json", ".ts", ".tsx", ".mts"];
const INDEX_FILES: &[&str] = &[
    "index.js",
    "index.mjs",
    "index.cjs",
    "index.json",
    "index.ts",
    "index.tsx",
];

// ── Error type ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ResolveError {
    pub specifier: String,
    pub parent: String,
    pub searched: Vec<PathBuf>,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Cannot find module '{}' from '{}'",
            self.specifier, self.parent
        )?;
        if !self.searched.is_empty() {
            writeln!(f, "Searched in:")?;
            for p in &self.searched {
                writeln!(f, "  - {}", p.display())?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ResolveError {}

// ── Get stdlib path ───────────────────────────────────────────
fn get_stdlib_path() -> PathBuf {
    if let Ok(path) = std::env::var("OOLONG_STDLIB_PATH") {
        return PathBuf::from(path);
    }

    let exe_path = std::env::current_exe()
        .ok()
        .or_else(|| std::env::var("OOLONG_DLL_PATH").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    let stdlib_path = exe_path
        .parent()
        .map(|p| p.join("src").join("stdlib"))
        .unwrap_or_else(|| PathBuf::from("./src/stdlib"));

    if !stdlib_path.exists() {
        PathBuf::from("./src/stdlib")
    } else {
        stdlib_path
    }
}

// ── Resolver ────────────────────────────────────────────────────────────────
pub struct ModuleResolver {
    resolve_cache: RefCell<HashMap<(String, PathBuf), PathBuf>>,
    exists_cache: RefCell<HashMap<PathBuf, bool>>,
    pkg_cache: RefCell<HashMap<PathBuf, Option<String>>>,
    cache_cap: usize,
    stdlib_path: PathBuf,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let stdlib_path = get_stdlib_path();
        Self {
            resolve_cache: RefCell::new(HashMap::with_capacity(cap)),
            exists_cache: RefCell::new(HashMap::with_capacity(cap * 4)),
            pkg_cache: RefCell::new(HashMap::with_capacity(cap)),
            cache_cap: cap,
            stdlib_path,
        }
    }

    // ── Public API ──────────────────────────────────────────────────────

    pub fn stdlib_path(&self) -> &Path {
        &self.stdlib_path
    }

    pub fn resolve(&self, specifier: &str, parent_path: &Path) -> Result<PathBuf, ResolveError> {
        let parent_dir = parent_path.parent().unwrap_or(Path::new("/")).to_path_buf();

        let cache_key = (specifier.to_string(), parent_dir.clone());
        if let Some(cached) = self.resolve_cache.borrow().get(&cache_key) {
            return Ok(cached.clone());
        }

        let result = if Self::is_node_internal(specifier) {
            self.resolve_nodejs_stdlib(specifier)
        } else if Self::is_relative(specifier) || Self::is_absolute(specifier) {
            self.resolve_path(specifier, &parent_dir)
        } else {
            if let Ok(path) = self.resolve_nodejs_stdlib(specifier) {
                Ok(path)
            } else if let Some(path) = self.resolve_cha_cache(specifier) {
                Ok(path)
            } else {
                self.resolve_node_modules(specifier, &parent_dir)
            }
        };

        match result {
            Ok(resolved) => {
                let mut cache = self.resolve_cache.borrow_mut();
                if cache.len() >= self.cache_cap {
                    cache.clear();
                }
                cache.insert(cache_key, resolved.clone());
                Ok(resolved)
            }
            Err(e) => Err(e),
        }
    }

    // ── Node.js stdlib resolution ───────────────────────────────────────

    /// Resolve a Node.js built-in module from stdlib on disk
    fn resolve_nodejs_stdlib(&self, specifier: &str) -> Result<PathBuf, ResolveError> {
        let module_name = if let Some(stripped) = specifier.strip_prefix("node:") {
            stripped
        } else {
            specifier
        };

        let stdlib_path = &self.stdlib_path;

        let direct_path = stdlib_path.join(module_name).with_extension("js");
        if self.file_exists(&direct_path) {
            return Ok(direct_path);
        }

        let index_path = stdlib_path.join(module_name).join("index.js");
        if self.file_exists(&index_path) {
            return Ok(index_path);
        }

        let internal_path = stdlib_path.join(format!("{}.js", module_name));
        if self.file_exists(&internal_path) {
            return Ok(internal_path);
        }

        Err(ResolveError {
            specifier: specifier.to_string(),
            parent: "node:".to_string(),
            searched: vec![direct_path, index_path, internal_path],
        })
    }

    // ── Entry point resolution ──────────────────────────────────────────

    fn resolve_entry_in_dir(&self, pkg_dir: &Path) -> Option<PathBuf> {
        let pkg_json_path = pkg_dir.join("package.json");
        if pkg_json_path.exists()
            && let Ok(content) = std::fs::read_to_string(&pkg_json_path)
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(main) = v.get("main").and_then(|m| m.as_str())
        {
            let main_path = pkg_dir.join(main);
            if self.file_exists(&main_path) {
                return Some(main_path);
            }
            let main_no_ext = main_path.with_extension("");
            for ext in FILE_EXTENSIONS {
                let candidate = PathBuf::from(format!("{}{}", main_no_ext.display(), ext));
                if self.file_exists(&candidate) {
                    return Some(candidate);
                }
            }
        }

        for idx in INDEX_FILES {
            let idx_path = pkg_dir.join(idx);
            if self.file_exists(&idx_path) {
                return Some(idx_path);
            }
        }

        None
    }

    /// Extract package name from a global key like "npm:is-odd@3.0.1".
    fn key_to_package_name(key: &str) -> Option<String> {
        let colon = key.find(':')?;
        let rest = &key[colon + 1..];
        let at = rest.rfind('@')?;
        let name = &rest[..at];
        Some(name.replace('+', "/"))
    }

    fn resolve_global_key(&self, key: &str) -> Option<PathBuf> {
        let pkg_dir = Self::global_key_to_dir(key)?;
        if !pkg_dir.exists() {
            return None;
        }
        self.resolve_entry_in_dir(&pkg_dir)
    }

    /// Resolve a bare specifier from the cha global cache (~/.cha/modules/).
    ///
    /// Resolution order:
    ///   1. `cha.json` imports map on disk (specifier → global key)
    ///   2. `cha-lock.json` packages keys matching the specifier name
    fn resolve_cha_cache(&self, specifier: &str) -> Option<PathBuf> {
        let cha_json_path = Path::new("cha.json");
        if cha_json_path.exists()
            && let Ok(content) = std::fs::read_to_string(cha_json_path)
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(imports) = v.get("imports").and_then(|i| i.as_object())
            && let Some(key) = imports.get(specifier).and_then(|s| s.as_str())
        {
            if let Some(path) = self.resolve_global_key(key) {
                return Some(path);
            }
            eprintln!(
                "Warning: package '{}' (key: {}) not found in oolong cache.\n  Tip: run `cha install {}`",
                specifier, key, specifier
            );
        }

        let lockfile_path = Path::new("cha-lock.json");
        if lockfile_path.exists()
            && let Ok(content) = std::fs::read_to_string(lockfile_path)
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(packages) = v.get("packages").and_then(|p| p.as_object())
        {
            for (key, _) in packages {
                if let Some(name) = Self::key_to_package_name(key)
                    && name == specifier
                    && let Some(path) = self.resolve_global_key(key)
                {
                    return Some(path);
                }
            }
        }

        eprintln!(
            "Warning: package '{}' not found in oolong cache.\n  Tip: run `cha install {}` to install it",
            specifier, specifier
        );
        None
    }

    /// Compute `~/.cha/modules/<source_type>/<name@version>` from a global key.
    fn global_key_to_dir(key: &str) -> Option<PathBuf> {
        let colon = key.find(':')?;
        let source_type = &key[..colon];
        let rest = &key[colon + 1..];

        let (name, version) = if let Some(at) = rest.rfind('@') {
            if at == 0 {
                (rest, "")
            } else {
                (&rest[..at], &rest[at + 1..])
            }
        } else {
            (rest, "")
        };

        if name.is_empty() {
            return None;
        }

        let safe_name = name.replace('/', "+");
        let dir_name = if version.is_empty() {
            safe_name
        } else {
            format!("{}@{}", safe_name, version)
        };

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "~".to_string());
        Some(
            PathBuf::from(home)
                .join(".cha")
                .join("modules")
                .join(source_type)
                .join(dir_name),
        )
    }

    pub fn clear_cache(&self) {
        self.resolve_cache.borrow_mut().clear();
        self.exists_cache.borrow_mut().clear();
        self.pkg_cache.borrow_mut().clear();
    }

    // ── Path classification ─────────────────────────────────────────────

    fn is_relative(specifier: &str) -> bool {
        specifier.starts_with("./") || specifier.starts_with("../")
    }

    fn is_absolute(specifier: &str) -> bool {
        let p = Path::new(specifier);
        p.is_absolute()
    }

    fn is_node_internal(specifier: &str) -> bool {
        specifier.starts_with("node:")
    }

    // ── Relative / absolute path resolution ─────────────────────────────

    fn resolve_path(&self, specifier: &str, parent_dir: &Path) -> Result<PathBuf, ResolveError> {
        let candidate = if Self::is_absolute(specifier) {
            PathBuf::from(specifier)
        } else {
            parent_dir.join(specifier)
        };

        let candidate = Self::normalize_path(&candidate);

        let mut searched = Vec::new();

        if self.file_exists(&candidate) {
            return Ok(candidate);
        }
        searched.push(candidate.clone());

        for ext in FILE_EXTENSIONS {
            let with_ext = candidate.with_extension(ext.trim_start_matches('.'));
            let with_ext_appended = PathBuf::from(format!("{}{}", candidate.display(), ext));

            if self.file_exists(&with_ext) {
                return Ok(with_ext);
            }
            searched.push(with_ext.clone());

            if with_ext != with_ext_appended && self.file_exists(&with_ext_appended) {
                return Ok(with_ext_appended);
            }
        }

        if self.dir_exists(&candidate) {
            for idx in INDEX_FILES {
                let index_path = candidate.join(idx);
                if self.file_exists(&index_path) {
                    return Ok(index_path);
                }
                searched.push(index_path);
            }
        }

        Err(ResolveError {
            specifier: specifier.to_string(),
            parent: parent_dir.display().to_string(),
            searched,
        })
    }

    // ── Bare specifier / node_modules resolution ────────────────────────

    fn resolve_node_modules(
        &self,
        specifier: &str,
        start_dir: &Path,
    ) -> Result<PathBuf, ResolveError> {
        let (pkg_name, sub_path) = Self::split_specifier(specifier);

        let mut searched = Vec::new();
        let mut dir = Some(start_dir.to_path_buf());

        while let Some(current) = dir {
            let nm_dir = current.join("node_modules").join(pkg_name);

            if self.dir_exists(&nm_dir) {
                searched.push(nm_dir.clone());

                if let Some(sub) = sub_path {
                    match self.resolve_path(&format!("./{}", sub), &nm_dir.join("__dummy__")) {
                        Ok(resolved) => return Ok(resolved),
                        Err(mut e) => {
                            searched.append(&mut e.searched);
                        }
                    }
                } else if let Some(main_field) = self.read_package_main(&nm_dir)
                    && let Ok(resolved) =
                        self.resolve_path(&format!("./{}", main_field), &nm_dir.join("__dummy__"))
                {
                    return Ok(resolved);
                }

                for idx in INDEX_FILES {
                    let index_path = nm_dir.join(idx);
                    if self.file_exists(&index_path) {
                        return Ok(index_path);
                    }
                    searched.push(index_path);
                }
            } else {
                searched.push(nm_dir);
            }

            dir = current.parent().map(|p| p.to_path_buf());
        }

        Err(ResolveError {
            specifier: specifier.to_string(),
            parent: start_dir.display().to_string(),
            searched,
        })
    }

    fn split_specifier(specifier: &str) -> (&str, Option<&str>) {
        if specifier.starts_with('@') {
            if let Some(first_slash) = specifier.find('/') {
                if let Some(second_slash) = specifier[first_slash + 1..].find('/') {
                    let split_at = first_slash + 1 + second_slash;
                    (&specifier[..split_at], Some(&specifier[split_at + 1..]))
                } else {
                    (specifier, None)
                }
            } else {
                (specifier, None)
            }
        } else {
            if let Some(slash) = specifier.find('/') {
                (&specifier[..slash], Some(&specifier[slash + 1..]))
            } else {
                (specifier, None)
            }
        }
    }

    // ── package.json parsing ────────────────────────────────────────────

    fn read_package_main(&self, pkg_dir: &Path) -> Option<String> {
        if let Some(cached) = self.pkg_cache.borrow().get(pkg_dir) {
            return cached.clone();
        }

        let pkg_json_path = pkg_dir.join("package.json");
        let result = if self.file_exists(&pkg_json_path) {
            std::fs::read_to_string(&pkg_json_path)
                .ok()
                .and_then(|content| {
                    serde_json::from_str::<serde_json::Value>(&content)
                        .ok()
                        .and_then(|v| v.get("main").and_then(|m| m.as_str().map(String::from)))
                })
        } else {
            None
        };

        let mut cache = self.pkg_cache.borrow_mut();
        if cache.len() >= self.cache_cap {
            cache.clear();
        }
        cache.insert(pkg_dir.to_path_buf(), result.clone());
        result
    }

    // ── File system helpers (with caching) ──────────────────────────────

    fn file_exists(&self, path: &Path) -> bool {
        if let Some(&cached) = self.exists_cache.borrow().get(path) {
            return cached;
        }

        let exists = path.is_file();

        let mut cache = self.exists_cache.borrow_mut();
        if cache.len() >= self.cache_cap * 4 {
            cache.clear();
        }
        cache.insert(path.to_path_buf(), exists);
        exists
    }

    fn dir_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                other => {
                    components.push(other);
                }
            }
        }
        components.iter().collect()
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_specifier_regular() {
        assert_eq!(ModuleResolver::split_specifier("lodash"), ("lodash", None));
        assert_eq!(
            ModuleResolver::split_specifier("lodash/fp"),
            ("lodash", Some("fp"))
        );
        assert_eq!(
            ModuleResolver::split_specifier("lodash/fp/object"),
            ("lodash", Some("fp/object"))
        );
    }

    #[test]
    fn test_split_specifier_scoped() {
        assert_eq!(
            ModuleResolver::split_specifier("@babel/core"),
            ("@babel/core", None)
        );
        assert_eq!(
            ModuleResolver::split_specifier("@babel/core/lib/parse"),
            ("@babel/core", Some("lib/parse"))
        );
    }

    #[test]
    fn test_is_relative() {
        assert!(ModuleResolver::is_relative("./foo"));
        assert!(ModuleResolver::is_relative("../bar"));
        assert!(!ModuleResolver::is_relative("lodash"));
        assert!(!ModuleResolver::is_relative("/abs/path"));
    }

    #[test]
    fn test_normalize_path() {
        let p = ModuleResolver::normalize_path(Path::new("/a/b/../c/./d"));
        assert_eq!(p, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn test_file_extensions_includes_ts() {
        assert!(
            FILE_EXTENSIONS.contains(&".ts"),
            "FILE_EXTENSIONS should include .ts"
        );
        assert!(
            FILE_EXTENSIONS.contains(&".tsx"),
            "FILE_EXTENSIONS should include .tsx"
        );
        assert!(
            FILE_EXTENSIONS.contains(&".mts"),
            "FILE_EXTENSIONS should include .mts"
        );
    }

    #[test]
    fn test_index_files_includes_ts() {
        assert!(
            INDEX_FILES.contains(&"index.ts"),
            "INDEX_FILES should include index.ts"
        );
        assert!(
            INDEX_FILES.contains(&"index.tsx"),
            "INDEX_FILES should include index.tsx"
        );
    }

    #[test]
    fn test_ts_extensions_after_js_extensions() {
        let ts_start = FILE_EXTENSIONS.iter().position(|e| *e == ".ts").unwrap();
        let js_pos = FILE_EXTENSIONS.iter().position(|e| *e == ".js").unwrap();
        assert!(
            ts_start > js_pos,
            ".ts should come after .js in FILE_EXTENSIONS"
        );
    }

    fn ts_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oolong_ts_{}", name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_resolve_ts_file_via_temp() {
        let dir = ts_test_dir("resolve_ts_file");
        std::fs::write(dir.join("module.ts"), "export const x = 1;").unwrap();

        let resolver = ModuleResolver::new();
        let result = resolver.resolve("./module.ts", &dir.join("entry.js"));
        assert!(
            result.is_ok(),
            "should resolve .ts file: {:?}",
            result.err()
        );
        let resolved = result.unwrap();
        assert_eq!(resolved.file_name().unwrap(), "module.ts");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_resolve_ts_without_extension() {
        let dir = ts_test_dir("resolve_ts_noext");
        std::fs::write(dir.join("lib.ts"), "export const x = 1;").unwrap();

        let resolver = ModuleResolver::new();
        let result = resolver.resolve("./lib", &dir.join("entry.js"));
        assert!(
            result.is_ok(),
            "should resolve .ts without extension: {:?}",
            result.err()
        );
        let resolved = result.unwrap();
        assert_eq!(resolved.file_name().unwrap(), "lib.ts");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_prefer_js_over_ts() {
        let dir = ts_test_dir("prefer_js");
        std::fs::write(dir.join("lib.js"), "const x = 1;").unwrap();
        std::fs::write(dir.join("lib.ts"), "export const x = 1;").unwrap();

        let resolver = ModuleResolver::new();
        let result = resolver.resolve("./lib", &dir.join("entry.js")).unwrap();
        assert_eq!(
            result.file_name().unwrap(),
            "lib.js",
            ".js should be preferred over .ts"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
