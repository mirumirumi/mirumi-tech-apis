use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use serde_json::json;

pub struct AttributeValueItem(pub HashMap<String, AttributeValue>);

impl AttributeValueItem {
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
                .map(Self::serialize_attribute_value)
                .collect::<Vec<_>>()),
            _ => unimplemented!("No serializer for DynamoDB AttributeValue."),
        }
    }
}

impl Serialize for AttributeValueItem {
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

pub trait ListToVec<T> {
    fn list_to_vec(&self, key: &str) -> Vec<T>;
}

impl ListToVec<String> for AttributeValueItem {
    fn list_to_vec(&self, key: &str) -> Vec<String> {
        if let AttributeValue::L(items) = self.0.get(key).unwrap() {
            items
                .iter()
                .map(|item| item.as_s().unwrap().clone())
                .collect::<Vec<String>>()
        } else {
            panic!("`list_to_vec` function is not implemented for any type other than `AttributeValue::L` type.")
        }
    }
}

impl ListToVec<i64> for AttributeValueItem {
    fn list_to_vec(&self, key: &str) -> Vec<i64> {
        if let AttributeValue::L(items) = self.0.get(key).unwrap() {
            items
                .iter()
                .map(|item| item.as_n().unwrap().parse::<i64>().unwrap())
                .collect::<Vec<i64>>()
        } else {
            panic!("`list_to_vec` function is not implemented for any type other than `AttributeValue::L` type.")
        }
    }
}

impl ListToVec<bool> for AttributeValueItem {
    fn list_to_vec(&self, key: &str) -> Vec<bool> {
        if let AttributeValue::L(items) = self.0.get(key).unwrap() {
            items
                .iter()
                .map(|item| *item.as_bool().unwrap())
                .collect::<Vec<bool>>()
        } else {
            panic!("`list_to_vec` function is not implemented for any type other than `AttributeValue::L` type.")
        }
    }
}
