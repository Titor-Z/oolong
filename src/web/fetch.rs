use crate::web::headers::JsHeaders;
use crate::web::request::JsRequest;
use crate::web::response::JsResponse;
use boa_engine::class::Class;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{
    Context, JsArgs, JsNativeError, JsResult, JsValue, NativeFunction, js_string,
    object::FunctionObjectBuilder, property::Attribute,
};
use std::collections::HashMap;

fn to_reqwest_method(method: &str) -> reqwest::Method {
    match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "PATCH" => reqwest::Method::PATCH,
        "OPTIONS" => reqwest::Method::OPTIONS,
        _ => reqwest::Method::GET,
    }
}

fn headers_to_reqwest(headers: &JsHeaders) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (key, val) in headers.iter() {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes())
            && let Ok(value) = reqwest::header::HeaderValue::from_str(val)
        {
            map.append(name, value);
        }
    }
    map
}

fn headers_from_reqwest(headers: &reqwest::header::HeaderMap) -> JsHeaders {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers.iter() {
        map.entry(name.as_str().to_lowercase())
            .or_default()
            .push(value.to_str().unwrap_or("").to_string());
    }
    JsHeaders::from_map(map)
}

fn do_fetch(resource: JsValue, context: &mut Context) -> JsResult<JsValue> {
    let (method, url, req_headers, body) = if let Some(obj) = resource.as_object() {
        if let Some(req) = obj.downcast_ref::<JsRequest>() {
            let m = req.get_method().to_std_string_escaped();
            let u = req.get_url().to_std_string_escaped();
            let h = req.get_headers().clone();
            let b = req.get_body().to_vec();
            (m, u, h, if b.is_empty() { None } else { Some(b) })
        } else {
            let url_str = resource.to_string(context)?.to_std_string_escaped();
            (String::new(), url_str, JsHeaders::new(), None)
        }
    } else {
        let url_str = resource.to_string(context)?.to_std_string_escaped();
        (String::new(), url_str, JsHeaders::new(), None)
    };

    let req_method = to_reqwest_method(&method);
    let client = reqwest::blocking::Client::new();
    let mut req_builder = client.request(req_method, &url);

    if let Some(body_bytes) = &body {
        req_builder = req_builder.body(body_bytes.clone());
    }

    let req_headers_map = headers_to_reqwest(&req_headers);
    for (name, value) in req_headers_map.iter() {
        req_builder = req_builder.header(name.as_str(), value.to_str().unwrap_or(""));
    }

    let resp = req_builder
        .send()
        .map_err(|e| JsNativeError::typ().with_message(format!("fetch failed: {e}")))?;

    let status = resp.status().as_u16();
    let resp_headers = headers_from_reqwest(resp.headers());
    let resp_body = resp.bytes().map(|b| b.to_vec()).unwrap_or_default();

    let js_response = JsResponse::from_parts(resp_body, status, resp_headers);
    Ok(Class::from_data(js_response, context)?.into())
}

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    let fetch_fn = NativeFunction::from_fn_ptr(|_, args, context| {
        let resource = args.get_or_undefined(0).clone();
        let result = do_fetch(resource, context)?;
        Ok(JsPromise::resolve(result, context).into())
    });

    let fetch_obj = FunctionObjectBuilder::new(context.realm(), fetch_fn)
        .name(js_string!("fetch"))
        .length(1)
        .build();

    context.register_global_property(js_string!("fetch"), fetch_obj, Attribute::all())?;
    Ok(())
}
