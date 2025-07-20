use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Encode `serde_json::Value` to JSONB (MessagePack) binary.
pub fn to_jsonb(val: &Value) -> Result<Vec<u8>> {
    rmp_serde::to_vec(val).context("encode jsonb")
}

/// Decode JSONB binary into `serde_json::Value`.
pub fn from_jsonb(data: &[u8]) -> Result<Value> {
    rmp_serde::from_slice(data).context("decode jsonb")
}

/// Validate JSON value against JSON Schema. Returns `true` if valid.
pub fn validate_schema(instance: &Value, schema: &Value) -> Result<bool> {
    let compiled = jsonschema::JSONSchema::compile(schema)?;
    Ok(compiled.is_valid(instance))
}

/// Evaluate JSONPath query and return matching values.
pub fn jsonpath_query<'a>(val: &'a Value, path: &str) -> Result<Vec<&'a Value>> {
    let expr = jsonpath_lib::Compiled::compile(path).context("compile jsonpath")?;
    Ok(expr.select(val).context("exec jsonpath")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let v: Value = serde_json::json!({"name":"Alice","active":true});
        let b = to_jsonb(&v).unwrap();
        let v2 = from_jsonb(&b).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn path() {
        let v: Value = serde_json::json!({"name":"Bob","tags":[{"k":"role","v":"admin"}]});
        let res = jsonpath_query(&v, "$.tags[0].v").unwrap();
        assert_eq!(res[0], &serde_json::Value::String("admin".into()));
    }
} 