#![allow(clippy::collapsible_if)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsObject, JsResult, JsValue, NativeFunction, js_string};

thread_local! {
    static TIMERS: RefCell<HashMap<String, Instant>> = RefCell::new(HashMap::new());
    static COUNTERS: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
}

fn format_value(val: &JsValue, ctx: &mut Context) -> String {
    if let Some(s) = val.as_string() {
        return s.to_std_string_escaped();
    }
    if let Some(n) = val.as_number() {
        return n.to_string();
    }
    if val.is_null() {
        return "null".into();
    }
    if val.is_undefined() {
        return "undefined".into();
    }
    if let Some(b) = val.as_boolean() {
        return b.to_string();
    }
    val.to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|_| "?".into())
}

fn format_args(args: &[JsValue], ctx: &mut Context) -> String {
    if args.is_empty() {
        return String::new();
    }
    let first = &args[0];
    if let Some(fmt) = first.as_string() {
        let fmt = fmt.to_std_string_escaped();
        if fmt.contains('%') {
            return format_specifier(&fmt, &args[1..], ctx);
        }
    }
    args.iter()
        .map(|v| format_value(v, ctx))
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_specifier(fmt: &str, args: &[JsValue], ctx: &mut Context) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    let mut in_percent = false;

    for c in fmt.chars() {
        if in_percent {
            in_percent = false;
            match c {
                's' => {
                    result.push_str(
                        &args
                            .get(arg_idx)
                            .map(|v| format_value(v, ctx))
                            .unwrap_or_default(),
                    );
                    arg_idx += 1;
                }
                'd' | 'i' => {
                    let s = args
                        .get(arg_idx)
                        .and_then(|v| v.as_number())
                        .map(|n| format!("{}", n as i64))
                        .unwrap_or_else(|| "NaN".into());
                    result.push_str(&s);
                    arg_idx += 1;
                }
                'f' => {
                    let s = args
                        .get(arg_idx)
                        .and_then(|v| v.as_number())
                        .map(|n| format!("{n}"))
                        .unwrap_or_else(|| "NaN".into());
                    result.push_str(&s);
                    arg_idx += 1;
                }
                'o' | 'O' => {
                    result.push_str(
                        &args
                            .get(arg_idx)
                            .map(|v| format_value(v, ctx))
                            .unwrap_or_default(),
                    );
                    arg_idx += 1;
                }
                '%' => {
                    result.push('%');
                }
                _ => {
                    result.push('%');
                    result.push(c);
                }
            }
        } else if c == '%' {
            in_percent = true;
        } else {
            result.push(c);
        }
    }
    // Append remaining args
    if arg_idx < args.len() {
        for v in &args[arg_idx..] {
            result.push(' ');
            result.push_str(&format_value(v, ctx));
        }
    }
    result
}

fn stdout_write(msg: &str) {
    let _ = writeln!(std::io::stdout(), "{msg}");
}
fn stderr_write(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}

// ── Log methods ──────────────────────────────────────────────────────────────

fn log_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = format_args(args, ctx);
    stdout_write(&msg);
    Ok(JsValue::undefined())
}

fn warn_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = format_args(args, ctx);
    stderr_write(&msg);
    Ok(JsValue::undefined())
}

fn error_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = format_args(args, ctx);
    stderr_write(&msg);
    Ok(JsValue::undefined())
}

fn trace_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = format_args(args, ctx);
    // Get a simple stack trace indicator
    let mut output = if msg.is_empty() {
        "Trace:".to_string()
    } else {
        format!("Trace: {msg}")
    };
    // The spec says to output a stack trace. We'll add a basic one.
    output.push_str("\n    (stack trace not yet implemented)");
    stderr_write(&output);
    Ok(JsValue::undefined())
}

fn assert_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let condition = args.first().map(|v| v.to_boolean()).unwrap_or(false);
    if !condition {
        let msg = if args.len() > 1 {
            format_args(&args[1..], ctx)
        } else {
            "console.assert".to_string()
        };
        stderr_write(&format!("Assertion failed: {msg}"));
    }
    Ok(JsValue::undefined())
}

// ── Time methods ─────────────────────────────────────────────────────────────

fn time_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    TIMERS.with(|t| {
        t.borrow_mut().insert(label, Instant::now());
    });
    Ok(JsValue::undefined())
}

fn time_end_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    let elapsed = TIMERS.with(|t| t.borrow_mut().remove(&label));
    if let Some(start) = elapsed {
        let dur = start.elapsed();
        let ms = dur.as_secs_f64() * 1000.0;
        stdout_write(&format!("{label}: {ms:.1} ms"));
    } else {
        stderr_write(&format!("Timer \"{label}\" doesn't exist"));
    }
    Ok(JsValue::undefined())
}

fn time_log_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    let elapsed = TIMERS.with(|t| t.borrow().get(&label).copied());
    let extra = if args.len() > 1 {
        format_args(&args[1..], ctx)
    } else {
        String::new()
    };
    if let Some(start) = elapsed {
        let dur = start.elapsed();
        let ms = dur.as_secs_f64() * 1000.0;
        let msg = if extra.is_empty() {
            format!("{label}: {ms:.1} ms")
        } else {
            format!("{label}: {ms:.1} ms {extra}")
        };
        stdout_write(&msg);
    } else {
        stderr_write(&format!("Timer \"{label}\" doesn't exist"));
    }
    Ok(JsValue::undefined())
}

// ── Count methods ────────────────────────────────────────────────────────────

fn count_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    let count = COUNTERS.with(|c| {
        let mut c = c.borrow_mut();
        let entry = c.entry(label.clone()).or_insert(0);
        *entry += 1;
        *entry
    });
    stdout_write(&format!("{label}: {count}"));
    Ok(JsValue::undefined())
}

fn count_reset_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    COUNTERS.with(|c| {
        c.borrow_mut().remove(&label);
    });
    Ok(JsValue::undefined())
}

// ── Clear ────────────────────────────────────────────────────────────────────

fn clear_impl(_: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    // In a terminal, clear has no visible effect.
    // We write an empty output for compatibility.
    Ok(JsValue::undefined())
}

// ── Table ────────────────────────────────────────────────────────────────────

fn table_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if let Some(data) = args.first() {
        if let Some(obj) = data.as_object() {
            // Try array first
            if let Ok(arr) = JsArray::from_object(obj.clone()) {
                let len = arr.length(ctx).unwrap_or(0) as usize;
                for i in 0..len {
                    if let Ok(item) = arr.get(i as u32, ctx) {
                        let line = format!("{i}: {}", format_value(&item, ctx));
                        stdout_write(&line);
                    }
                }
                return Ok(JsValue::undefined());
            }
            // Object: show keys
            if let Ok(keys) = obj.own_property_keys(ctx) {
                for key in &keys {
                    use boa_engine::property::PropertyKey::String as PkString;
                    if let PkString(jk) = key {
                        if let Ok(val) = obj.get(jk.clone(), ctx) {
                            let ks = jk.to_std_string_escaped();
                            let vs = format_value(&val, ctx);
                            stdout_write(&format!("{ks}: {vs}"));
                        }
                    }
                }
                return Ok(JsValue::undefined());
            }
        }
        stdout_write(&format_value(data, ctx));
    }
    Ok(JsValue::undefined())
}

// ── Group ────────────────────────────────────────────────────────────────────

thread_local! {
    static GROUP_LEVEL: RefCell<usize> = const { RefCell::new(0) };
}

fn group_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let label = args
        .first()
        .map(|v| format_value(v, ctx))
        .unwrap_or_default();
    let indent = GROUP_LEVEL.with(|l| {
        let level = *l.borrow();
        *l.borrow_mut() += 1;
        level
    });
    let prefix = "  ".repeat(indent);
    if !label.is_empty() {
        stdout_write(&format!("{prefix}{label}"));
    }
    Ok(JsValue::undefined())
}

fn group_end_impl(_: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    GROUP_LEVEL.with(|l| {
        let mut l = l.borrow_mut();
        if *l > 0 {
            *l -= 1;
        }
    });
    Ok(JsValue::undefined())
}

fn group_collapsed_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    group_impl(this, args, ctx)
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    let console = JsObject::with_object_proto(context.intrinsics());

    let methods: &[(&str, NativeFunction, usize)] = &[
        ("log", NativeFunction::from_fn_ptr(log_impl), 1),
        ("debug", NativeFunction::from_fn_ptr(log_impl), 1),
        ("info", NativeFunction::from_fn_ptr(log_impl), 1),
        ("warn", NativeFunction::from_fn_ptr(warn_impl), 1),
        ("error", NativeFunction::from_fn_ptr(error_impl), 1),
        ("trace", NativeFunction::from_fn_ptr(trace_impl), 1),
        ("assert", NativeFunction::from_fn_ptr(assert_impl), 1),
        ("time", NativeFunction::from_fn_ptr(time_impl), 1),
        ("timeEnd", NativeFunction::from_fn_ptr(time_end_impl), 1),
        ("timeLog", NativeFunction::from_fn_ptr(time_log_impl), 1),
        ("count", NativeFunction::from_fn_ptr(count_impl), 1),
        (
            "countReset",
            NativeFunction::from_fn_ptr(count_reset_impl),
            1,
        ),
        ("clear", NativeFunction::from_fn_ptr(clear_impl), 1),
        ("table", NativeFunction::from_fn_ptr(table_impl), 1),
        ("group", NativeFunction::from_fn_ptr(group_impl), 1),
        ("groupEnd", NativeFunction::from_fn_ptr(group_end_impl), 1),
        (
            "groupCollapsed",
            NativeFunction::from_fn_ptr(group_collapsed_impl),
            1,
        ),
    ];

    for &(name, ref func, len) in methods {
        let f: JsValue = FunctionObjectBuilder::new(context.realm(), func.clone())
            .name(js_string!(name))
            .length(len)
            .build()
            .into();
        console.set(js_string!(name), f, false, context)?;
    }

    context.register_global_property(js_string!("console"), console, Attribute::all())?;

    Ok(())
}
