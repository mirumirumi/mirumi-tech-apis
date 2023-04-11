use aws_sdk_dynamodb::types::AttributeValue;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use serde_json::json;
use std::collections::HashMap;

pub struct AttributeValueMap(pub HashMap<String, AttributeValue>);

impl AttributeValueMap {
    fn serialize_attribute_value(value: &AttributeValue) -> serde_json::Value {
        match value {
            AttributeValue::S(s) => json!(s),
            AttributeValue::N(n) => json!(n),
            AttributeValue::Bool(b) => json!(b),
            AttributeValue::Null(_) => json!(null),
            AttributeValue::Ss(ss) => json!(ss),
            AttributeValue::Ns(ns) => json!(ns),
            AttributeValue::L(l) => json!(l
                .iter()
                .map(|item| Self::serialize_attribute_value(item))
                .collect::<Vec<_>>()),
            _ => unimplemented!("No serializer for DynamoDB AttributeValue."),
        }
    }
}

impl Serialize for AttributeValueMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, value) in &self.0 {
            let serialized = Self::serialize_attribute_value(value);
            map.serialize_entry(key, &serialized)?;
        }

        map.end()
    }
}
