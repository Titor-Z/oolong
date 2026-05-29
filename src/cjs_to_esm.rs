//! CJS → ESM 静态转译器
//!
//! 通过 OXC 解析 AST，采集 require() / module.exports / exports.X 等模式，
//! 生成等价的标准 ES Module 代码。

use std::fmt::Write;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::{self, AssignmentTarget, BindingPattern, Expression, PropertyKey, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

// ── 数据模型 ────────────────────────────────────────────────────────────────────

/// 单个 `require('x')` 的导入信息
struct ImportReq {
    specifier: String,
    local_name: Option<String>,
    bindings: Vec<(String, Option<String>)>,
    span: (usize, usize),
}

/// 导出信息
enum ExportItem {
    Default(String),
    Named { name: String, expr: String },
}

/// 分析结果
struct Analysis {
    imports: Vec<ImportReq>,
    side_effects: Vec<String>,
    exports: Vec<ExportItem>,
    has_dirname: bool,
    has_filename: bool,
}

// ── 公开入口 ────────────────────────────────────────────────────────────────────

pub fn transform(source: &str, source_path: Option<&Path>) -> Result<String, String> {
    let path = source_path.unwrap_or(Path::new("file.js"));
    let source_type =
        SourceType::from_path(path).map_err(|e| format!("无法确定源文件类型: {e}"))?;

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    if !ret.errors.is_empty() {
        let errs: Vec<String> = ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(format!("CJS→ESM 解析错误:\n{}", errs.join("\n")));
    }

    let analysis = analyze(&ret.program.body, source);
    let output = generate(source, &analysis, source_path);
    Ok(output)
}

// ── 分析 ────────────────────────────────────────────────────────────────────────

fn analyze(body: &[Statement], source: &str) -> Analysis {
    let mut a = Analysis {
        imports: Vec::new(),
        side_effects: Vec::new(),
        exports: Vec::new(),
        has_dirname: source.contains("__dirname"),
        has_filename: source.contains("__filename"),
    };

    for stmt in body {
        match stmt {
            // `const x = require('y')`
            // `const { a } = require('y')`
            Statement::VariableDeclaration(var_decl) => {
                for decl in &var_decl.declarations {
                    collect_require(decl, &mut a);
                }
            }

            // `module.exports = expr`
            // `exports.X = expr`
            // `require('x')`（纯副作用）
            Statement::ExpressionStatement(expr_stmt) => {
                let expr = &expr_stmt.expression;
                match expr {
                    Expression::AssignmentExpression(assign) => {
                        collect_export_assignment(assign, source, &mut a);
                    }
                    Expression::CallExpression(call) => {
                        if let Some(spec) = extract_require_string(call) {
                            a.side_effects.push(spec);
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    }

    a
}

// ── require 采集 ────────────────────────────────────────────────────────────────

fn collect_require(decl: &ast::VariableDeclarator, a: &mut Analysis) {
    let init = match &decl.init {
        Some(e) => e,
        None => return,
    };
    let call = match unwrap_require_call(init) {
        Some(c) => c,
        None => return,
    };
    let specifier = match extract_require_string(call) {
        Some(s) => s,
        None => return,
    };

    let span = (decl.span.start as usize, decl.span.end as usize);

    match &decl.id {
        BindingPattern::BindingIdentifier(ident) => {
            a.imports.push(ImportReq {
                specifier,
                local_name: Some(ident.name.to_string()),
                bindings: Vec::new(),
                span,
            });
        }
        BindingPattern::ObjectPattern(obj) => {
            let mut bindings = Vec::new();
            for prop in &obj.properties {
                let name = match &prop.key {
                    PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                    PropertyKey::StringLiteral(s) => s.value.to_string(),
                    _ => continue,
                };
                let local = match &prop.value {
                    BindingPattern::BindingIdentifier(ident) => ident.name.to_string(),
                    _ => continue,
                };
                bindings.push((name.clone(), if local != name { Some(local) } else { None }));
            }
            a.imports.push(ImportReq {
                specifier,
                local_name: None,
                bindings,
                span,
            });
        }
        BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => {
            // 解构或默认值绑定 — 兜底为 namespace 导入
            a.imports.push(ImportReq {
                specifier,
                local_name: Some("__cjs_ns".to_string()),
                bindings: Vec::new(),
                span,
            });
        }
    }
}

// ── export 采集 ─────────────────────────────────────────────────────────────────

fn collect_export_assignment(assign: &ast::AssignmentExpression, source: &str, a: &mut Analysis) {
    let rhs = extract_rhs_text(source, assign.span);

    if is_module_exports_target(&assign.left) {
        a.exports.push(ExportItem::Default(rhs));
        return;
    }

    if let Some(name) = match_exports_prop(&assign.left) {
        a.exports.push(ExportItem::Named { name, expr: rhs });
        return;
    }

    if let Some(name) = match_module_exports_prop(&assign.left) {
        a.exports.push(ExportItem::Named { name, expr: rhs });
    }
}

fn is_module_exports_target(target: &AssignmentTarget) -> bool {
    match target {
        AssignmentTarget::StaticMemberExpression(m) => {
            m.property.name == "exports" && is_ident_expr(&m.object, "module")
        }
        _ => false,
    }
}

fn match_exports_prop(target: &AssignmentTarget) -> Option<String> {
    match target {
        AssignmentTarget::StaticMemberExpression(m) => {
            if is_ident_expr(&m.object, "exports") {
                Some(m.property.name.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn match_module_exports_prop(target: &AssignmentTarget) -> Option<String> {
    match target {
        AssignmentTarget::StaticMemberExpression(m) => {
            if let Expression::StaticMemberExpression(inner) = &m.object
                && inner.property.name == "exports"
                && is_ident_expr(&inner.object, "module")
            {
                return Some(m.property.name.to_string());
            }
            None
        }
        _ => None,
    }
}

// ── AST 工具 ────────────────────────────────────────────────────────────────────

fn unwrap_require_call<'a>(expr: &'a Expression<'a>) -> Option<&'a ast::CallExpression<'a>> {
    match expr {
        Expression::CallExpression(call) => {
            if is_require_ident(&call.callee) {
                Some(call)
            } else {
                None
            }
        }
        Expression::StaticMemberExpression(m) => unwrap_require_call(&m.object),
        Expression::ComputedMemberExpression(m) => unwrap_require_call(&m.object),
        _ => None,
    }
}

fn is_require_ident(callee: &Expression) -> bool {
    match callee {
        Expression::Identifier(ident) => ident.name == "require",
        _ => false,
    }
}

fn extract_require_string(call: &ast::CallExpression) -> Option<String> {
    if !is_require_ident(&call.callee) {
        return None;
    }
    let arg = call.arguments.first()?;
    let expr = arg.as_expression()?;
    match expr {
        Expression::StringLiteral(s) => Some(s.value.to_string()),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => {
            Some(t.quasis[0].value.raw.to_string())
        }
        _ => None,
    }
}

fn is_ident_expr(expr: &Expression, expected: &str) -> bool {
    match expr {
        Expression::Identifier(ident) => ident.name == expected,
        _ => false,
    }
}

/// 从赋值表达式中提取 `=` 右侧文本
fn extract_rhs_text(source: &str, span: oxc_span::Span) -> String {
    let s = &source[span.start as usize..span.end as usize];
    if let Some(eq) = s.find('=') {
        s[eq + 1..].trim().to_string()
    } else {
        s.to_string()
    }
}

// ── ESM 生成 ────────────────────────────────────────────────────────────────────

fn generate(source: &str, a: &Analysis, source_path: Option<&Path>) -> String {
    let mut out = String::new();

    // 1. Import 声明
    for imp in &a.imports {
        if let Some(ref local) = imp.local_name {
            let _ = writeln!(out, "import {} from {:?};", local, imp.specifier);
        } else {
            let items: Vec<String> = imp
                .bindings
                .iter()
                .map(|(name, alias)| match alias {
                    Some(a) => format!("{} as {}", name, a),
                    None => name.clone(),
                })
                .collect();
            let _ = writeln!(
                out,
                "import {{ {} }} from {:?};",
                items.join(", "),
                imp.specifier
            );
        }
    }
    for spec in &a.side_effects {
        let _ = writeln!(out, "import {:?};", spec);
    }

    // 2. __dirname / __filename polyfill
    if a.has_dirname || a.has_filename {
        let dir_str = source_path
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string();
        let file_str = source_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        if a.has_dirname {
            let _ = writeln!(out, "const __dirname = {:?};", dir_str);
        }
        if a.has_filename {
            let _ = writeln!(out, "const __filename = {:?};", file_str);
        }
    }

    // 3. 原始源码，跳过已被 import 替代的语句的 byte 区间
    let mut spans: Vec<(usize, usize)> = a.imports.iter().map(|i| i.span).collect();
    spans.sort_by_key(|s| s.0);

    let mut pos = 0usize;
    for &(start, end) in &spans {
        out.push_str(&source[pos..start]);
        pos = end;
    }
    out.push_str(&source[pos..]);

    // 4. Export 声明
    for exp in &a.exports {
        match exp {
            ExportItem::Default(expr) => {
                let _ = writeln!(out, "export default {};", expr);
            }
            ExportItem::Named { name, expr } => {
                let _ = writeln!(out, "export const {} = {};", name, expr);
            }
        }
    }

    out
}

// ── 测试 ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn xform(src: &str) -> String {
        transform(src, Some(Path::new("test.js"))).unwrap()
    }

    #[test]
    fn test_basic_require() {
        let out = xform("const fs = require('fs');\nfs.readFileSync('/x');\n");
        assert!(out.contains("import fs from \"fs\""), "输出:\n{out}");
        assert!(out.contains("fs.readFileSync('/x')"), "输出:\n{out}");
    }

    #[test]
    fn test_destructured_require() {
        let out = xform("const { join, resolve } = require('path');\njoin('a', 'b');\n");
        assert!(
            out.contains("import { join, resolve } from \"path\""),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_require_alias() {
        let out = xform("const { readFile: rf } = require('fs');\nrf('/x');\n");
        assert!(
            out.contains("import { readFile as rf } from \"fs\""),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_module_exports_default() {
        let out = xform("module.exports = { foo: 1, bar: 2 };\n");
        assert!(
            out.contains("export default { foo: 1, bar: 2 };"),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_exports_property() {
        let out = xform("exports.hello = function() { return 'world'; };\n");
        assert!(
            out.contains("export const hello = function() { return 'world'; };"),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_module_exports_property() {
        let out = xform("module.exports.utils = { add: (a, b) => a + b };\n");
        assert!(
            out.contains("export const utils = { add: (a, b) => a + b };"),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_side_effect_require() {
        let out = xform("require('./polyfill');\nconsole.log('ok');\n");
        assert!(out.contains("import \"./polyfill\""), "输出:\n{out}");
        assert!(out.contains("console.log('ok')"), "输出:\n{out}");
    }

    #[test]
    fn test_empty() {
        assert_eq!(xform("").trim(), "");
    }

    #[test]
    fn test_no_cjs() {
        let out = xform("const x = 1;\nconsole.log(x);\n");
        assert!(!out.contains("import"), "输出:\n{out}");
    }

    #[test]
    fn test_mixed() {
        let out = xform(
            "const fs = require('fs');\n\
       module.exports = { read: fs.readFileSync };\n\
       exports.version = '1.0.0';\n",
        );
        assert!(out.contains("import fs from \"fs\""), "输出:\n{out}");
        assert!(
            out.contains("export default { read: fs.readFileSync };"),
            "输出:\n{out}"
        );
        assert!(
            out.contains("export const version = '1.0.0';"),
            "输出:\n{out}"
        );
    }

    #[test]
    fn test_dynamic_require_unchanged() {
        let out = xform("const name = 'fs';\nconst m = require(name);\n");
        assert!(out.contains("require(name)"), "输出:\n{out}");
    }

    #[test]
    fn test_template_require() {
        let out = xform("const m = require(`./mod`);\n");
        assert!(out.contains("import m from \"./mod\""), "输出:\n{out}");
    }

    #[test]
    fn test_dirname_polyfill() {
        let out = xform("const p = require('path');\np.join(__dirname, 'x');\n");
        assert!(out.contains("const __dirname = "), "输出:\n{out}");
        assert!(!out.contains("import.meta.dirname"), "输出:\n{out}");
    }
}
