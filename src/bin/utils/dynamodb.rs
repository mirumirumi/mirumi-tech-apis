use aws_sdk_dynamodb::types::AttributeValue;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use serde_json::json;
use std::collections::HashMap;

pub struct AttributeValueMap(pub HashMap<String, AttributeValue>);

impl Serialize for AttributeValueMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, value) in &self.0 {
            let serialized = match value {
                AttributeValue::S(s) => json!(s),
                AttributeValue::N(n) => json!(n),
                AttributeValue::Bool(b) => json!(b),
                AttributeValue::Null(_) => json!(null),
                AttributeValue::Ss(ss) => json!(ss),
                AttributeValue::Ns(ns) => json!(ns),
                AttributeValue::L(l) => json!(l
                    .iter()
                    .map(|item| {
                        serde_json::to_value(AttributeValueMap(
                            vec![("item".to_string(), item.clone())]
                                .into_iter()
                                .collect(),
                        ))
                        .unwrap()
                    })
                    .collect::<Vec<serde_json::Value>>()),
                _ => unimplemented!("No serializer for DynamoDB AttributeValue."),
            };
            map.serialize_entry(key, &serialized)?;
        }

        map.end()
    }
}
