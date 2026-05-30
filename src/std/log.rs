#![allow(non_snake_case, clippy::collapsible_if)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::property::PropertyKey;

use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

// ── Level constants ──────────────────────────────────────────────────────────

const LEVELS: &[(i32, &str, &str)] = &[
    (10, "DBG", "DEBUG"),
    (20, "INF", "INFO"),
    (30, "WRN", "WARN"),
    (40, "ERR", "ERROR"),
    (50, "FTL", "FATAL"),
];
const LVL_VALS: [i32; 5] = [10, 20, 30, 40, 50];

fn lv_short(v: i32) -> &'static str {
    LEVELS
        .iter()
        .find(|(val, _, _)| *val == v)
        .map(|(_, s, _)| *s)
        .unwrap_or("???")
}
fn lv_name(v: i32) -> &'static str {
    LEVELS
        .iter()
        .find(|(val, _, _)| *val == v)
        .map(|(_, _, n)| *n)
        .unwrap_or("UNKNOWN")
}

fn lv_from_num(n: f64) -> i32 {
    LVL_VALS
        .into_iter()
        .find(|&l| (n - l as f64).abs() < 0.5)
        .unwrap_or(20)
}
// ── ANSI colors (Morandi palette) ────────────────────────────────────────────

const COLORS: &[(&str, &str)] = &[
    ("debug", "\x1b[38;2;184;176;160m"),
    ("info", "\x1b[38;2;143;170;143m"),
    ("warn", "\x1b[38;2;196;168;107m"),
    ("error", "\x1b[38;2;194;143;143m"),
    ("fatal", "\x1b[38;2;155;127;155m"),
];
const RST: &str = "\x1b[0m";
const DIM: &str = "\x1b[38;2;160;160;160m";
const KEY: &str = "\x1b[38;2;184;176;155m";

fn color(level: &str, overrides: &HashMap<String, String>) -> String {
    if let Some(c) = overrides.get(level) {
        return if c.starts_with('#') {
            hex_ansi(c)
        } else {
            c.clone()
        };
    }
    COLORS
        .iter()
        .find(|(k, _)| *k == level)
        .map(|(_, c)| c.to_string())
        .unwrap_or_default()
}

fn hex_ansi(h: &str) -> String {
    let h = h.trim_start_matches('#');
    if h.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        )
    {
        return format!("\x1b[38;2;{r};{g};{b}m");
    }
    String::new()
}

// ── Value formatting ─────────────────────────────────────────────────────────

fn ts_hhmm() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        % 86400;
    format!("{:02}:{:02}:{:02}", d / 3600, (d / 60) % 60, d % 60)
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn val_json(v: &JsValue, ctx: &mut Context) -> String {
    if let Some(s) = v.as_string() {
        return json_str(&s.to_std_string_escaped());
    }
    if let Some(n) = v.as_number() {
        return if n.fract() == 0.0 && n.is_finite() && n.abs() < (i64::MAX as f64) {
            format!("{}", n as i64)
        } else {
            n.to_string()
        };
    }
    if v.is_null() || v.is_undefined() {
        return "null".into();
    }
    if let Some(b) = v.as_boolean() {
        return b.to_string();
    }
    if let Some(obj) = v.as_object() {
        if let Ok(arr) = JsArray::from_object(obj.clone()) {
            let len = arr.length(ctx).unwrap_or(0) as usize;
            let items: Vec<JsValue> = (0..len)
                .filter_map(|i| arr.get(i as u32, ctx).ok())
                .collect();
            let items: Vec<String> = items.iter().map(|v| val_json(v, ctx)).collect();
            return format!("[{}]", items.join(","));
        }
        if let Ok(keys) = obj.own_property_keys(ctx) {
            if !keys.is_empty() {
                let pairs: Vec<String> = keys
                    .iter()
                    .filter_map(|k| {
                        if let Some(js) = jsstr_from_propkey(k) {
                            let ks = js.to_std_string_escaped();
                            obj.get(js, ctx)
                                .ok()
                                .map(|vv| format!("{}:{}", json_str(&ks), val_json(&vv, ctx)))
                        } else {
                            None
                        }
                    })
                    .collect();
                return format!("{{{}}}", pairs.join(","));
            }
        }
        if let Ok(s) = v.to_string(ctx) {
            return json_str(&s.to_std_string_escaped());
        }
    }
    json_str(&format!("{v:?}"))
}

fn val_text(v: &JsValue, ctx: &mut Context) -> String {
    if let Some(s) = v.as_string() {
        return s.to_std_string_escaped();
    }
    if let Some(n) = v.as_number() {
        return n.to_string();
    }
    if v.is_null() {
        return "null".into();
    }
    if v.is_undefined() {
        return "undefined".into();
    }
    if let Some(b) = v.as_boolean() {
        return b.to_string();
    }
    v.to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|_| "?".into())
}

fn jsstr_from_propkey(k: &PropertyKey) -> Option<JsString> {
    match k {
        PropertyKey::String(s) => Some(s.clone()),
        _ => None,
    }
}

// ── Global state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Cfg {
    format: String,
    map: HashMap<String, String>,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            format: "text".into(),
            map: HashMap::new(),
        }
    }
}

thread_local! {
    static CFG: RefCell<Cfg> = RefCell::new(Cfg { format: "text".into(), map: HashMap::new() });
    static REG: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
}

// ── Logger prototype ─────────────────────────────────────────────────────────

fn build_proto(ctx: &mut Context) -> JsObject {
    let p = JsObject::with_object_proto(ctx.intrinsics());
    let ms: &[(&str, NativeFunction, usize)] = &[
        ("debug", NativeFunction::from_fn_ptr(dbg_impl), 1),
        ("info", NativeFunction::from_fn_ptr(inf_impl), 1),
        ("warn", NativeFunction::from_fn_ptr(wrn_impl), 1),
        ("error", NativeFunction::from_fn_ptr(err_impl), 1),
        ("fatal", NativeFunction::from_fn_ptr(ftl_impl), 1),
        ("child", NativeFunction::from_fn_ptr(child_impl), 1),
    ];
    for &(n, ref func, len) in ms {
        let f: JsValue = FunctionObjectBuilder::new(ctx.realm(), func.clone())
            .name(JsString::from(n))
            .length(len)
            .build()
            .into();
        p.set(js_string!(n), f, false, ctx).ok();
    }
    p
}

fn get_proto(ctx: &mut Context) -> JsObject {
    REG.with(|r| {
        if let Some(v) = r.borrow().get("__proto__") {
            return v.as_object().unwrap().clone();
        }
        let p = build_proto(ctx);
        r.borrow_mut().insert("__proto__".into(), p.clone().into());
        p
    })
}

fn make_logger(name: &str, ctx: &mut Context) -> JsValue {
    let o = ObjectInitializer::new(ctx).build();
    o.set(
        js_string!("__proto__"),
        JsValue::from(get_proto(ctx)),
        false,
        ctx,
    )
    .ok();
    o.set(
        js_string!("name"),
        JsValue::from(js_string!(name)),
        false,
        ctx,
    )
    .ok();
    o.set(js_string!("level"), JsValue::from(20i32), false, ctx)
        .ok();
    o.set(
        js_string!("bindings"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    )
    .ok();
    o.into()
}

fn get_or_create(name: &str, ctx: &mut Context) -> JsValue {
    REG.with(|r| {
        if let Some(v) = r.borrow().get(name) {
            return v.clone();
        }
        let v = make_logger(name, ctx);
        r.borrow_mut().insert(name.to_string(), v.clone());
        v
    })
}

// ── Core log output ──────────────────────────────────────────────────────────

fn emit(inst: &JsValue, lvl: i32, args: &[JsValue], ctx: &mut Context) {
    let obj = match inst.as_object() {
        Some(o) => o,
        None => return,
    };
    let cur = obj
        .get(js_string!("level"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .unwrap_or(20.0) as i32;
    if cur > lvl {
        return;
    }
    let name = obj
        .get(js_string!("name"), ctx)
        .ok()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let msg = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let extra = if args.len() > 1 { &args[1..] } else { &[] };

    // Bindings from child logger
    let extra_bindings = obj.get(js_string!("bindings"), ctx).ok();

    let cfg = CFG.with(|c| c.borrow().clone());
    let ts = ts_hhmm();
    let short = lv_short(lvl);
    let ln = lv_name(lvl).to_lowercase();
    let clr = color(&ln, &cfg.map);

    if cfg.format == "json" {
        let mut out = format!(
            r#"{{"level":{},"time":"{}","name":{},"msg":{}"#,
            lvl,
            ts,
            json_str(&name),
            json_str(&msg)
        );
        if let Some(b) = extra_bindings {
            if let Some(bo) = b.as_object() {
                let ks: Vec<JsString> = bo
                    .own_property_keys(ctx)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(jsstr_from_propkey)
                    .collect();
                if !ks.is_empty() {
                    for k in &ks {
                        if let Ok(v) = bo.get(k.clone(), ctx) {
                            let ks = k.to_std_string_escaped();
                            out.push_str(&format!(",{}:{}", json_str(&ks), val_json(&v, ctx)));
                        }
                    }
                }
            }
        }
        for a in extra {
            if let Some(o) = a.as_object() {
                let ks: Vec<JsString> = o
                    .own_property_keys(ctx)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(jsstr_from_propkey)
                    .collect();
                if !ks.is_empty() {
                    for k in &ks {
                        if let Ok(v) = o.get(k.clone(), ctx) {
                            let ks = k.to_std_string_escaped();
                            out.push_str(&format!(",{}:{}", json_str(&ks), val_json(&v, ctx)));
                        }
                    }
                    continue;
                }
            }
            out.push_str(&format!(",{}", val_json(a, ctx)));
        }
        out.push('}');
        eprintln!("{out}");
    } else {
        let mut line = format!("{ts}  {clr}{short}{RST}  {DIM}[{name}]{RST}  {msg}");
        if let Some(b) = extra_bindings {
            if let Some(bo) = b.as_object() {
                let ks: Vec<JsString> = bo
                    .own_property_keys(ctx)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(jsstr_from_propkey)
                    .collect();
                for k in &ks {
                    if let Ok(v) = bo.get(k.clone(), ctx) {
                        let ks = k.to_std_string_escaped();
                        let vs = val_text(&v, ctx);
                        line.push_str(&format!("  {KEY}{ks}{RST}={vs}"));
                    }
                }
            }
        }
        for a in extra {
            if let Some(o) = a.as_object() {
                let ks: Vec<JsString> = o
                    .own_property_keys(ctx)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(jsstr_from_propkey)
                    .collect();
                if !ks.is_empty() {
                    for k in &ks {
                        if let Ok(v) = o.get(k.clone(), ctx) {
                            let ks = k.to_std_string_escaped();
                            let vs = val_text(&v, ctx);
                            line.push_str(&format!("  {KEY}{ks}{RST}={vs}"));
                        }
                    }
                    continue;
                }
            }
            line.push_str(&format!(" {}", val_text(a, ctx)));
        }
        eprintln!("{line}");
    }
}

// ── Level methods ────────────────────────────────────────────────────────────

fn dbg_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    emit(this, 10, args, ctx);
    Ok(JsValue::undefined())
}
fn inf_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    emit(this, 20, args, ctx);
    Ok(JsValue::undefined())
}
fn wrn_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    emit(this, 30, args, ctx);
    Ok(JsValue::undefined())
}
fn err_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    emit(this, 40, args, ctx);
    Ok(JsValue::undefined())
}
fn ftl_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    emit(this, 50, args, ctx);
    Ok(JsValue::undefined())
}

// ── child() ──────────────────────────────────────────────────────────────────

fn child_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Logger.child 必须在 Logger 实例上调用")
    })?;
    let name = obj
        .get(js_string!("name"), ctx)
        .ok()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let level = obj
        .get(js_string!("level"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .unwrap_or(20.0) as i32;

    let child_obj = ObjectInitializer::new(ctx).build();
    child_obj
        .set(
            js_string!("__proto__"),
            JsValue::from(get_proto(ctx)),
            false,
            ctx,
        )
        .ok();
    child_obj
        .set(
            js_string!("name"),
            JsValue::from(js_string!(name)),
            false,
            ctx,
        )
        .ok();
    child_obj
        .set(js_string!("level"), JsValue::from(level), false, ctx)
        .ok();

    // Merge parent bindings + new bindings
    let merged = JsObject::with_object_proto(ctx.intrinsics());
    if let Ok(existing) = obj.get(js_string!("bindings"), ctx) {
        if let Some(eo) = existing.as_object() {
            let ks: Vec<JsString> = eo
                .own_property_keys(ctx)
                .unwrap_or_default()
                .iter()
                .filter_map(jsstr_from_propkey)
                .collect();
            for k in &ks {
                if let Ok(v) = eo.get(k.clone(), ctx) {
                    merged.set(k.clone(), v, false, ctx).ok();
                }
            }
        }
    }
    if let Some(bind_arg) = args.first() {
        if let Some(bo) = bind_arg.as_object() {
            let ks: Vec<JsString> = bo
                .own_property_keys(ctx)
                .unwrap_or_default()
                .iter()
                .filter_map(jsstr_from_propkey)
                .collect();
            for k in &ks {
                if let Ok(v) = bo.get(k.clone(), ctx) {
                    merged.set(k.clone(), v, false, ctx).ok();
                }
            }
        }
    }
    child_obj
        .set(js_string!("bindings"), JsValue::from(merged), false, ctx)
        .ok();
    Ok(child_obj.into())
}

// ── Module functions ─────────────────────────────────────────────────────────

fn mod_dbg(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let logger = get_or_create("default", ctx);
    emit(&logger, 10, args, ctx);
    Ok(JsValue::undefined())
}
fn mod_inf(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let logger = get_or_create("default", ctx);
    emit(&logger, 20, args, ctx);
    Ok(JsValue::undefined())
}
fn mod_wrn(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let logger = get_or_create("default", ctx);
    emit(&logger, 30, args, ctx);
    Ok(JsValue::undefined())
}
fn mod_err(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let logger = get_or_create("default", ctx);
    emit(&logger, 40, args, ctx);
    Ok(JsValue::undefined())
}
fn mod_ftl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let logger = get_or_create("default", ctx);
    emit(&logger, 50, args, ctx);
    Ok(JsValue::undefined())
}

fn mod_get_logger(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());
    Ok(get_or_create(&name, ctx))
}

fn mod_setup(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if let Some(opts) = args.first().and_then(|v| v.as_object()) {
        let mut cfg = Cfg::default();

        if let Ok(f) = opts.get(js_string!("format"), ctx) {
            if let Some(s) = f.as_string() {
                if s.to_std_string_escaped() == "json" {
                    cfg.format = "json".into();
                }
            }
        }

        if let Ok(c) = opts.get(js_string!("colors"), ctx) {
            if let Some(co) = c.as_object() {
                let ks: Vec<JsString> = co
                    .own_property_keys(ctx)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(jsstr_from_propkey)
                    .collect();
                for k in &ks {
                    if let Ok(v) = co.get(k.clone(), ctx) {
                        if let Some(s) = v.as_string() {
                            cfg.map
                                .insert(k.to_std_string_escaped(), s.to_std_string_escaped());
                        }
                    }
                }
            }
        }

        CFG.with(|c| *c.borrow_mut() = cfg);
    }
    Ok(JsValue::undefined())
}

// ── Module export ────────────────────────────────────────────────────────────

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

pub fn create_log_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("Logger"),
        js_string!("LogLevel"),
        js_string!("getLogger"),
        js_string!("setup"),
        js_string!("debug"),
        js_string!("info"),
        js_string!("warn"),
        js_string!("error"),
        js_string!("fatal"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                // ── LogLevel enum ──
                let lvl_obj = JsObject::with_object_proto(ctx.intrinsics());
                lvl_obj
                    .set(js_string!("DEBUG"), JsValue::from(10), false, ctx)
                    .ok();
                lvl_obj
                    .set(js_string!("INFO"), JsValue::from(20), false, ctx)
                    .ok();
                lvl_obj
                    .set(js_string!("WARN"), JsValue::from(30), false, ctx)
                    .ok();
                lvl_obj
                    .set(js_string!("ERROR"), JsValue::from(40), false, ctx)
                    .ok();
                lvl_obj
                    .set(js_string!("FATAL"), JsValue::from(50), false, ctx)
                    .ok();
                m.set_export(&js_string!("LogLevel"), JsValue::from(lvl_obj.clone()))?;

                // ── Logger constructor ──
                let ctor = FunctionObjectBuilder::new(
                    ctx.realm(),
                    NativeFunction::from_fn_ptr(logger_ctor),
                )
                .name("Logger")
                .length(1)
                .constructor(true)
                .build();
                // Set up prototype
                {
                    let proto = get_proto(ctx);
                    ctor.set(
                        js_string!("prototype"),
                        JsValue::from(proto.clone()),
                        false,
                        ctx,
                    )
                    .ok();
                    proto
                        .set(
                            js_string!("constructor"),
                            JsValue::from(ctor.clone()),
                            false,
                            ctx,
                        )
                        .ok();
                }
                let ctor_val: JsValue = ctor.into();
                let lvl_val: JsValue = lvl_obj.clone().into();
                m.set_export(&js_string!("Logger"), ctor_val.clone())?;

                // ── Module functions ──
                let mut func_vals: Vec<(&str, JsValue)> = Vec::new();
                let funcs: &[(&str, NativeFunction, usize)] = &[
                    ("getLogger", NativeFunction::from_fn_ptr(mod_get_logger), 1),
                    ("setup", NativeFunction::from_fn_ptr(mod_setup), 1),
                    ("debug", NativeFunction::from_fn_ptr(mod_dbg), 1),
                    ("info", NativeFunction::from_fn_ptr(mod_inf), 1),
                    ("warn", NativeFunction::from_fn_ptr(mod_wrn), 1),
                    ("error", NativeFunction::from_fn_ptr(mod_err), 1),
                    ("fatal", NativeFunction::from_fn_ptr(mod_ftl), 1),
                ];
                for &(n, ref func, len) in funcs {
                    let fv = make_fn(func.clone(), n, len, ctx);
                    m.set_export(&js_string!(n), fv.clone())?;
                    func_vals.push((n, fv));
                }

                // ── default ──
                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj
                    .set(js_string!("Logger"), ctor_val, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("LogLevel"), lvl_val, false, ctx)
                    .ok();
                for (key, val) in &func_vals {
                    default_obj
                        .set(js_string!(*key), val.clone(), false, ctx)
                        .ok();
                }
                m.set_export(&js_string!("default"), default_obj.into())?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}

// ── Logger constructor (placeholder for now, returns a default logger) ──
fn logger_ctor(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "default".to_string());

    let obj = ObjectInitializer::new(ctx).build();
    obj.set(
        js_string!("__proto__"),
        JsValue::from(get_proto(ctx)),
        false,
        ctx,
    )
    .ok();
    obj.set(
        js_string!("name"),
        JsValue::from(js_string!(name.as_str())),
        false,
        ctx,
    )
    .ok();
    obj.set(js_string!("level"), JsValue::from(20i32), false, ctx)
        .ok();
    obj.set(
        js_string!("bindings"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    )
    .ok();

    if let Some(opts) = args.get(1).and_then(|v| v.as_object()) {
        if let Ok(lv) = opts.get(js_string!("level"), ctx) {
            if let Some(n) = lv.as_number() {
                let v = lv_from_num(n);
                obj.set(js_string!("level"), JsValue::from(v), false, ctx)
                    .ok();
            }
        }
    }

    Ok(obj.into())
}
