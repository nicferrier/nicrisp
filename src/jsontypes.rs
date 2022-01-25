use serde_json;

pub fn display(data: &serde_json::Value) -> String {
    format!("{}", serde_json::to_string_pretty(data).unwrap())
}

// End
