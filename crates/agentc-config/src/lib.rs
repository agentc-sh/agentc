// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_config;

pub mod config;
pub mod de;
pub mod declarations;
pub mod errors;
pub mod node;
pub mod path;
pub mod secret;
pub mod traits;

pub mod prelude {
    pub use crate::config::*;
    pub use crate::de::*;
    pub use crate::declarations::*;
    pub use crate::errors::*;
    pub use crate::node::*;
    pub use crate::path::*;
    pub use crate::secret::*;
    pub use crate::traits::*;
}

pub mod macros {
    pub use crate::path;

    #[cfg(feature = "derive")]
    pub use agentc_config_macros::ConfigSchema;
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use crate::{
        config::Config,
        de::from_node,
        declarations::{Declarations, VariantCondition, VariantTag},
        errors::ConfigError,
        macros::path,
        node::{ConfigNode, Scalar, SequenceIndexes},
        path::{Path, Segment},
        traits::{ConfigSchema, MappedValue, MappedVar, OsEnvSource, PrefixMapper},
    };

    #[test]
    fn node_get_shallow_key() {
        assert_eq!(
            ConfigNode::map([("port", ConfigNode::scalar("8080"))]).get(&path!["port"]),
            Some(&ConfigNode::scalar("8080"))
        )
    }

    #[test]
    fn node_get_nested_key() {
        assert_eq!(
            ConfigNode::map([(
                "database",
                ConfigNode::map([("host", ConfigNode::scalar("localhost"))])
            )])
            .get(&path!["database", "host"]),
            Some(&ConfigNode::scalar("localhost"))
        )
    }

    #[test]
    fn node_get_index() {
        assert_eq!(
            ConfigNode::sequence(vec![
                ConfigNode::scalar("first"),
                ConfigNode::scalar("second"),
                ConfigNode::scalar("third")
            ])
            .get(&path![1]),
            Some(&ConfigNode::scalar("second"))
        )
    }

    #[test]
    fn node_get_missing_returns_none() {
        assert_eq!(
            ConfigNode::map([("port", ConfigNode::scalar("8080"))]).get(&path!["missing"]),
            None
        )
    }

    #[test]
    fn insert_into_null_vivifies_map() {
        let mut root = ConfigNode::Null;
        root.insert(&path!["a", "b"], ConfigNode::scalar("v"), &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(root.get(&path!["a", "b"]), Some(&ConfigNode::scalar("v")))
    }

    #[test]
    fn insert_into_null_vivifies_sequence() {
        let mut root = ConfigNode::Null;
        root.insert(&path![0], ConfigNode::scalar("first"), &mut SequenceIndexes::default())
            .unwrap();
        root.insert(&path![1], ConfigNode::scalar("second"), &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(root.get(&path![0]), Some(&ConfigNode::scalar("first")));
        assert_eq!(root.get(&path![1]), Some(&ConfigNode::scalar("second")));
    }

    #[test]
    fn insert_past_end_appends() {
        let mut root = ConfigNode::Null;
        root.insert(&path![2], ConfigNode::scalar("third"), &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(root.get(&path![0]), Some(&ConfigNode::scalar("third")));
        assert_eq!(root.get(&path![1]), None);
    }

    #[test]
    fn insert_conflict_returns_error() {
        let mut root = ConfigNode::sequence(vec![ConfigNode::scalar("a")]);

        assert!(
            root.insert(&path!["x"], ConfigNode::scalar("v"), &mut SequenceIndexes::default())
                .is_err()
        )
    }

    #[test]
    fn insert_existing_index_addresses_item() {
        let mut root = ConfigNode::sequence(vec![
            ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
            ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
        ]);

        root.insert(&path![1, "port"], ConfigNode::scalar("80"), &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![
                ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
                ConfigNode::map([
                    ("hostname", ConfigNode::scalar("b")),
                    ("port", ConfigNode::scalar("80")),
                ]),
            ])
        )
    }

    #[test]
    fn insert_past_end_index_appends_at_length() {
        let mut root = ConfigNode::sequence(vec![
            ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
            ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
        ]);

        root.insert(
            &path![5, "hostname"],
            ConfigNode::scalar("x"),
            &mut SequenceIndexes::default(),
        )
        .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![
                ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("x"))]),
            ])
        )
    }

    #[test]
    fn insert_same_past_end_index_reaches_appended_item() {
        let mut root = ConfigNode::sequence(vec![
            ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
            ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
        ]);
        let mut indexes = SequenceIndexes::default();

        root.insert(&path![5, "hostname"], ConfigNode::scalar("x"), &mut indexes)
            .unwrap();
        root.insert(&path![5, "port"], ConfigNode::scalar("80"), &mut indexes)
            .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![
                ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
                ConfigNode::map([
                    ("hostname", ConfigNode::scalar("x")),
                    ("port", ConfigNode::scalar("80")),
                ]),
            ])
        )
    }

    #[test]
    fn insert_distinct_past_end_indexes_append_in_order() {
        let mut root = ConfigNode::sequence(vec![
            ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
            ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
        ]);
        let mut indexes = SequenceIndexes::default();

        root.insert(&path![5, "hostname"], ConfigNode::scalar("x"), &mut indexes)
            .unwrap();
        root.insert(&path![9, "hostname"], ConfigNode::scalar("y"), &mut indexes)
            .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![
                ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("x"))]),
                ConfigNode::map([("hostname", ConfigNode::scalar("y"))]),
            ])
        )
    }

    #[test]
    fn insert_existing_and_past_end_indexes() {
        let mut root = ConfigNode::sequence(vec![
            ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
            ConfigNode::map([("hostname", ConfigNode::scalar("b"))]),
        ]);
        let mut indexes = SequenceIndexes::default();

        root.insert(&path![1, "port"], ConfigNode::scalar("80"), &mut indexes)
            .unwrap();
        root.insert(&path![5, "hostname"], ConfigNode::scalar("x"), &mut indexes)
            .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![
                ConfigNode::map([("hostname", ConfigNode::scalar("a"))]),
                ConfigNode::map([
                    ("hostname", ConfigNode::scalar("b")),
                    ("port", ConfigNode::scalar("80")),
                ]),
                ConfigNode::map([("hostname", ConfigNode::scalar("x"))]),
            ])
        )
    }

    #[test]
    fn insert_into_empty_sequence_appends() {
        let mut root = ConfigNode::sequence(Vec::new());

        root.insert(
            &path![0, "hostname"],
            ConfigNode::scalar("x"),
            &mut SequenceIndexes::default(),
        )
        .unwrap();

        assert_eq!(
            root,
            ConfigNode::sequence(vec![ConfigNode::map([("hostname", ConfigNode::scalar("x"))])])
        )
    }

    #[test]
    fn remove_index_is_positional_unset() {
        let mut root = ConfigNode::sequence(vec![ConfigNode::scalar("a")]);

        assert!(matches!(
            root.remove(&path![0], &mut SequenceIndexes::default()),
            Err(ConfigError::PositionalUnset { .. })
        ))
    }

    #[test]
    fn insert_replaces_whole_value() {
        let mut root =
            ConfigNode::map([("allow", ConfigNode::sequence(vec![ConfigNode::scalar("a")]))]);

        root.insert(
            &path!["allow"],
            ConfigNode::scalar(r#"[{"hostname":"x"}]"#),
            &mut SequenceIndexes::default(),
        )
        .unwrap();

        assert_eq!(root.get(&path!["allow"]), Some(&ConfigNode::scalar(r#"[{"hostname":"x"}]"#)))
    }

    #[test]
    fn insert_below_raw_scalar_decodes_document() {
        let mut root = ConfigNode::map([("allow", ConfigNode::scalar(r#"[{"hostname":"a"}]"#))]);

        root.insert(
            &path!["allow", 0, "port"],
            ConfigNode::scalar("80"),
            &mut SequenceIndexes::default(),
        )
        .unwrap();

        assert_eq!(
            root.get(&path!["allow"]),
            Some(&ConfigNode::sequence(vec![ConfigNode::map([
                ("hostname", ConfigNode::Scalar(Scalar::String("a".into()))),
                ("port", ConfigNode::scalar("80")),
            ])]))
        )
    }

    #[test]
    fn insert_below_typed_scalar_is_merge_conflict() {
        let mut root = ConfigNode::from(json!({ "port": 80 }));

        assert!(matches!(
            root.insert(
                &path!["port", "value"],
                ConfigNode::scalar("80"),
                &mut SequenceIndexes::default(),
            ),
            Err(ConfigError::MergeConflict { .. })
        ))
    }

    #[test]
    fn insert_below_invalid_document_is_error() {
        let mut root = ConfigNode::map([("allow", ConfigNode::scalar("not json"))]);

        assert!(matches!(
            root.insert(
                &path!["allow", "port"],
                ConfigNode::scalar("80"),
                &mut SequenceIndexes::default(),
            ),
            Err(ConfigError::InvalidDocument { .. })
        ))
    }

    #[test]
    fn insert_merges_map_by_key() {
        let mut root = ConfigNode::map([(
            "database",
            ConfigNode::map([("host", ConfigNode::scalar("localhost"))]),
        )]);

        root.insert(
            &path!["database", "port"],
            ConfigNode::scalar("5432"),
            &mut SequenceIndexes::default(),
        )
        .unwrap();

        assert_eq!(
            root,
            ConfigNode::map([(
                "database",
                ConfigNode::map([
                    ("host", ConfigNode::scalar("localhost")),
                    ("port", ConfigNode::scalar("5432")),
                ]),
            )])
        )
    }

    #[test]
    fn remove_key_removes_entry() {
        let mut root = ConfigNode::map([
            ("host", ConfigNode::scalar("localhost")),
            ("port", ConfigNode::scalar("5432")),
        ]);

        root.remove(&path!["port"], &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(root, ConfigNode::map([("host", ConfigNode::scalar("localhost"))]))
    }

    #[test]
    fn remove_below_raw_scalar_decodes_document() {
        let mut root = ConfigNode::map([(
            "database",
            ConfigNode::scalar(r#"{"host":"localhost","port":"5432"}"#),
        )]);

        root.remove(&path!["database", "port"], &mut SequenceIndexes::default())
            .unwrap();

        assert_eq!(
            root.get(&path!["database"]),
            Some(&ConfigNode::from(json!({ "host": "localhost" })))
        )
    }

    #[test]
    fn decode_at_decodes_raw_object() {
        let mut root = ConfigNode::map([("pubsub", ConfigNode::scalar(r#"{"kind":"redis"}"#))]);

        root.decode_at(&path!["pubsub"], &Path::new())
            .unwrap();

        assert_eq!(root.get(&path!["pubsub"]), Some(&ConfigNode::from(json!({ "kind": "redis" }))))
    }

    #[test]
    fn decode_at_decodes_raw_value_with_path_below() {
        let mut root = ConfigNode::map([("pubsub", ConfigNode::scalar(r#"{"kind":"redis"}"#))]);

        root.decode_at(&path!["pubsub", "kind"], &Path::new())
            .unwrap();

        assert_eq!(
            root.get(&path!["pubsub", "kind"]),
            Some(&ConfigNode::Scalar(Scalar::String("redis".into())))
        )
    }

    #[test]
    fn decode_at_leaves_plain_text() {
        let mut root = ConfigNode::map([("pubsub", ConfigNode::scalar("memory"))]);

        root.decode_at(&path!["pubsub"], &Path::new())
            .unwrap();

        assert_eq!(root.get(&path!["pubsub"]), Some(&ConfigNode::scalar("memory")))
    }

    #[test]
    fn from_value_builds_typed_scalars() {
        assert_eq!(
            ConfigNode::from(json!({
                "enabled": true,
                "port": 80,
                "host": "localhost",
                "proxy": null,
            })),
            ConfigNode::map([
                ("enabled", ConfigNode::Scalar(Scalar::Bool(true))),
                ("port", ConfigNode::Scalar(Scalar::Number(80.into()))),
                ("host", ConfigNode::Scalar(Scalar::String("localhost".into()))),
                ("proxy", ConfigNode::Null),
            ])
        )
    }

    #[test]
    fn scalar_as_text_reads_every_scalar() {
        assert_eq!(Scalar::Raw("8080".into()).as_text(), "8080");
        assert_eq!(Scalar::String("8080".into()).as_text(), "8080");
        assert_eq!(Scalar::Number(8080.into()).as_text(), "8080");
        assert_eq!(Scalar::Bool(true).as_text(), "true");
    }

    #[test]
    fn node_as_text_reads_scalars_only() {
        assert_eq!(ConfigNode::scalar("8080").as_text(), Some("8080".into()));
        assert_eq!(ConfigNode::Null.as_text(), None);
    }

    #[test]
    fn deserialize_string() {
        assert_eq!(from_node::<String>(&ConfigNode::scalar("hello")).unwrap(), "hello")
    }

    #[test]
    fn deserialize_u16() {
        assert_eq!(from_node::<u16>(&ConfigNode::scalar("8080")).unwrap(), 8080)
    }

    #[test]
    fn deserialize_f64() {
        assert_eq!(from_node::<f64>(&ConfigNode::scalar("3.22")).unwrap(), 3.22)
    }

    #[test]
    fn deserialize_bool_true_variants() {
        for value in ["true", "1", "yes", "on"] {
            assert!(from_node::<bool>(&ConfigNode::scalar(value)).unwrap())
        }
    }

    #[test]
    fn deserialize_bool_false_variants() {
        for value in ["false", "0", "no", "off"] {
            assert!(!from_node::<bool>(&ConfigNode::scalar(value)).unwrap())
        }
    }

    #[test]
    fn deserialize_bool_invalid_returns_error() {
        assert!(from_node::<bool>(&ConfigNode::scalar("maybe")).is_err())
    }

    #[test]
    fn deserialize_char() {
        assert_eq!(from_node::<char>(&ConfigNode::scalar("x")).unwrap(), 'x')
    }

    #[test]
    fn deserialize_char_too_long_returns_error() {
        assert!(from_node::<char>(&ConfigNode::scalar("ab")).is_err())
    }

    #[test]
    fn deserialize_option_some() {
        assert_eq!(from_node::<Option<u16>>(&ConfigNode::scalar("42")).unwrap(), Some(42))
    }

    #[test]
    fn deserialize_option_none_on_null() {
        assert_eq!(from_node::<Option<u16>>(&ConfigNode::Null).unwrap(), None)
    }

    #[test]
    fn deserialize_option_none_on_empty_string() {
        assert_eq!(from_node::<Option<String>>(&ConfigNode::scalar("")).unwrap(), None)
    }

    #[test]
    fn deserialize_flat_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct DbConfig {
            host: String,
            port: u16,
            ssl: bool,
        }

        assert_eq!(
            from_node::<DbConfig>(&ConfigNode::map([
                ("host", ConfigNode::scalar("localhost")),
                ("port", ConfigNode::scalar("5432")),
                ("ssl", ConfigNode::scalar("true")),
            ]))
            .unwrap(),
            DbConfig {
                host: "localhost".into(),
                port: 5432,
                ssl: true
            }
        )
    }

    #[test]
    fn deserialize_nested_struct() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct DbConfig {
            host: String,
            port: u16,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct AppConfig {
            name: String,
            database: DbConfig,
        }

        assert_eq!(
            from_node::<AppConfig>(&ConfigNode::map([
                ("name", ConfigNode::scalar("myapp")),
                (
                    "database",
                    ConfigNode::map([
                        ("host", ConfigNode::scalar("db.local")),
                        ("port", ConfigNode::scalar("5432")),
                    ])
                ),
            ]))
            .unwrap(),
            AppConfig {
                name: "myapp".into(),
                database: DbConfig { host: "db.local".into(), port: 5432 },
            }
        )
    }

    #[test]
    fn deserialize_vec_of_strings() {
        assert_eq!(
            from_node::<Vec<String>>(&ConfigNode::sequence(vec![
                ConfigNode::scalar("a"),
                ConfigNode::scalar("b"),
                ConfigNode::scalar("c"),
            ]))
            .unwrap(),
            vec!["a", "b", "c"]
        )
    }

    #[test]
    fn deserialize_unit_enum_from_scalar() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum LogLevel {
            Debug,
            Info,
            Warn,
            Error,
        }

        assert_eq!(from_node::<LogLevel>(&ConfigNode::scalar("info")).unwrap(), LogLevel::Info)
    }

    #[test]
    fn deserialize_struct_enum_variant() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum DbKind {
            Sqlite { path: String },
            Postgres,
        }

        assert_eq!(
            from_node::<DbKind>(&ConfigNode::map([(
                "sqlite",
                ConfigNode::map([("path", ConfigNode::scalar("/tmp/db.sqlite3")),])
            ),]))
            .unwrap(),
            DbKind::Sqlite { path: "/tmp/db.sqlite3".into() }
        )
    }

    #[test]
    fn deserialize_newtype_enum_variant() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum DbKind {
            Redis(String),
            Postgres,
        }

        assert_eq!(
            from_node::<DbKind>(&ConfigNode::map([(
                "redis",
                ConfigNode::scalar("redis://localhost:6379")
            ),]))
            .unwrap(),
            DbKind::Redis("redis://localhost:6379".into())
        )
    }

    #[test]
    fn deserialize_json_scalar() {
        assert_eq!(from_node::<u32>(&ConfigNode::from(json!(42))).unwrap(), 42)
    }

    #[test]
    fn deserialize_json_object() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct DbConfig {
            host: String,
            port: u16,
            ssl: bool,
        }

        assert_eq!(
            from_node::<DbConfig>(&ConfigNode::from(json!({
                "host": "localhost",
                "port": 5432,
                "ssl": false,
            })))
            .unwrap(),
            DbConfig {
                host: "localhost".into(),
                port: 5432,
                ssl: false
            }
        )
    }

    #[test]
    fn deserialize_json_array() {
        assert_eq!(
            from_node::<Vec<String>>(&ConfigNode::from(json!(["x", "y", "z"]))).unwrap(),
            vec!["x", "y", "z"]
        )
    }

    #[test]
    fn missing_required_field_returns_error() {
        #[allow(unused)]
        #[derive(Deserialize)]
        struct DbConfig {
            host: String,
            port: u16,
        }

        assert!(
            from_node::<DbConfig>(&ConfigNode::map([("host", ConfigNode::scalar("localhost")),]))
                .is_err()
        )
    }

    #[test]
    fn deserialize_any_reads_raw_text_as_json() {
        for (raw, value) in [
            ("10", json!(10)),
            ("true", json!(true)),
            (r#""42""#, json!("42")),
            ("redis://localhost", json!("redis://localhost")),
            (r#"{"kind":"redis"}"#, json!({ "kind": "redis" })),
        ] {
            assert_eq!(from_node::<Value>(&ConfigNode::scalar(raw)).unwrap(), value)
        }
    }

    #[test]
    fn deserialize_internally_tagged_enum_with_usize_field() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "kind", rename_all = "lowercase")]
        enum Pubsub {
            Memory { capacity: usize },
            Redis { url: String },
        }

        assert_eq!(
            from_node::<Pubsub>(&ConfigNode::from(json!({ "kind": "memory", "capacity": 64 })))
                .unwrap(),
            Pubsub::Memory { capacity: 64 }
        );

        assert_eq!(
            from_node::<Pubsub>(&ConfigNode::map([
                ("kind", ConfigNode::scalar("memory")),
                ("capacity", ConfigNode::scalar("64")),
            ]))
            .unwrap(),
            Pubsub::Memory { capacity: 64 }
        )
    }

    #[test]
    fn deserialize_externally_tagged_enum_from_whole_value() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum Pubsub {
            Memory,
            Redis { url: String },
        }

        assert_eq!(
            from_node::<Pubsub>(&ConfigNode::scalar(r#"{"redis":{"url":"redis://localhost"}}"#))
                .unwrap(),
            Pubsub::Redis { url: "redis://localhost".into() }
        )
    }

    #[tokio::test]
    async fn prefix_mapper_indexed_list_of_objects() {
        unsafe {
            std::env::set_var("AGENTC__ITEMS__0__NAME", "alpha");
            std::env::set_var("AGENTC__ITEMS__0__VALUE", "1");
            std::env::set_var("AGENTC__ITEMS__1__NAME", "beta");
            std::env::set_var("AGENTC__ITEMS__1__VALUE", "2");
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Item {
            name: String,
            value: u32,
        }

        let config = Config::builder()
            .source(OsEnvSource)
            .mapper(PrefixMapper::new("AGENTC", "__"))
            .build()
            .await
            .unwrap();

        unsafe {
            std::env::remove_var("AGENTC__ITEMS__0__NAME");
            std::env::remove_var("AGENTC__ITEMS__0__VALUE");
            std::env::remove_var("AGENTC__ITEMS__1__NAME");
            std::env::remove_var("AGENTC__ITEMS__1__VALUE");
        }

        assert_eq!(
            config
                .get::<Vec<Item>>(path!["items"])
                .unwrap(),
            vec![
                Item { name: "alpha".into(), value: 1 },
                Item { name: "beta".into(), value: 2 },
            ]
        )
    }

    #[tokio::test]
    async fn prefix_mapper_with_alias_mappings() {
        unsafe {
            std::env::set_var("AGENTC__PORT", "8080");
            std::env::set_var("AGENTC__DEBUG", "true");
            std::env::set_var("MY_APP_NAME", "myapp");
            std::env::set_var("MY_APP_NAME_EXTRA", "ignored");
            std::env::set_var("MY_FRIENDS__0", "alice");
            std::env::set_var("MY_FRIENDS__1", "bob");
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct AppConfig {
            name: String,
            port: u16,
            debug: bool,
            friends: Vec<String>,
        }

        let config = Config::builder()
            .source(OsEnvSource)
            .mapper(
                PrefixMapper::new("AGENTC", "__")
                    .alias(path!["name"], "MY_APP_NAME")
                    .alias(path!["friends"], "MY_FRIENDS"),
            )
            .build()
            .await
            .unwrap();

        unsafe {
            std::env::remove_var("AGENTC__PORT");
            std::env::remove_var("AGENTC__DEBUG");
            std::env::remove_var("MY_APP_NAME");
            std::env::remove_var("MY_APP_NAME_EXTRA");
            std::env::remove_var("MY_FRIENDS__0");
            std::env::remove_var("MY_FRIENDS__1");
        }

        assert_eq!(
            config
                .try_deserialize::<AppConfig>()
                .unwrap(),
            AppConfig {
                name: "myapp".into(),
                port: 8080,
                debug: true,
                friends: vec!["alice".into(), "bob".into()],
            }
        )
    }

    #[tokio::test]
    async fn prefix_mapper_with_json_type() {
        unsafe {
            std::env::set_var("MY_APP_PARAMS__KEY1", "value1");
            std::env::set_var("MY_APP_PARAMS__KEY2", "value2");
            std::env::set_var("MY_APP_PARAMS__KEY3__0", "item1");
            std::env::set_var("MY_APP_PARAMS__KEY3__1", "item2");
            std::env::set_var("MY_APP_PARAMS__KEY4__SUBKEY", "subvalue");
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct AppConfig {
            params: serde_json::Value,
        }

        let config = Config::builder()
            .source(OsEnvSource)
            .mapper(PrefixMapper::new("AGENTC", "__").alias(path!["params"], "MY_APP_PARAMS"))
            .build()
            .await
            .unwrap();

        unsafe {
            std::env::remove_var("MY_APP_PARAMS__KEY1");
            std::env::remove_var("MY_APP_PARAMS__KEY2");
            std::env::remove_var("MY_APP_PARAMS__KEY3__0");
            std::env::remove_var("MY_APP_PARAMS__KEY3__1");
            std::env::remove_var("MY_APP_PARAMS__KEY4__SUBKEY");
        }

        assert_eq!(
            config
                .try_deserialize::<AppConfig>()
                .unwrap(),
            AppConfig {
                params: json!({
                    "key1": "value1",
                    "key2": "value2",
                    "key3": ["item1", "item2"],
                    "key4": { "subkey": "subvalue" },
                }),
            }
        )
    }

    #[test]
    fn variant_key_defaults_apply_only_when_condition_holds() {
        let declarations = |pubsub: Value| {
            Declarations::new()
                .default(path!["pubsub"], pubsub)
                .variant(
                    VariantCondition::new(path!["pubsub"], VariantTag::Key, "redis"),
                    |declarations| {
                        declarations.default(path!["pubsub", "redis", "url"], "redis://localhost")
                    },
                )
        };

        assert_eq!(
            declarations(json!({ "redis": {} }))
                .build(&[], &[])
                .unwrap()
                .get(&path!["pubsub", "redis", "url"]),
            Some(&ConfigNode::from(json!("redis://localhost")))
        );

        assert_eq!(
            declarations(json!({ "memory": {} }))
                .build(&[], &[])
                .unwrap()
                .get(&path!["pubsub", "redis", "url"]),
            None
        )
    }

    #[test]
    fn variant_field_defaults_apply_only_when_condition_holds() {
        let declarations = |kind: &str| {
            Declarations::new()
                .default(path!["database", "kind"], kind)
                .variant(
                    VariantCondition::new(
                        path!["database"],
                        VariantTag::Field("kind".into()),
                        "postgres",
                    ),
                    |declarations| declarations.default(path!["database", "port"], 5432),
                )
        };

        assert_eq!(
            declarations("postgres")
                .build(&[], &[])
                .unwrap()
                .get(&path!["database", "port"]),
            Some(&ConfigNode::from(json!(5432)))
        );

        assert_eq!(
            declarations("sqlite")
                .build(&[], &[])
                .unwrap()
                .get(&path!["database", "port"]),
            None
        )
    }

    #[test]
    fn mapped_var_selecting_other_variant_applies_its_declarations() {
        let root = Declarations::new()
            .default(path!["database", "kind"], "sqlite")
            .variant(
                VariantCondition::new(
                    path!["database"],
                    VariantTag::Field("kind".into()),
                    "sqlite",
                ),
                |declarations| declarations.default(path!["database", "path"], "/tmp/agentc.db"),
            )
            .variant(
                VariantCondition::new(
                    path!["database"],
                    VariantTag::Field("kind".into()),
                    "postgres",
                ),
                |declarations| declarations.default(path!["database", "port"], 5432),
            )
            .build(
                &[MappedVar {
                    path: path!["database", "kind"],
                    value: MappedValue::Set(ConfigNode::scalar("postgres")),
                }],
                &[],
            )
            .unwrap();

        assert_eq!(root.get(&path!["database", "path"]), None);
        assert_eq!(root.get(&path!["database", "port"]), Some(&ConfigNode::from(json!(5432))))
    }

    #[test]
    fn nested_variant_applies_when_both_conditions_hold() {
        assert_eq!(
            Declarations::new()
                .default(
                    path!["database"],
                    json!({ "kind": "postgres", "tls": { "mode": "verify" } }),
                )
                .variant(
                    VariantCondition::new(
                        path!["database"],
                        VariantTag::Field("kind".into()),
                        "postgres",
                    ),
                    |declarations| {
                        declarations.variant(
                            VariantCondition::new(
                                path!["database", "tls"],
                                VariantTag::Field("mode".into()),
                                "verify",
                            ),
                            |declarations| {
                                declarations
                                    .default(path!["database", "tls", "ca"], "/etc/agentc/ca.pem")
                            },
                        )
                    },
                )
                .build(&[], &[])
                .unwrap()
                .get(&path!["database", "tls", "ca"]),
            Some(&ConfigNode::from(json!("/etc/agentc/ca.pem")))
        )
    }

    #[test]
    fn seal_applies_only_inside_selected_variant() {
        let declarations = Declarations::new()
            .default(path!["database", "kind"], "postgres")
            .variant(
                VariantCondition::new(
                    path!["database"],
                    VariantTag::Field("kind".into()),
                    "postgres",
                ),
                |declarations| declarations.constant(path!["database", "port"], 5432),
            )
            .variant(
                VariantCondition::new(
                    path!["database"],
                    VariantTag::Field("kind".into()),
                    "sqlite",
                ),
                |declarations| declarations.constant(path!["database", "path"], "/tmp/agentc.db"),
            );

        assert!(matches!(
            declarations.build(
                &[MappedVar {
                    path: path!["database", "port"],
                    value: MappedValue::Set(ConfigNode::scalar("6543")),
                }],
                &[],
            ),
            Err(ConfigError::Sealed { .. })
        ));

        assert_eq!(
            declarations
                .build(
                    &[MappedVar {
                        path: path!["database", "path"],
                        value: MappedValue::Set(ConfigNode::scalar("/var/agentc.db")),
                    }],
                    &[],
                )
                .unwrap()
                .get(&path!["database", "path"]),
            Some(&ConfigNode::scalar("/var/agentc.db"))
        )
    }

    #[test]
    fn whole_value_mapped_var_selects_variant() {
        assert_eq!(
            Declarations::new()
                .default(path!["pubsub"], json!({ "memory": {} }))
                .variant(
                    VariantCondition::new(path!["pubsub"], VariantTag::Key, "redis"),
                    |declarations| declarations.default(path!["cache", "backend"], "redis"),
                )
                .build(
                    &[MappedVar {
                        path: path!["pubsub"],
                        value: MappedValue::Set(ConfigNode::scalar(r#"{"redis":{}}"#)),
                    }],
                    &[],
                )
                .unwrap()
                .get(&path!["cache", "backend"]),
            Some(&ConfigNode::from(json!("redis")))
        )
    }

    #[test]
    fn declaration_deselecting_variant_is_unstable_variant() {
        assert!(matches!(
            Declarations::new()
                .default(path!["database", "kind"], "postgres")
                .variant(
                    VariantCondition::new(
                        path!["database"],
                        VariantTag::Field("kind".into()),
                        "postgres",
                    ),
                    |declarations| declarations.default(path!["database", "kind"], "sqlite"),
                )
                .build(&[], &[]),
            Err(ConfigError::UnstableVariant { .. })
        ))
    }

    #[tokio::test]
    async fn config_schema_builds_its_declarations() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct AppConfig {
            port: u16,
            name: String,
        }

        impl ConfigSchema for AppConfig {
            fn declare(declarations: Declarations, path: &Path) -> Declarations {
                declarations
                    .default(path.child(Segment::key("port")), 8080)
                    .constant(path.child(Segment::key("name")), "agentc")
            }
        }

        assert_eq!(
            AppConfig::schema()
                .build()
                .await
                .unwrap()
                .try_deserialize::<AppConfig>()
                .unwrap(),
            AppConfig { port: 8080, name: "agentc".into() }
        )
    }
}
