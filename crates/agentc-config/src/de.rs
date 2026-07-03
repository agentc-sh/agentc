// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use serde::{
    Deserializer,
    de::{
        self, DeserializeOwned, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
        Visitor,
    },
};
use serde_json::Value;

use crate::{
    errors::ConfigError,
    node::ConfigNode,
    path::{Path, Segment},
};

pub fn from_node<T: DeserializeOwned>(node: &ConfigNode) -> Result<T, ConfigError> {
    T::deserialize(ConfigDeserializer::new(node.clone(), Path::new()))
}

macro_rules! impl_deserialize_number {
    ($method:ident, $visit:ident, $type:ty) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            match self.node {
                ConfigNode::Scalar(ref scalar) => {
                    visitor.$visit(
                        self.parse_scalar(&scalar, stringify!($type), |s| s.parse::<$type>())?,
                    )
                }
                ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
                ConfigNode::Null => Err(ConfigError::missing_required(self.path)),
                other => {
                    Err(ConfigError::type_mismatch(self.path, stringify!($type), other.kind()))
                }
            }
        }
    };
}

pub struct ConfigDeserializer {
    node: ConfigNode,
    path: Path,
}

impl ConfigDeserializer {
    pub fn new(node: ConfigNode, path: Path) -> Self {
        Self { node, path }
    }

    fn parse_scalar<T, E, F>(
        &self,
        s: &str,
        type_name: &'static str,
        f: F,
    ) -> Result<T, ConfigError>
    where
        F: FnOnce(&str) -> Result<T, E>,
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        f(s).map_err(|e| ConfigError::parse_failure(self.path.clone(), s, type_name, e))
    }

    fn delegate_json<'a, V: Visitor<'a>>(
        &self,
        value: &Value,
        visitor: V,
    ) -> Result<V::Value, ConfigError> {
        value
            .clone()
            .into_deserializer()
            .deserialize_any(visitor)
            .map_err(|e| {
                ConfigError::parse_failure(self.path.clone(), value.to_string(), "json", e)
            })
    }
}

impl<'de> Deserializer<'de> for ConfigDeserializer {
    type Error = ConfigError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Null => visitor.visit_none(),
            ConfigNode::Scalar(scalar) => visitor.visit_string(scalar),
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Map(map) => visitor.visit_map(MapDeserializer::new(map, self.path.clone())),
            ConfigNode::Sequence(seq) => {
                visitor.visit_seq(SequenceDeserializer::new(seq, self.path.clone()))
            }
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.node {
            ConfigNode::Null => visitor.visit_none(),
            ConfigNode::Scalar(scalar) if scalar.is_empty() => visitor.visit_none(),
            ConfigNode::Json(Value::Null) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Scalar(scalar) => {
                visitor.visit_bool(match scalar.to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => {
                        return Err(ConfigError::parse_failure(
                            self.path.clone(),
                            &scalar,
                            "bool",
                            format!("invalid boolean value: {}", scalar),
                        ));
                    }
                })
            }
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Null => Err(ConfigError::missing_required(self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "bool", other.kind())),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Scalar(scalar) => visitor.visit_string(scalar),
            ConfigNode::Json(Value::String(value)) => visitor.visit_string(value),
            ConfigNode::Json(ref value) => visitor.visit_string(value.to_string()),
            ConfigNode::Null => Err(ConfigError::missing_required(self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "string", other.kind())),
        }
    }

    impl_deserialize_number!(deserialize_i8, visit_i8, i8);
    impl_deserialize_number!(deserialize_i16, visit_i16, i16);
    impl_deserialize_number!(deserialize_i32, visit_i32, i32);
    impl_deserialize_number!(deserialize_i64, visit_i64, i64);
    impl_deserialize_number!(deserialize_u8, visit_u8, u8);
    impl_deserialize_number!(deserialize_u16, visit_u16, u16);
    impl_deserialize_number!(deserialize_u32, visit_u32, u32);
    impl_deserialize_number!(deserialize_u64, visit_u64, u64);
    impl_deserialize_number!(deserialize_f32, visit_f32, f32);
    impl_deserialize_number!(deserialize_f64, visit_f64, f64);

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Scalar(scalar) => {
                let mut chars = scalar.chars();

                match (chars.next(), chars.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(ConfigError::parse_failure(
                        self.path.clone(),
                        &scalar,
                        "char",
                        format!("expected a single character, got: {}", scalar),
                    )),
                }
            }
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Null => Err(ConfigError::missing_required(self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "char", other.kind())),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Scalar(scalar) => visitor.visit_byte_buf(scalar.into_bytes()),
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            other => Err(ConfigError::type_mismatch(self.path, "bytes", other.kind())),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Sequence(sequence) => {
                visitor.visit_seq(SequenceDeserializer::new(sequence, self.path))
            }
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Null => visitor.visit_seq(SequenceDeserializer::new(vec![], self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "sequence", other.kind())),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Map(map) => visitor.visit_map(MapDeserializer::new(map, self.path)),
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Null => visitor.visit_map(MapDeserializer::new(BTreeMap::new(), self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "map", other.kind())),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.node {
            ConfigNode::Scalar(scalar) => visitor
                .visit_enum(scalar.clone().into_deserializer())
                .map_err(|e: serde::de::value::Error| {
                    ConfigError::parse_failure(self.path.clone(), &scalar, "enum variant", e)
                }),
            ConfigNode::Map(map) if map.len() == 1 => {
                let (variant, value) = map.into_iter().next().unwrap();

                visitor.visit_enum(EnumDeserializer {
                    variant: variant.clone(),
                    node: value,
                    path: self.path.clone(),
                })
            }
            ConfigNode::Json(Value::String(value)) => visitor
                .visit_enum(value.clone().into_deserializer())
                .map_err(|e: serde::de::value::Error| {
                    ConfigError::parse_failure(self.path.clone(), value, "enum variant", e)
                }),
            ConfigNode::Json(ref value) => self.delegate_json(value, visitor),
            ConfigNode::Null => Err(ConfigError::missing_required(self.path)),
            other => Err(ConfigError::type_mismatch(self.path, "enum", other.kind())),
        }
    }
}

struct SequenceDeserializer {
    iter: std::iter::Enumerate<std::vec::IntoIter<ConfigNode>>,
    path: Path,
}

impl SequenceDeserializer {
    fn new(seq: Vec<ConfigNode>, path: Path) -> Self {
        Self { iter: seq.into_iter().enumerate(), path }
    }
}

impl<'de> SeqAccess<'de> for SequenceDeserializer {
    type Error = ConfigError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        self.iter
            .next()
            .map(|(idx, node)| {
                seed.deserialize(ConfigDeserializer::new(
                    node,
                    self.path.child(Segment::index(idx)),
                ))
            })
            .transpose()
    }
}

struct MapDeserializer {
    iter: std::collections::btree_map::IntoIter<String, ConfigNode>,
    path: Path,
    pending_key: Option<String>,
    pending_value: Option<ConfigNode>,
}

impl MapDeserializer {
    fn new(map: BTreeMap<String, ConfigNode>, path: Path) -> Self {
        Self {
            iter: map.into_iter(),
            path,
            pending_key: None,
            pending_value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = ConfigError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            None => Ok(None),
            Some((key, value)) => {
                self.pending_key = Some(key.clone());
                self.pending_value = Some(value);

                seed.deserialize(key.into_deserializer())
                    .map(Some)
                    .map_err(|e: serde::de::value::Error| {
                        ConfigError::parse_failure(
                            self.path.clone(),
                            self.pending_key
                                .as_deref()
                                .unwrap_or(""),
                            "map key",
                            e,
                        )
                    })
            }
        }
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        seed.deserialize(ConfigDeserializer::new(
            self.pending_value
                .take()
                .expect("next_value_seed called without pending value"),
            self.path.child(Segment::key(
                self.pending_key
                    .take()
                    .expect("next_value_seed called without pending key"),
            )),
        ))
    }
}

struct EnumDeserializer {
    variant: String,
    node: ConfigNode,
    path: Path,
}

impl<'de> EnumAccess<'de> for EnumDeserializer {
    type Error = ConfigError;
    type Variant = VariantDeserializer;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant_path = self
            .path
            .child(Segment::key(self.variant.clone()));
        Ok((
            seed.deserialize(self.variant.clone().into_deserializer())
                .map_err(|e: serde::de::value::Error| {
                    ConfigError::parse_failure(
                        variant_path.clone(),
                        &self.variant,
                        "enum variant",
                        e,
                    )
                })?,
            VariantDeserializer { node: self.node, path: variant_path },
        ))
    }
}

struct VariantDeserializer {
    node: ConfigNode,
    path: Path,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = ConfigError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(ConfigDeserializer::new(self.node, self.path))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Deserializer::deserialize_seq(ConfigDeserializer::new(self.node, self.path), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Deserializer::deserialize_map(ConfigDeserializer::new(self.node, self.path), visitor)
    }
}
