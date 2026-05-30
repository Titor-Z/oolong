use crate::web::headers::JsHeaders;
use crate::web::response::JsResponse;
use boa_engine::object::builtins::{JsPromise, JsUint8Array};
use boa_engine::{
    Context, JsData, JsNativeError, JsResult, JsString, JsValue, boa_class, js_error, js_string,
};
use boa_gc::{Finalize, Trace};

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct JsRequest {
    #[unsafe_ignore_trace]
    body_data: Vec<u8>,
    method: JsString,
    url: JsString,
    headers: JsHeaders,
    body_used: bool,
}

impl JsRequest {
    pub fn new(method: JsString, url: JsString, headers: JsHeaders, body: Vec<u8>) -> Self {
        Self {
            body_data: body,
            method,
            url,
            headers,
            body_used: false,
        }
    }

    pub fn get_body(&self) -> &[u8] {
        &self.body_data
    }

    pub fn get_method(&self) -> &JsString {
        &self.method
    }

    pub fn get_url(&self) -> &JsString {
        &self.url
    }

    pub fn get_headers(&self) -> &JsHeaders {
        &self.headers
    }
}

#[boa_class(rename = "Request")]
impl JsRequest {
    #[boa(constructor)]
    pub fn constructor(input: JsValue, init: JsValue, context: &mut Context) -> JsResult<Self> {
        let (mut method, url, mut headers, body_data) = if let Some(obj) = input.as_object() {
            if let Some(existing) = obj.downcast_ref::<JsRequest>() {
                (
                    existing.method.clone(),
                    existing.url.clone(),
                    existing.headers.clone(),
                    existing.body_data.clone(),
                )
            } else {
                let url_str = input.to_string(context)?.to_std_string_escaped();
                (
                    js_string!("GET"),
                    JsString::from(url_str),
                    JsHeaders::new(),
                    Vec::new(),
                )
            }
        } else {
            let url_str = input.to_string(context)?.to_std_string_escaped();
            (
                js_string!("GET"),
                JsString::from(url_str),
                JsHeaders::new(),
                Vec::new(),
            )
        };

        let mut body = body_data;

        if let Some(init_obj) = init.as_object() {
            if let Ok(m) = init_obj.get(js_string!("method"), context)
                && let Some(s) = m.as_string()
            {
                let method_str = s.to_std_string_escaped().to_uppercase();
                if !matches!(
                    method_str.as_str(),
                    "GET"
                        | "HEAD"
                        | "POST"
                        | "PUT"
                        | "DELETE"
                        | "PATCH"
                        | "OPTIONS"
                        | "CONNECT"
                        | "TRACE"
                ) {
                    return Err(js_error!(RangeError: "Invalid HTTP method: {method_str}"));
                }
                method = JsString::from(method_str);
            }

            if let Ok(h) = init_obj.get(js_string!("headers"), context)
                && h.is_object()
            {
                headers = parse_init_headers(&h, context)?;
            }

            if let Ok(b) = init_obj.get(js_string!("body"), context)
                && !b.is_undefined()
            {
                body = extract_req_body(&b, context)?;
            }
        }

        Ok(Self {
            body_data: body,
            method,
            url,
            headers,
            body_used: false,
        })
    }

    #[boa(getter)]
    pub fn method(&self) -> JsString {
        self.method.clone()
    }

    #[boa(getter)]
    pub fn url(&self) -> JsString {
        self.url.clone()
    }

    #[boa(getter)]
    pub fn headers(&self) -> JsHeaders {
        self.headers.clone()
    }

    #[boa(getter)]
    pub fn body_used(&self) -> bool {
        self.body_used
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
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body_used: false,
        }
    }
}

fn parse_init_headers(val: &JsValue, context: &mut Context) -> JsResult<JsHeaders> {
    let obj = val
        .as_object()
        .ok_or_else(|| js_error!(TypeError: "Headers must be an object"))?;

    if let Some(h) = obj.downcast_ref::<JsHeaders>() {
        return Ok(h.clone());
    }

    let mut headers = JsHeaders::new();
    if let Ok(length) = obj
        .get(js_string!("length"), context)
        .and_then(|v| v.to_length(context))
        && length > 0
    {
        for i in 0..length {
            if let Ok(entry) = obj.get(i, context)
                && let Some(e_obj) = entry.as_object()
                && let Ok(k) = e_obj.get(0, context).and_then(|v| v.to_string(context))
                && let Ok(v) = e_obj.get(1, context).and_then(|v| v.to_string(context))
            {
                let _ = headers.append(k, v);
            }
        }
        return Ok(headers);
    }

    // Plain object
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

fn extract_req_body(body_val: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
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
        if let Some(req) = obj.downcast_ref::<JsRequest>() {
            return Ok(req.body_data.clone());
        }
        if let Some(resp) = obj.downcast_ref::<JsResponse>() {
            return Ok(resp.get_body().to_vec());
        }
    }

    Ok(body_val
        .to_string(context)?
        .to_std_string_escaped()
        .into_bytes())
}

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<JsRequest>()?;
    Ok(())
}
