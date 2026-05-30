use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};
use std::fmt::Write;

fn make_fn(
    native: boa_engine::NativeFunction,
    name: &str,
    len: usize,
    ctx: &mut Context,
) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVerData {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
    build: Vec<String>,
}

fn parse_semver(s: &str) -> Option<SemVerData> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (base, build_str) = if let Some(pos) = s.find('+') {
        (&s[..pos], Some(&s[pos + 1..]))
    } else {
        (s, None)
    };

    let (ver_str, pre_str) = if let Some(pos) = base.find('-') {
        (&base[..pos], Some(&base[pos + 1..]))
    } else {
        (base, None)
    };

    let parts: Vec<&str> = ver_str.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts[2].parse().ok()?;

    let prerelease: Vec<String> = pre_str
        .map(|s| s.split('.').map(|p| p.to_string()).collect())
        .unwrap_or_default();

    for ident in &prerelease {
        if ident.is_empty() {
            return None;
        }
        if !ident
            .chars()
            .all(|c: char| c.is_ascii_alphanumeric() || c == '-')
        {
            return None;
        }
    }

    let build: Vec<String> = build_str
        .map(|s| s.split('.').map(|p| p.to_string()).collect())
        .unwrap_or_default();

    for ident in &build {
        if ident.is_empty() {
            return None;
        }
        if !ident
            .chars()
            .all(|c: char| c.is_ascii_alphanumeric() || c == '-')
        {
            return None;
        }
    }

    Some(SemVerData {
        major,
        minor,
        patch,
        prerelease,
        build,
    })
}

fn parse_version_arg(v: &JsValue, ctx: &mut Context) -> Result<SemVerData, JsValue> {
    if let Some(str) = v.as_string() {
        let s = str.to_std_string_escaped();
        parse_semver(&s).ok_or_else(|| JsValue::from(js_string!("invalid semver")))
    } else if v.is_object() {
        let obj = v.as_object().unwrap();
        let major = obj
            .get(js_string!("major"), ctx)
            .ok()
            .and_then(|v| v.as_number())
            .unwrap_or(0.0) as u64;
        let minor = obj
            .get(js_string!("minor"), ctx)
            .ok()
            .and_then(|v| v.as_number())
            .unwrap_or(0.0) as u64;
        let patch = obj
            .get(js_string!("patch"), ctx)
            .ok()
            .and_then(|v| v.as_number())
            .unwrap_or(0.0) as u64;
        let prerelease = obj
            .get(js_string!("prerelease"), ctx)
            .ok()
            .and_then(|v| v.as_object())
            .map(|o| {
                let mut r = Vec::new();
                let len = o
                    .get(js_string!("length"), ctx)
                    .ok()
                    .and_then(|v| v.as_number())
                    .unwrap_or(0.0) as u32;
                for i in 0..len {
                    if let Ok(v) = o.get(i, ctx) {
                        if let Some(s) = v.as_string() {
                            r.push(s.to_std_string_escaped());
                        }
                    }
                }
                r
            })
            .unwrap_or_default();
        let build = obj
            .get(js_string!("build"), ctx)
            .ok()
            .and_then(|v| v.as_object())
            .map(|o| {
                let mut r = Vec::new();
                let len = o
                    .get(js_string!("length"), ctx)
                    .ok()
                    .and_then(|v| v.as_number())
                    .unwrap_or(0.0) as u32;
                for i in 0..len {
                    if let Ok(v) = o.get(i, ctx) {
                        if let Some(s) = v.as_string() {
                            r.push(s.to_std_string_escaped());
                        }
                    }
                }
                r
            })
            .unwrap_or_default();
        Ok(SemVerData {
            major,
            minor,
            patch,
            prerelease,
            build,
        })
    } else {
        Err(JsValue::from(js_string!("invalid semver")))
    }
}

fn compare_identifiers(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let max_len = a.len().max(b.len());
    for i in 0..max_len {
        let a_has = i < a.len();
        let b_has = i < b.len();
        if !a_has && b_has {
            return std::cmp::Ordering::Less;
        }
        if a_has && !b_has {
            return std::cmp::Ordering::Greater;
        }
        let a_part = &a[i];
        let b_part = &b[i];
        let a_num = a_part.parse::<u64>();
        let b_num = b_part.parse::<u64>();
        match (a_num, b_num) {
            (Ok(an), Ok(bn)) => {
                let c = an.cmp(&bn);
                if c != std::cmp::Ordering::Equal {
                    return c;
                }
            }
            (Ok(_), Err(_)) => return std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => return std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => {
                let c = a_part.cmp(b_part);
                if c != std::cmp::Ordering::Equal {
                    return c;
                }
            }
        }
    }
    std::cmp::Ordering::Equal
}

fn compare_semver(a: &SemVerData, b: &SemVerData) -> std::cmp::Ordering {
    let c = a.major.cmp(&b.major);
    if c != std::cmp::Ordering::Equal {
        return c;
    }
    let c = a.minor.cmp(&b.minor);
    if c != std::cmp::Ordering::Equal {
        return c;
    }
    let c = a.patch.cmp(&b.patch);
    if c != std::cmp::Ordering::Equal {
        return c;
    }
    if a.prerelease.is_empty() && !b.prerelease.is_empty() {
        return std::cmp::Ordering::Greater;
    }
    if !a.prerelease.is_empty() && b.prerelease.is_empty() {
        return std::cmp::Ordering::Less;
    }
    compare_identifiers(&a.prerelease, &b.prerelease)
}

fn create_semver_object(data: &SemVerData, ctx: &mut Context) -> JsValue {
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    obj.set(js_string!("major"), data.major, false, ctx).ok();
    obj.set(js_string!("minor"), data.minor, false, ctx).ok();
    obj.set(js_string!("patch"), data.patch, false, ctx).ok();

    let prerelease_arr = JsArray::new(ctx);
    for (i, s) in data.prerelease.iter().enumerate() {
        prerelease_arr
            .set(i as u32, JsValue::from(js_string!(s.clone())), false, ctx)
            .ok();
    }
    obj.set(js_string!("prerelease"), prerelease_arr, false, ctx)
        .ok();

    let build_arr = JsArray::new(ctx);
    for (i, s) in data.build.iter().enumerate() {
        build_arr
            .set(i as u32, JsValue::from(js_string!(s.clone())), false, ctx)
            .ok();
    }
    obj.set(js_string!("build"), build_arr, false, ctx).ok();

    let fmt_str = format_semver(data);
    let to_string_fn = {
        let s = fmt_str;
        FunctionObjectBuilder::new(
            ctx.realm(),
            NativeFunction::from_copy_closure_with_captures(
                |_: &JsValue, _: &[JsValue], captured: &String, _: &mut Context| {
                    Ok(JsValue::from(js_string!(captured.clone())))
                },
                s,
            ),
        )
        .name(js_string!("toString"))
        .length(0)
        .build()
    };
    obj.set(js_string!("toString"), to_string_fn, false, ctx)
        .ok();

    obj.into()
}

fn format_semver(data: &SemVerData) -> String {
    let mut s = format!("{}.{}.{}", data.major, data.minor, data.patch);
    if !data.prerelease.is_empty() {
        write!(s, "-{}", data.prerelease.join(".")).ok();
    }
    if !data.build.is_empty() {
        write!(s, "+{}", data.build.join(".")).ok();
    }
    s
}

fn parse_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let input = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let data = parse_semver(&input)
        .ok_or_else(|| JsNativeError::typ().with_message("invalid semver string"))?;
    Ok(create_semver_object(&data, ctx))
}

fn format_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.is_empty() {
        return Err(JsNativeError::typ()
            .with_message("format requires a SemVer argument")
            .into());
    }
    let data = parse_version_arg(&args[0], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid semver argument"))?;
    Ok(JsValue::from(js_string!(format_semver(&data))))
}

fn compare_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Err(JsNativeError::typ()
            .with_message("compare requires two arguments")
            .into());
    }
    let a = parse_version_arg(&args[0], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid first argument"))?;
    let b = parse_version_arg(&args[1], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid second argument"))?;
    let ord = compare_semver(&a, &b);
    let result: i32 = match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };
    Ok(JsValue::from(result))
}

fn greater_than_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Err(JsNativeError::typ()
            .with_message("greaterThan requires two arguments")
            .into());
    }
    let a = parse_version_arg(&args[0], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid first argument"))?;
    let b = parse_version_arg(&args[1], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid second argument"))?;
    Ok(JsValue::from(
        compare_semver(&a, &b) == std::cmp::Ordering::Greater,
    ))
}

fn less_than_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Err(JsNativeError::typ()
            .with_message("lessThan requires two arguments")
            .into());
    }
    let a = parse_version_arg(&args[0], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid first argument"))?;
    let b = parse_version_arg(&args[1], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid second argument"))?;
    Ok(JsValue::from(
        compare_semver(&a, &b) == std::cmp::Ordering::Less,
    ))
}

fn equals_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Err(JsNativeError::typ()
            .with_message("equals requires two arguments")
            .into());
    }
    let a = parse_version_arg(&args[0], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid first argument"))?;
    let b = parse_version_arg(&args[1], ctx)
        .map_err(|_| JsNativeError::typ().with_message("invalid second argument"))?;
    Ok(JsValue::from(
        compare_semver(&a, &b) == std::cmp::Ordering::Equal,
    ))
}

#[derive(Debug, Clone)]
enum RangeOp {
    Caret,
    Tilde,
    Gte,
    Lte,
    Gt,
    Lt,
    Eq,
    Exact,
    Wild,
}

#[derive(Debug, Clone)]
struct RangeConstraint {
    op: RangeOp,
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
}

fn parse_range(s: &str) -> Option<Vec<RangeConstraint>> {
    let s = s.trim();
    if s == "*" || s == "x" || s == "X" {
        return Some(vec![RangeConstraint {
            op: RangeOp::Wild,
            major: 0,
            minor: None,
            patch: None,
        }]);
    }
    let (op, rest) = if s.starts_with("^") {
        (RangeOp::Caret, &s[1..])
    } else if s.starts_with("~") {
        (RangeOp::Tilde, &s[1..])
    } else if s.starts_with(">=") {
        (RangeOp::Gte, &s[2..])
    } else if s.starts_with("<=") {
        (RangeOp::Lte, &s[2..])
    } else if s.starts_with(">") {
        (RangeOp::Gt, &s[1..])
    } else if s.starts_with("<") {
        (RangeOp::Lt, &s[1..])
    } else if s.starts_with("=") {
        (RangeOp::Eq, &s[1..])
    } else {
        (RangeOp::Exact, s)
    };
    let rest = rest.trim();
    let parts: Vec<&str> = rest.split('.').collect();
    let major = parts.first()?.parse().ok()?;
    let minor = parts.get(1).and_then(|p| {
        if *p == "x" || *p == "X" || *p == "*" {
            None
        } else {
            p.parse().ok()
        }
    });
    let patch = parts.get(2).and_then(|p| {
        if *p == "x" || *p == "X" || *p == "*" {
            None
        } else {
            p.parse().ok()
        }
    });
    Some(vec![RangeConstraint {
        op,
        major,
        minor,
        patch,
    }])
}

fn satisfies_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Err(JsNativeError::typ()
            .with_message("satisfies requires two arguments")
            .into());
    }
    let version_str = args[0]
        .as_string()
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let range_str = args[1]
        .as_string()
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let version = match parse_semver(&version_str) {
        Some(v) => v,
        None => return Ok(JsValue::from(false)),
    };
    let constraints = match parse_range(&range_str) {
        Some(c) => c,
        None => return Ok(JsValue::from(false)),
    };
    for constraint in &constraints {
        if !satisfies_constraint(&version, constraint) {
            return Ok(JsValue::from(false));
        }
    }
    Ok(JsValue::from(true))
}

fn satisfies_constraint(v: &SemVerData, c: &RangeConstraint) -> bool {
    match c.op {
        RangeOp::Wild => true,
        RangeOp::Exact | RangeOp::Eq => {
            if v.major != c.major {
                return false;
            }
            if let Some(minor) = c.minor {
                if v.minor != minor {
                    return false;
                }
            }
            if let Some(patch) = c.patch {
                if v.patch != patch {
                    return false;
                }
            }
            true
        }
        RangeOp::Caret => {
            let lower = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            if compare_semver(v, &lower) == std::cmp::Ordering::Less {
                return false;
            }
            let upper = if c.major > 0 {
                SemVerData {
                    major: c.major + 1,
                    minor: 0,
                    patch: 0,
                    prerelease: vec![],
                    build: vec![],
                }
            } else if let Some(minor) = c.minor {
                if minor > 0 {
                    SemVerData {
                        major: 0,
                        minor: minor + 1,
                        patch: 0,
                        prerelease: vec![],
                        build: vec![],
                    }
                } else {
                    SemVerData {
                        major: 0,
                        minor: 0,
                        patch: c.patch.unwrap_or(0) + 1,
                        prerelease: vec![],
                        build: vec![],
                    }
                }
            } else {
                SemVerData {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    prerelease: vec![],
                    build: vec![],
                }
            };
            compare_semver(v, &upper) == std::cmp::Ordering::Less
        }
        RangeOp::Tilde => {
            let lower = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            if compare_semver(v, &lower) == std::cmp::Ordering::Less {
                return false;
            }
            if c.major != v.major {
                return false;
            }
            if let Some(minor) = c.minor {
                if v.minor != minor {
                    return false;
                }
            }
            true
        }
        RangeOp::Gte => {
            let target = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            compare_semver(v, &target) != std::cmp::Ordering::Less
        }
        RangeOp::Lte => {
            let target = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            compare_semver(v, &target) != std::cmp::Ordering::Greater
        }
        RangeOp::Gt => {
            let target = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            compare_semver(v, &target) == std::cmp::Ordering::Greater
        }
        RangeOp::Lt => {
            let target = SemVerData {
                major: c.major,
                minor: c.minor.unwrap_or(0),
                patch: c.patch.unwrap_or(0),
                prerelease: vec![],
                build: vec![],
            };
            compare_semver(v, &target) == std::cmp::Ordering::Less
        }
    }
}

pub fn create_semver_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("parse"),
        js_string!("format"),
        js_string!("compare"),
        js_string!("greaterThan"),
        js_string!("lessThan"),
        js_string!("equals"),
        js_string!("satisfies"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let parse_fn = make_fn(NativeFunction::from_fn_ptr(parse_impl), "parse", 1, ctx);
                let format_fn = make_fn(NativeFunction::from_fn_ptr(format_impl), "format", 1, ctx);
                let compare_fn =
                    make_fn(NativeFunction::from_fn_ptr(compare_impl), "compare", 2, ctx);
                let gt_fn = make_fn(
                    NativeFunction::from_fn_ptr(greater_than_impl),
                    "greaterThan",
                    2,
                    ctx,
                );
                let lt_fn = make_fn(
                    NativeFunction::from_fn_ptr(less_than_impl),
                    "lessThan",
                    2,
                    ctx,
                );
                let eq_fn = make_fn(NativeFunction::from_fn_ptr(equals_impl), "equals", 2, ctx);
                let satisfies_fn = make_fn(
                    NativeFunction::from_fn_ptr(satisfies_impl),
                    "satisfies",
                    2,
                    ctx,
                );

                m.set_export(&js_string!("parse"), parse_fn.clone())?;
                m.set_export(&js_string!("format"), format_fn.clone())?;
                m.set_export(&js_string!("compare"), compare_fn.clone())?;
                m.set_export(&js_string!("greaterThan"), gt_fn.clone())?;
                m.set_export(&js_string!("lessThan"), lt_fn.clone())?;
                m.set_export(&js_string!("equals"), eq_fn.clone())?;
                m.set_export(&js_string!("satisfies"), satisfies_fn.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj
                    .set(js_string!("parse"), parse_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("format"), format_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("compare"), compare_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("greaterThan"), gt_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("lessThan"), lt_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("equals"), eq_fn, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("satisfies"), satisfies_fn, false, ctx)
                    .ok();
                m.set_export(&js_string!("default"), JsValue::from(default_obj))?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
