use boa_engine::object::builtins::JsArray;
use boa_engine::property::PropertyKey;
use boa_engine::{
    Context, JsData, JsObject, JsResult, JsString, JsValue, boa_class, js_error, js_string,
};
use boa_gc::{Finalize, Trace};
use std::collections::HashMap;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct JsHeaders {
    #[unsafe_ignore_trace]
    fields: HashMap<String, Vec<String>>,
}

impl Default for JsHeaders {
    fn default() -> Self {
        Self::new()
    }
}

impl JsHeaders {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn from_map(map: HashMap<String, Vec<String>>) -> Self {
        Self { fields: map }
    }

    fn normalize(name: &str) -> String {
        name.to_lowercase()
    }

    fn is_forbidden(name: &str) -> bool {
        matches!(
            name,
            "set-cookie"
                | "set-cookie2"
                | "accept-charset"
                | "accept-encoding"
                | "access-control-request-headers"
                | "access-control-request-method"
                | "connection"
                | "content-length"
                | "cookie"
                | "cookie2"
                | "date"
                | "dnt"
                | "expect"
                | "host"
                | "keep-alive"
                | "origin"
                | "referer"
                | "te"
                | "trailer"
                | "transfer-encoding"
                | "upgrade"
                | "via"
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().flat_map(|(k, vals)| {
            let k: &str = k;
            vals.iter().map(move |v| (k, v.as_str()))
        })
    }
}

#[boa_class(rename = "Headers")]
#[boa(rename_all = "camelCase")]
impl JsHeaders {
    #[boa(constructor)]
    pub fn constructor(init: JsValue, context: &mut Context) -> JsResult<Self> {
        let mut headers = JsHeaders::new();

        if init.is_undefined() || init.is_null() {
            return Ok(headers);
        }

        let obj = init
            .as_object()
            .ok_or_else(|| js_error!(TypeError: "Headers constructor: init must be an object"))?;

        if let Some(other) = obj.downcast_ref::<JsHeaders>() {
            headers.fields = other.fields.clone();
            return Ok(headers);
        }

        let length = obj
            .get(js_string!("length"), context)
            .and_then(|v| v.to_length(context))
            .unwrap_or(0);

        if length > 0 {
            for i in 0..length {
                let entry = obj.get(i, context).map_err(
                    |_| js_error!(TypeError: "Headers constructor: cannot read entry at index {i}"),
                )?;
                let entry_obj = entry.as_object().ok_or_else(|| {
                    js_error!(TypeError: "Headers constructor: entry must be a [key, value] pair")
                })?;
                let key_val = entry_obj.get(0, context).map_err(
                    |_| js_error!(TypeError: "Headers constructor: cannot read key at index {i}"),
                )?;
                let val_val = entry_obj.get(1, context).map_err(
                    |_| js_error!(TypeError: "Headers constructor: cannot read value at index {i}"),
                )?;
                let key = key_val.to_string(context)?.to_std_string_escaped();
                let val = val_val.to_string(context)?.to_std_string_escaped();
                headers.append_internal(key, val)?;
            }
        } else {
            let keys = obj.own_property_keys(context)?;
            for key in keys {
                let key_str = match &key {
                    PropertyKey::String(s) => s.to_std_string_escaped(),
                    PropertyKey::Index(i) => i.get().to_string(),
                    PropertyKey::Symbol(_) => continue,
                };
                let val = obj.get(key, context).map_err(
                    |_| js_error!(TypeError: "Headers constructor: cannot read header '{key_str}'"),
                )?;
                let val_str = val.to_string(context)?.to_std_string_escaped();
                headers.append_internal(key_str, val_str)?;
            }
        }

        Ok(headers)
    }

    pub fn append(&mut self, name: JsString, value: JsString) -> JsResult<()> {
        self.append_internal(name.to_std_string_escaped(), value.to_std_string_escaped())
    }

    fn append_internal(&mut self, name: String, value: String) -> JsResult<()> {
        if name.is_empty() {
            return Err(js_error!(TypeError: "Header name cannot be empty"));
        }
        let key = Self::normalize(&name);
        if Self::is_forbidden(&key) {
            return Err(js_error!(TypeError: "Cannot set forbidden header '{name}'"));
        }
        self.fields.entry(key).or_default().push(value);
        Ok(())
    }

    pub fn delete(&mut self, name: JsString) -> JsResult<()> {
        let key = Self::normalize(&name.to_std_string_escaped());
        self.fields.remove(&key);
        Ok(())
    }

    pub fn get(&self, name: JsString, _context: &mut Context) -> JsResult<JsValue> {
        let key = Self::normalize(&name.to_std_string_escaped());
        match self.fields.get(&key) {
            Some(vals) => {
                let result = vals.join(", ");
                Ok(JsString::from(result).into())
            }
            None => Ok(JsValue::null()),
        }
    }

    pub fn get_set_cookie(&self) -> Vec<JsString> {
        self.fields
            .get("set-cookie")
            .map(|v| v.iter().map(|s| JsString::from(s.as_str())).collect())
            .unwrap_or_default()
    }

    pub fn has(&self, name: JsString) -> JsResult<bool> {
        let key = Self::normalize(&name.to_std_string_escaped());
        Ok(self.fields.contains_key(&key))
    }

    pub fn set(&mut self, name: JsString, value: JsString) -> JsResult<()> {
        let raw_name = name.to_std_string_escaped();
        let raw_value = value.to_std_string_escaped();
        if raw_name.is_empty() {
            return Err(js_error!(TypeError: "Header name cannot be empty"));
        }
        let key = Self::normalize(&raw_name);
        if Self::is_forbidden(&key) {
            return Err(js_error!(TypeError: "Cannot set forbidden header '{raw_name}'"));
        }
        self.fields.insert(key, vec![raw_value]);
        Ok(())
    }

    pub fn for_each(
        &self,
        callback: boa_engine::object::builtins::TypedJsFunction<(JsString, JsString, JsObject), ()>,
        context: &mut Context,
    ) -> JsResult<()> {
        let this: JsValue = JsObject::with_object_proto(context.intrinsics()).into();
        for (key, vals) in &self.fields {
            for val in vals {
                callback.call_with_this(
                    &this,
                    context,
                    (
                        JsString::from(val.as_str()),
                        JsString::from(key.as_str()),
                        JsObject::with_object_proto(context.intrinsics()),
                    ),
                )?;
            }
        }
        Ok(())
    }

    pub fn entries(&self, context: &mut Context) -> JsValue {
        let mut arr = Vec::new();
        for (k, vals) in &self.fields {
            for v in vals {
                arr.push(
                    JsArray::from_iter(
                        [
                            JsString::from(k.as_str()).into(),
                            JsString::from(v.as_str()).into(),
                        ],
                        context,
                    )
                    .into(),
                );
            }
        }
        JsArray::from_iter(arr, context).into()
    }

    pub fn keys(&self) -> Vec<JsString> {
        let mut seen = Vec::new();
        for key in self.fields.keys() {
            let js_key = JsString::from(key.as_str());
            if !seen.contains(&js_key) {
                seen.push(js_key);
            }
        }
        seen
    }

    pub fn values(&self) -> Vec<JsString> {
        self.fields
            .values()
            .flat_map(|v| v.iter().map(|s| JsString::from(s.as_str())))
            .collect()
    }
}

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<JsHeaders>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_lowercases() {
        assert_eq!(JsHeaders::normalize("Content-Type"), "content-type");
        assert_eq!(JsHeaders::normalize("CONTENT-TYPE"), "content-type");
        assert_eq!(JsHeaders::normalize("content-type"), "content-type");
        assert_eq!(JsHeaders::normalize("X-Custom-Header"), "x-custom-header");
    }

    #[test]
    fn test_normalize_empty() {
        assert_eq!(JsHeaders::normalize(""), "");
    }

    #[test]
    fn test_is_forbidden_set_cookie() {
        assert!(JsHeaders::is_forbidden("set-cookie"));
        assert!(JsHeaders::is_forbidden("set-cookie2"));
    }

    #[test]
    fn test_is_forbidden_accept_charset() {
        assert!(JsHeaders::is_forbidden("accept-charset"));
    }

    #[test]
    fn test_is_forbidden_content_length() {
        assert!(JsHeaders::is_forbidden("content-length"));
    }

    #[test]
    fn test_is_forbidden_host() {
        assert!(JsHeaders::is_forbidden("host"));
    }

    #[test]
    fn test_is_forbidden_cookie() {
        assert!(JsHeaders::is_forbidden("cookie"));
        assert!(JsHeaders::is_forbidden("cookie2"));
    }

    #[test]
    fn test_is_forbidden_connection() {
        assert!(JsHeaders::is_forbidden("connection"));
    }

    #[test]
    fn test_is_forbidden_allows_normal_headers() {
        assert!(!JsHeaders::is_forbidden("content-type"));
        assert!(!JsHeaders::is_forbidden("authorization"));
        assert!(!JsHeaders::is_forbidden("x-api-key"));
        assert!(!JsHeaders::is_forbidden("accept"));
        assert!(!JsHeaders::is_forbidden("user-agent"));
    }

    #[test]
    fn test_is_forbidden_case_sensitive() {
        assert!(JsHeaders::is_forbidden("set-cookie"));
        assert!(
            !JsHeaders::is_forbidden("Set-Cookie"),
            "is_forbidden does not normalize; caller should normalize first"
        );
    }

    #[test]
    fn test_from_map_and_iter() {
        let mut map = HashMap::new();
        map.insert("content-type".into(), vec!["text/html".into()]);
        map.insert("x-custom".into(), vec!["val1".into(), "val2".into()]);
        let headers = JsHeaders::from_map(map);
        let pairs: Vec<(&str, &str)> = headers.iter().collect();
        assert_eq!(pairs.len(), 3);
        assert!(pairs.contains(&("content-type", "text/html")));
        assert!(pairs.contains(&("x-custom", "val1")));
        assert!(pairs.contains(&("x-custom", "val2")));
    }
}
