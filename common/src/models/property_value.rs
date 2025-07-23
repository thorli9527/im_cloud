use anyhow::{Result, anyhow};
use chrono;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PropertyValue {
    pub property_name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeValue {
    String,
    Number,
    Boolean,
    Int32,
    Int64,
    Float,
    Double,
    Date,
    U32,
    U64,
}

impl TypeValue {
    pub fn parse_value(&self, value: &str) -> Result<serde_json::Value> {
        match self {
            TypeValue::String => Ok(serde_json::Value::String(value.to_string())),
            TypeValue::Number | TypeValue::Double => value
                .parse::<f64>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid double: {}", value)),
            TypeValue::Float => value
                .parse::<f32>()
                .map(|f| serde_json::Value::from(f as f64))
                .map_err(|_| anyhow!("Invalid float: {}", value)),
            TypeValue::Boolean => value
                .parse::<bool>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid boolean: {}", value)),
            TypeValue::Int32 => value
                .parse::<i32>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid int32: {}", value)),
            TypeValue::Int64 => value
                .parse::<i64>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid int64: {}", value)),
            TypeValue::U32 => value
                .parse::<u32>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid u32: {}", value)),
            TypeValue::U64 => value
                .parse::<u64>()
                .map(serde_json::Value::from)
                .map_err(|_| anyhow!("Invalid u64: {}", value)),
            TypeValue::Date => {
                if let Ok(ts) = value.parse::<i64>() {
                    Ok(serde_json::Value::from(ts))
                } else if let Ok(date) = chrono::DateTime::parse_from_rfc3339(value) {
                    Ok(serde_json::Value::String(date.to_rfc3339()))
                } else {
                    Err(anyhow!("Invalid date format: {}", value))
                }
            }
        }
    }
}
