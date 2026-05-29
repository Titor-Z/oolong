use boa_engine::{
    Context, JsResult, JsValue, Module, NativeFunction, Source,
    js_string, object::FunctionObjectBuilder,
};

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(js_string!(name))
        .length(len)
        .build()
        .into()
}

fn register_tty_native(ctx: &mut Context) -> Result<(), String> {
    let isatty_fn = make_fn(
        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let fd = args
                .first()
                .and_then(|v| v.as_number())
                .map(|n| n as i32)
                .unwrap_or(0);
            let result = unsafe { libc::isatty(fd) != 0 };
            Ok(JsValue::from(result))
        },
        "_isatty",
        1,
        ctx,
    );

    let _ = ctx.register_global_property(
        js_string!("_isatty"),
        isatty_fn,
        boa_engine::property::Attribute::all(),
    );

    Ok(())
}

fn tty_js_source() -> String {
    String::from(
        r#"
function _isatty(fd) {
  return typeof globalThis._isatty === "function" ? globalThis._isatty(fd) : false;
}

function _WriteStream(fd) {
  this.fd = fd;
  this.isTTY = true;
  this.isRaw = false;
  this.columns = 80;
  this.rows = 24;
}
_WriteStream.prototype.getWindowSize = function() { return [this.columns, this.rows]; };
_WriteStream.prototype.setRawMode = function(mode) { this.isRaw = !!mode; return this; };

function _ReadStream(fd) {
  this.fd = fd;
  this.isTTY = true;
  this.isRaw = false;
}
_ReadStream.prototype.setRawMode = function(mode) { this.isRaw = !!mode; return this; };

var _tty = {
  isatty: _isatty,
  WriteStream: _WriteStream,
  ReadStream: _ReadStream,
};
export { _isatty as isatty, _WriteStream as WriteStream, _ReadStream as ReadStream };
export default _tty;
"#,
    )
}

pub fn create_node_tty_module(context: &mut Context) -> Result<Module, String> {
    register_tty_native(context)?;
    let js = tty_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:tty 模块失败: {e}"))
}
