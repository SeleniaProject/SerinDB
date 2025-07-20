use serde_json::Value;

/// Extract (<path>, <scalar>) pairs for GIN indexing.
pub fn extract_gin_keys(val: &Value, prefix: String, out: &mut Vec<(String, String)>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                let new_pref = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                extract_gin_keys(v, new_pref, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                extract_gin_keys(v, format!("{}[{}]", prefix, i), out);
            }
        }
        _ => {
            out.push((prefix, val.to_string()));
        }
    }
} 