// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{
        self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor,
        value::{
            BoolDeserializer, CharDeserializer, F32Deserializer, F64Deserializer,
            I64Deserializer, I128Deserializer, MapAccessDeserializer,
            SeqAccessDeserializer, StringDeserializer, U64Deserializer,
            U128Deserializer, UnitDeserializer,
        },
    },
    ser::SerializeMap,
};
use std::{fmt, marker::PhantomData};

const RUNTIME_VALUE_KEY: &str = "$runtime";

struct RuntimeValueDefault<T>(Option<T>);

impl<T> RuntimeValueDefault<T> {
    fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    fn into_option(self) -> Option<T> {
        self.0
    }
}

impl<T> Default for RuntimeValueDefault<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RuntimeValueDefault<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "E: Deserialize<'de>, T: Deserialize<'de>"), deny_unknown_fields)]
struct RuntimeValuePayload<E, T> {
    env: E,
    #[serde(default)]
    default: RuntimeValueDefault<T>,
    #[serde(default)]
    secret: bool,
}

impl<E, T> RuntimeValuePayload<E, T> {
    fn new(env: E, default: Option<T>, secret: bool) -> Self {
        Self {
            env,
            default: RuntimeValueDefault(default),
            secret,
        }
    }

    fn into_parts(self) -> (E, Option<T>, bool) {
        (self.env, self.default.into_option(), self.secret)
    }
}

impl<E: Serialize, T: Serialize> Serialize for RuntimeValuePayload<E, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(
            1 + usize::from(self.default.as_ref().is_some()) + usize::from(self.secret),
        ))?;

        map.serialize_entry("env", &self.env)?;

        if let Some(default) = self.default.as_ref() {
            map.serialize_entry("default", default)?;
        }

        if self.secret {
            map.serialize_entry("secret", &self.secret)?;
        }

        map.end()
    }
}

struct RuntimeValuePresent<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for RuntimeValuePresent<D> {
    type Error = D::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_enum(name, variants, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}

struct RuntimeValueVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> RuntimeValueVisitor<T> {
    fn constant<D: Deserializer<'de>>(deserializer: D) -> Result<RuntimeValue<T>, D::Error> {
        T::deserialize(RuntimeValuePresent(deserializer)).map(RuntimeValue::Constant)
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for RuntimeValueVisitor<T> {
    type Value = RuntimeValue<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a constant or a runtime() value")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let first = map.next_key::<String>()?;

        if first.as_deref() == Some(RUNTIME_VALUE_KEY) {
            let (env, default, secret) =
                RuntimeValueWire::<String, T>::from_marker_value(map)?.into_parts();

            return Ok(RuntimeValue::Runtime {
                env,
                default,
                secret,
            });
        }

        Self::constant(MapAccessDeserializer::new(RuntimeValueReplayMapAccess {
            first,
            inner: map,
        }))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Self::constant(SeqAccessDeserializer::new(seq))
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Self::constant(BoolDeserializer::<E>::new(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Self::constant(I64Deserializer::<E>::new(value))
    }

    fn visit_i128<E: de::Error>(self, value: i128) -> Result<Self::Value, E> {
        Self::constant(I128Deserializer::<E>::new(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Self::constant(U64Deserializer::<E>::new(value))
    }

    fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
        Self::constant(U128Deserializer::<E>::new(value))
    }

    fn visit_f32<E: de::Error>(self, value: f32) -> Result<Self::Value, E> {
        Self::constant(F32Deserializer::<E>::new(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Self::constant(F64Deserializer::<E>::new(value))
    }

    fn visit_char<E: de::Error>(self, value: char) -> Result<Self::Value, E> {
        Self::constant(CharDeserializer::<E>::new(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Self::constant(StringDeserializer::<E>::new(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Self::constant(StringDeserializer::<E>::new(value))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        T::deserialize(UnitDeserializer::<E>::new()).map(RuntimeValue::Constant)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.visit_unit()
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

struct RuntimeValueReplayMapAccess<A> {
    first: Option<String>,
    inner: A,
}

impl<'de, A: MapAccess<'de>> MapAccess<'de> for RuntimeValueReplayMapAccess<A> {
    type Error = A::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let key = match self.first.take() {
            Some(key) => key,
            None => match self.inner.next_key::<String>()? {
                Some(key) if key == RUNTIME_VALUE_KEY => {
                    return Err(de::Error::custom("`$runtime` must be the only key"));
                }
                Some(key) => key,
                None => return Ok(None),
            },
        };

        seed.deserialize(StringDeserializer::<A::Error>::new(key))
            .map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.next_value_seed(seed)
    }
}

struct RuntimeValueWireVisitor<E, T>(PhantomData<(E, T)>);

impl<'de, E: Deserialize<'de>, T: Deserialize<'de>> Visitor<'de>
    for RuntimeValueWireVisitor<E, T>
{
    type Value = RuntimeValueWire<E, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a runtime value marker")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        if map.next_key::<String>()?.as_deref() != Some(RUNTIME_VALUE_KEY) {
            return Err(de::Error::custom("expected a `$runtime` marker"));
        }

        RuntimeValueWire::from_marker_value(map)
    }
}

pub struct RuntimeValueWire<E, T> {
    payload: RuntimeValuePayload<E, T>,
}

impl<E, T> RuntimeValueWire<E, T> {
    pub fn new(env: E, default: Option<T>, secret: bool) -> Self {
        Self {
            payload: RuntimeValuePayload::new(env, default, secret),
        }
    }

    fn from_marker_value<'de, A>(mut map: A) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>,
        E: Deserialize<'de>,
        T: Deserialize<'de>,
    {
        let payload = map.next_value::<RuntimeValuePayload<E, T>>()?;

        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("`$runtime` must be the only key"));
        }

        Ok(Self { payload })
    }

    pub fn into_parts(self) -> (E, Option<T>, bool) {
        self.payload.into_parts()
    }
}

impl<E: Serialize, T: Serialize> Serialize for RuntimeValueWire<E, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;

        map.serialize_entry(RUNTIME_VALUE_KEY, &self.payload)?;

        map.end()
    }
}

impl<'de, E: Deserialize<'de>, T: Deserialize<'de>> Deserialize<'de> for RuntimeValueWire<E, T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RuntimeValueWireVisitor(PhantomData))
    }
}

/// A value that can either be a constant or determined at runtime from an environment variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue<T> {
    Constant(T),
    Runtime {
        env: String,
        default: Option<T>,
        secret: bool,
    },
}

impl<T> RuntimeValue<T> {
    pub fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    pub fn required_runtime(env: impl Into<String>) -> Self {
        Self::Runtime {
            env: env.into(),
            default: None,
            secret: false,
        }
    }

    pub fn default_runtime(env: impl Into<String>, default: T) -> Self {
        Self::Runtime {
            env: env.into(),
            default: Some(default),
            secret: false,
        }
    }

    pub fn secret_runtime(env: impl Into<String>) -> Self {
        Self::Runtime {
            env: env.into(),
            default: None,
            secret: true,
        }
    }

    pub fn secret_default_runtime(env: impl Into<String>, default: T) -> Self {
        Self::Runtime {
            env: env.into(),
            default: Some(default),
            secret: true,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant(_))
    }

    pub fn is_runtime(&self) -> bool {
        matches!(self, Self::Runtime { .. })
    }

    pub fn as_constant(&self) -> Option<&T> {
        match self {
            Self::Constant(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_runtime(&self) -> Option<(&str, Option<&T>, bool)> {
        match self {
            Self::Runtime { env, default, secret } => {
                Some((env.as_str(), default.as_ref(), *secret))
            }
            _ => None,
        }
    }

    pub fn default_value(&self) -> Option<&T> {
        match self {
            Self::Constant(value) => Some(value),
            Self::Runtime { default: Some(d), .. } => Some(d),
            _ => None,
        }
    }

    pub fn render(
        &self,
        constant_expr: impl Fn(&T) -> TokenStream,
        parse_expr: Option<TokenStream>,
    ) -> TokenStream {
        match self {
            Self::Constant(value) => constant_expr(value),
            Self::Runtime { env, default, .. } => {
                let parse = parse_expr.unwrap_or_else(|| quote! { v });

                match default {
                    Some(default) => {
                        let default_tokens = constant_expr(default);

                        quote! {
                            std::env::var(#env)
                                .map(|v| #parse)
                                .unwrap_or_else(|_| #default_tokens)
                        }
                    }
                    None => quote! {
                        std::env::var(#env)
                            .map(|v| #parse)
                            .expect(&format!("Environment variable {} is required but not set", #env))
                    },
                }
            }
        }
    }
}

impl<T: Default> Default for RuntimeValue<T> {
    fn default() -> Self {
        Self::Constant(T::default())
    }
}

impl<T: Serialize> Serialize for RuntimeValue<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Constant(value) => value.serialize(serializer),
            Self::Runtime { env, default, secret } => {
                RuntimeValueWire::new(env, default.as_ref(), *secret).serialize(serializer)
            }
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RuntimeValue<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RuntimeValueVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::{from_str, from_value, json, to_value};

    use super::{RuntimeValue, RuntimeValueWire};

    #[test]
    fn constants_keep_their_shapes() {
        assert_eq!(
            from_value::<RuntimeValue<String>>(json!("x")).unwrap(),
            RuntimeValue::constant("x".to_string())
        );
        assert_eq!(
            from_value::<RuntimeValue<Option<String>>>(json!(null)).unwrap(),
            RuntimeValue::constant(None)
        );
        assert_eq!(
            from_value::<RuntimeValue<Option<String>>>(json!("x")).unwrap(),
            RuntimeValue::constant(Some("x".to_string()))
        );
        assert_eq!(
            from_value::<RuntimeValue<Option<BTreeSet<String>>>>(json!(["GET"]))
                .unwrap(),
            RuntimeValue::constant(Some(BTreeSet::from(["GET".to_string()])))
        );
        assert_eq!(
            from_value::<RuntimeValue<Option<BTreeMap<String, String>>>>(
                json!({ "env": "production" })
            )
            .unwrap(),
            RuntimeValue::constant(Some(BTreeMap::from([(
                "env".to_string(),
                "production".to_string(),
            )])))
        );
    }

    #[test]
    fn marker_round_trips_and_preserves_null_default() {
        let value = RuntimeValue::secret_default_runtime("TOKEN", "fallback".to_string());

        assert_eq!(
            to_value(RuntimeValueWire::new(
                "TOKEN",
                Some("fallback"),
                true,
            ))
            .unwrap(),
            to_value(&value).unwrap()
        );

        assert_eq!(
            from_value::<RuntimeValueWire<String, Option<String>>>(
                json!({ "$runtime": { "env": "TOKEN", "default": null } }),
            )
            .unwrap()
            .into_parts(),
            ("TOKEN".to_string(), Some(None), false)
        );

        assert_eq!(
            to_value(&value).unwrap(),
            json!({ "$runtime": { "env": "TOKEN", "default": "fallback", "secret": true } })
        );
        assert_eq!(
            from_value::<RuntimeValue<String>>(to_value(&value).unwrap()).unwrap(),
            value
        );

        let null_default = RuntimeValue::default_runtime("USER_AGENT", None::<String>);

        assert_eq!(
            from_value::<RuntimeValue<Option<String>>>(to_value(&null_default).unwrap())
                .unwrap(),
            null_default
        );
        assert_eq!(
            from_value::<RuntimeValue<Option<String>>>(
                json!({ "$runtime": { "env": "USER_AGENT" } })
            )
            .unwrap(),
            RuntimeValue::required_runtime("USER_AGENT")
        );
    }

    #[test]
    fn rejects_marker_siblings_and_unknown_spec_fields() {
        for source in [
            r#"{"$runtime":{"env":"X"},"other":"y"}"#,
            r#"{"other":"y","$runtime":{"env":"X"}}"#,
        ] {
            let error = from_str::<RuntimeValue<BTreeMap<String, String>>>(source)
                .unwrap_err()
                .to_string();

            assert!(error.contains("must be the only key"), "{error}");
        }

        let error = from_value::<RuntimeValue<String>>(
            json!({ "$runtime": { "env": "X", "defualt": "x" } })
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("defualt"), "{error}");
    }

    #[test]
    fn inner_errors_are_not_replaced_by_variant_errors() {
        let error = from_value::<RuntimeValue<Vec<BTreeMap<String, String>>>>(
            json!([{ "port": [1] }])
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("expected a string"), "{error}");
        assert!(!error.contains("did not match any variant"), "{error}");
    }
}
