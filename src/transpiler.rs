use std::path::Path;

use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_sourcemap::SourceMap;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};

pub struct TranspileResult {
    pub code: String,
    pub sourcemap: Option<SourceMap>,
}

pub fn transpile(source: &str, source_path: &Path) -> Result<TranspileResult, String> {
    let allocator = Allocator::default();

    let source_type = SourceType::from_path(source_path)
        .map_err(|e| format!("cannot determine source type: {e}"))?;

    let parser_return = Parser::new(&allocator, source, source_type).parse();

    if !parser_return.errors.is_empty() {
        let errs: Vec<String> = parser_return.errors.iter().map(|e| e.to_string()).collect();
        return Err(format!("parse errors:\n{}", errs.join("\n")));
    }

    let mut program = parser_return.program;

    let scoping = SemanticBuilder::new()
        .with_enum_eval(true)
        .build(&program)
        .semantic
        .into_scoping();

    let transform_options = TransformOptions {
        ..Default::default()
    };

    let transform_result = Transformer::new(&allocator, source_path, &transform_options)
        .build_with_scoping(scoping, &mut program);

    if !transform_result.errors.is_empty() {
        let errs: Vec<String> = transform_result
            .errors
            .iter()
            .map(|e| e.to_string())
            .collect();
        return Err(format!("transform errors:\n{}", errs.join("\n")));
    }

    let codegen_options = CodegenOptions {
        source_map_path: Some(source_path.to_path_buf()),
        ..CodegenOptions::default()
    };

    let codegen_return = Codegen::new()
        .with_options(codegen_options)
        .with_source_text(source)
        .with_source_type(source_type)
        .build(&program);

    Ok(TranspileResult {
        code: codegen_return.code,
        sourcemap: codegen_return.map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn transpile_ok(source: &str, name: &str) -> String {
        transpile(source, Path::new(name)).unwrap().code
    }

    #[test]
    fn test_strip_type_annotations() {
        let js = transpile_ok("const x: number = 1;", "test.ts");
        assert!(!js.contains(": number"));
        assert!(js.contains("const x = 1"));
    }

    #[test]
    fn test_strip_interface() {
        let js = transpile_ok(
            "interface Foo { bar: string; }\nconst x: Foo = { bar: 'hello' };",
            "test.ts",
        );
        assert!(!js.contains("interface"));
        assert!(js.contains("bar"));
    }

    #[test]
    fn test_transpile_enum() {
        let js = transpile_ok("enum Color { Red, Green, Blue }", "test.ts");
        assert!(!js.contains("enum"));
        assert!(js.contains("Color"));
    }

    #[test]
    fn test_strip_generics() {
        let js = transpile_ok("function identity<T>(arg: T): T { return arg; }", "test.ts");
        assert!(!js.contains("<T>"));
        assert!(js.contains("function identity("));
    }

    #[test]
    fn test_class_parameter_properties() {
        let js = transpile_ok(
            "class Person { constructor(public name: string) {} }",
            "test.ts",
        );
        assert!(!js.contains(": string"));
        assert!(js.contains("this.name = name"));
    }

    #[test]
    fn test_empty_source() {
        let js = transpile_ok("", "test.ts");
        assert_eq!(js.trim(), "");
    }

    #[test]
    fn test_plain_js_passthrough() {
        let js = transpile_ok("const x = 1; console.log(x);", "test.ts");
        assert!(js.contains("const x = 1"));
        assert!(js.contains("console.log(x)"));
    }

    #[test]
    fn test_invalid_syntax() {
        let result = transpile("const x = ", Path::new("test.ts"));
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_extension() {
        let result = transpile("const x = 1;", Path::new("test.py"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sourcemap_generated() {
        let result = transpile("const x: number = 1;", Path::new("test.ts")).unwrap();
        assert!(
            result.sourcemap.is_some(),
            "sourcemap should be generated when source_map_path is set"
        );
    }

    #[test]
    fn test_sourcemap_has_tokens() {
        let result = transpile("const x: number = 1;", Path::new("test.ts")).unwrap();
        let sm = result.sourcemap.unwrap();
        let tokens: Vec<_> = sm.get_tokens().collect();
        assert!(
            !tokens.is_empty(),
            "sourcemap should have at least one token"
        );
        let token = &tokens[0];
        assert!(token.get_dst_line() == 0 || token.get_src_line() == 0);
    }
}
