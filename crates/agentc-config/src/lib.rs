// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_config;

pub mod config;
pub mod de;
pub mod errors;
pub mod node;
pub mod path;
pub mod secret;
pub mod traits;

pub mod prelude {
    pub use crate::config::*;
    pub use crate::de::*;
    pub use crate::errors::*;
    pub use crate::node::*;
    pub use crate::path::*;
    pub use crate::secret::*;
    pub use crate::traits::*;
}

pub mod macros {
    pub use crate::path;
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use crate::{
        config::Config,
        de::from_node,
        macros::path,
        node::ConfigNode,
        traits::{OsEnvSource, PrefixMapper},
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
        root.insert(&path!["a", "b"], ConfigNode::scalar("v"))
            .unwrap();

        assert_eq!(root.get(&path!["a", "b"]), Some(&ConfigNode::scalar("v")))
    }

    #[test]
    fn insert_into_null_vivifies_sequence() {
        let mut root = ConfigNode::Null;
        root.insert(&path![0], ConfigNode::scalar("first"))
            .unwrap();
        root.insert(&path![1], ConfigNode::scalar("second"))
            .unwrap();

        assert_eq!(root.get(&path![0]), Some(&ConfigNode::scalar("first")));
        assert_eq!(root.get(&path![1]), Some(&ConfigNode::scalar("second")));
    }

    #[test]
    fn insert_sparse_sequence_fills_nulls() {
        let mut root = ConfigNode::Null;
        root.insert(&path![2], ConfigNode::scalar("third"))
            .unwrap();

        assert_eq!(root.get(&path![0]), Some(&ConfigNode::Null));
        assert_eq!(root.get(&path![1]), Some(&ConfigNode::Null));
        assert_eq!(root.get(&path![2]), Some(&ConfigNode::scalar("third")));
    }

    #[test]
    fn insert_conflict_returns_error() {
        let mut root = ConfigNode::sequence(vec![ConfigNode::scalar("a")]);

        assert!(
            root.insert(&path!["x"], ConfigNode::scalar("v"))
                .is_err()
        )
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
        assert_eq!(from_node::<u32>(&ConfigNode::json(json!(42))).unwrap(), 42)
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
            from_node::<DbConfig>(&ConfigNode::json(json!({
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
            from_node::<Vec<String>>(&ConfigNode::json(json!(["x", "y", "z"]))).unwrap(),
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
    async fn prefix_mapper_with_field_mappings() {
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
                    .field(path!["name"], "MY_APP_NAME")
                    .field(path!["friends"], "MY_FRIENDS"),
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
            .mapper(PrefixMapper::new("AGENTC", "__").field(path!["params"], "MY_APP_PARAMS"))
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
}
