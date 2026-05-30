use crate::web::headers::JsHeaders;
use boa_engine::object::builtins::{JsPromise, JsUint8Array};
use boa_engine::{
    Context, JsData, JsNativeError, JsResult, JsString, JsValue, boa_class, js_error, js_string,
};
use boa_gc::{Finalize, Trace};

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct JsResponse {
    #[unsafe_ignore_trace]
    body_data: Vec<u8>,
    #[unsafe_ignore_trace]
    status: u16,
    status_text: JsString,
    headers: JsHeaders,
    url: JsString,
    body_used: bool,
}

impl JsResponse {
    pub fn from_parts(body: Vec<u8>, status: u16, headers: JsHeaders) -> Self {
        let st = status_text_str(status);
        Self {
            body_data: body,
            status,
            status_text: JsString::from(st),
            headers,
            url: js_string!(""),
            body_used: false,
        }
    }

    pub fn get_body(&self) -> &[u8] {
        &self.body_data
    }

    pub fn get_status(&self) -> u16 {
        self.status
    }

    pub fn get_headers(&self) -> &JsHeaders {
        &self.headers
    }
}

fn status_text_str(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn extract_body_bytes(body_val: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    if body_val.is_null() || body_val.is_undefined() {
        return Ok(Vec::new());
    }

    if let Some(s) = body_val.as_string() {
        return Ok(s.to_std_string_escaped().into_bytes());
    }

    if let Some(obj) = body_val.as_object() {
        if let Some(blob) = obj.downcast_ref::<crate::web::blob::Blob>() {
            return Ok(blob.bytes().to_vec());
        }
        if let Some(resp) = obj.downcast_ref::<JsResponse>() {
            return Ok(resp.body_data.clone());
        }
    }

    Ok(body_val
        .to_string(context)?
        .to_std_string_escaped()
        .into_bytes())
}

#[boa_class(rename = "Response")]
impl JsResponse {
    #[boa(constructor)]
    pub fn constructor(body: JsValue, options: JsValue, context: &mut Context) -> JsResult<Self> {
        let body_data = extract_body_bytes(&body, context)?;
        let mut status = 200u16;
        let mut status_text = None;
        let mut headers = JsHeaders::new();

        if let Some(opts_obj) = options.as_object() {
            if let Ok(s) = opts_obj.get(js_string!("status"), context)
                && let Some(n) = s.as_number()
            {
                status = n as u16;
            }
            if let Ok(st) = opts_obj.get(js_string!("statusText"), context)
                && let Some(s) = st.as_string()
            {
                status_text = Some(s.clone());
            }
            if let Ok(h) = opts_obj.get(js_string!("headers"), context)
                && h.is_object()
            {
                headers = parse_headers_from_js(&h, context)?;
            }
        }

        let status_text = status_text.unwrap_or_else(|| JsString::from(status_text_str(status)));

        Ok(Self {
            body_data,
            status,
            status_text,
            headers,
            url: js_string!(""),
            body_used: false,
        })
    }

    #[boa(static)]
    pub fn error() -> Self {
        Self {
            body_data: Vec::new(),
            status: 0,
            status_text: js_string!(""),
            headers: JsHeaders::new(),
            url: js_string!(""),
            body_used: false,
        }
    }

    #[boa(static)]
    pub fn redirect(url: JsString, status: Option<u16>) -> JsResult<Self> {
        let st = status.unwrap_or(302);
        if st != 301 && st != 302 && st != 307 && st != 308 {
            return Err(js_error!(RangeError: "Invalid redirect status code: {st}"));
        }
        Ok(Self {
            body_data: Vec::new(),
            status: st,
            status_text: JsString::from(status_text_str(st)),
            headers: JsHeaders::new(),
            url,
            body_used: false,
        })
    }

    #[boa(getter)]
    pub fn status(&self) -> u16 {
        self.status
    }

    #[boa(getter)]
    pub fn status_text(&self) -> JsString {
        self.status_text.clone()
    }

    #[boa(getter)]
    pub fn headers(&self) -> JsHeaders {
        self.headers.clone()
    }

    #[boa(getter)]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    #[boa(getter)]
    pub fn body_used(&self) -> bool {
        self.body_used
    }

    #[boa(getter)]
    pub fn url(&self) -> JsString {
        self.url.clone()
    }

    #[boa(getter)]
    pub fn r#type(&self) -> JsString {
        if self.status == 0 {
            js_string!("error")
        } else if !self.url.is_empty() {
            js_string!("basic")
        } else {
            js_string!("default")
        }
    }

    fn text(&self, context: &mut Context) -> JsPromise {
        let body = self.body_data.clone();
        JsPromise::from_async_fn(
            async move |_| {
                let text = String::from_utf8_lossy(&body);
                Ok(JsString::from(text).into())
            },
            context,
        )
    }

    fn bytes(&self, context: &mut Context) -> JsPromise {
        let body = self.body_data.clone();
        JsPromise::from_async_fn(
            async move |context| {
                Ok(
                    JsUint8Array::from_iter(body.iter().copied(), &mut context.borrow_mut())
                        .map(Into::into)
                        .unwrap_or(JsValue::undefined()),
                )
            },
            context,
        )
    }

    fn json(&self, context: &mut Context) -> JsPromise {
        let body = self.body_data.clone();
        JsPromise::from_async_fn(
            async move |context| {
                let json_str = String::from_utf8_lossy(&body);
                let json = serde_json::from_str::<serde_json::Value>(&json_str).map_err(|e| {
                    JsNativeError::syntax().with_message(format!("JSON parse error: {e}"))
                })?;
                JsValue::from_json(&json, &mut context.borrow_mut())
            },
            context,
        )
    }

    fn clone(&self) -> Self {
        Self {
            body_data: self.body_data.clone(),
            status: self.status,
            status_text: self.status_text.clone(),
            headers: self.headers.clone(),
            url: self.url.clone(),
            body_used: false,
        }
    }
}

fn parse_headers_from_js(val: &JsValue, context: &mut Context) -> JsResult<JsHeaders> {
    let obj = val
        .as_object()
        .ok_or_else(|| js_error!(TypeError: "Headers must be an object"))?;

    if let Some(h) = obj.downcast_ref::<JsHeaders>() {
        return Ok(h.clone());
    }

    if let Ok(length) = obj
        .get(js_string!("length"), context)
        .and_then(|v| v.to_length(context))
        && length > 0
    {
        let mut headers = JsHeaders::new();
        for i in 0..length {
            let entry = obj
                .get(i, context)
                .map_err(|_| js_error!(TypeError: "Cannot read header entry at {i}"))?;
            if let Some(e_obj) = entry.as_object() {
                let key = e_obj
                    .get(0, context)
                    .ok()
                    .and_then(|v| v.to_string(context).ok())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let val = e_obj
                    .get(1, context)
                    .ok()
                    .and_then(|v| v.to_string(context).ok())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let _ = headers.append(js_string!(key), js_string!(val));
            }
        }
        return Ok(headers);
    }

    let mut headers = JsHeaders::new();
    let keys = obj.own_property_keys(context)?;
    for key in keys {
        if let boa_engine::property::PropertyKey::String(s) = &key {
            let name = s.to_std_string_escaped();
            if let Ok(val) = obj.get(key, context)
                && let Ok(val_str) = val.to_string(context)
            {
                let _ = headers.append(js_string!(name), val_str);
            }
        }
    }

    Ok(headers)
}

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<JsResponse>()?;
    Ok(())
}
