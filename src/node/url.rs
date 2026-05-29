use boa_engine::{Context, Module, Source};

fn url_js_source() -> String {
    String::from(
        r#"
function _fileURLToPath(url) {
  if (url instanceof _URL) {
    if (url.protocol !== "file:") throw new TypeError("The URL must be of scheme file");
    return decodeURIComponent(url.pathname);
  }
  var parsed = new _URL(url);
  if (parsed.protocol !== "file:") throw new TypeError("The URL must be of scheme file");
  return decodeURIComponent(parsed.pathname);
}

function _pathToFileURL(path) {
  if (typeof path !== "string") throw new TypeError("path must be a string");
  var resolved = path.startsWith("/") ? path : "/" + path;
  return new _URL("file://" + resolved);
}

var _URL = globalThis.URL || (typeof URL !== "undefined" ? URL : null);
var _URLSearchParams = globalThis.URLSearchParams || (typeof URLSearchParams !== "undefined" ? URLSearchParams : null);

var _url = { URL: _URL, URLSearchParams: _URLSearchParams, fileURLToPath: _fileURLToPath, pathToFileURL: _pathToFileURL };
export { _URL as URL, _URLSearchParams as URLSearchParams, _fileURLToPath as fileURLToPath, _pathToFileURL as pathToFileURL };
export default _url;
"#,
    )
}

pub fn create_node_url_module(context: &mut Context) -> Result<Module, String> {
    let js = url_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:url 模块失败: {e}"))
}
