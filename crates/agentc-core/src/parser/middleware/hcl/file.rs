use hcl::{Body, Expression};
use std::path::PathBuf;

use crate::parser::{
    errors::ParserError,
    middleware::{
        hcl::expression::{ExpressionPosition, ExpressionVisitor, VisitOrder},
        traits::FormatMiddleware,
    },
};

pub trait FileReader: Send + Sync {
    fn read(&self, path: &str) -> Result<String, ParserError>;
}

pub struct RootedFileReader {
    root: PathBuf,
}

impl RootedFileReader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl FileReader for RootedFileReader {
    fn read(&self, requested: &str) -> Result<String, ParserError> {
        let path = self.root.join(requested);

        std::fs::read_to_string(&path).map_err(|source| ParserError::Unexpected {
            message: format!(
                "file({requested:?}) could not read '{}'",
                path.display(),
            ),
            source: Some(Box::new(source)),
        })
    }
}

pub struct FileFunctionDeserialize<R: FileReader> {
    reader: R,
}

impl<R: FileReader> FileFunctionDeserialize<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: FileReader> FormatMiddleware<Body> for FileFunctionDeserialize<R> {
    fn apply(&self, input: Body) -> Result<Body, ParserError> {
        ExpressionVisitor::new(VisitOrder::Before, |expr, position| match expr {
            Expression::FuncCall(call)
                if call.name.name.as_str() == "file"
                    && position == ExpressionPosition::Value =>
            {
                match call.args.as_slice() {
                    [Expression::String(path)] => {
                        self.reader.read(path).map(Expression::String)
                    }
                    _ => Err(ParserError::InvalidExpression(
                        "file() requires one string path".to_string(),
                    )),
                }
            }
            expr => Ok(expr),
        })
        .visit_body(input)
    }
}

#[cfg(test)]
mod tests {
    use agentc_blocks::types::RuntimeValue;
    use hcl::{Attribute, FuncCall, ObjectKey, Structure, expr::Object};
    use serde::Deserialize;

    use super::*;
    use crate::parser::{
        SpecFormat,
        middleware::hcl::RuntimeFunctionDeserialize,
    };

    struct FixtureReader;

    impl FileReader for FixtureReader {
        fn read(&self, path: &str) -> Result<String, ParserError> {
            match path {
                "system.md" => Ok("You are helpful.\n".to_string()),
                "example.md" => Ok("Example response".to_string()),
                _ => Err(ParserError::InvalidExpression(path.to_string())),
            }
        }
    }

    #[derive(Deserialize)]
    struct Fixture {
        direct: String,
        nested: Vec<Nested>,
        ordinary: String,
        default: RuntimeValue<String>,
    }

    #[derive(Deserialize)]
    struct Nested {
        content: String,
    }

    #[test]
    fn file_values_are_materialized_before_manifest_types() {
        let value = SpecFormat::hcl()
            .with_hcl_deserialize_middleware(FileFunctionDeserialize::new(FixtureReader))
            .with_hcl_deserialize_middleware(RuntimeFunctionDeserialize)
            .deserialize_string::<Fixture>(
                r#"
direct = file("system.md")
nested = [{ content = file("example.md") }]
ordinary = "system.md"
default = runtime("DEFAULT", file("example.md"))
"#,
            )
            .expect("file expressions resolve");

        assert_eq!(value.direct, "You are helpful.\n");
        assert_eq!(value.nested[0].content, "Example response");
        assert_eq!(value.ordinary, "system.md");
        assert_eq!(
            value.default,
            RuntimeValue::default_runtime("DEFAULT", "Example response".to_string()),
        );
    }

    #[test]
    fn file_requires_one_string_path() {
        for expression in ["file()", "file(7)", "file(\"a\", \"b\")"] {
            let error = SpecFormat::hcl()
                .with_hcl_deserialize_middleware(FileFunctionDeserialize::new(FixtureReader))
                .deserialize_string::<serde_json::Value>(&format!("value = {expression}"))
                .expect_err("invalid call must fail");

            assert!(error.to_string().contains("file() requires one string path"));
        }
    }

    #[test]
    fn file_does_not_read_object_keys() {
        let mut object = Object::new();
        object.insert(
            ObjectKey::Expression(Expression::FuncCall(Box::new(
                FuncCall::builder("file")
                    .arg(Expression::String("key.md".to_string()))
                    .build(),
            ))),
            Expression::String("value".to_string()),
        );

        let body = FileFunctionDeserialize::new(FixtureReader)
            .apply(
                Body::builder()
                    .add_attribute(Attribute::new("value", Expression::Object(object)))
                    .build(),
            )
            .expect("object key remains unchanged");

        let Some(Structure::Attribute(attribute)) = body.into_iter().next() else {
            panic!("expected attribute");
        };
        let Expression::Object(object) = attribute.expr else {
            panic!("expected object expression");
        };

        assert!(matches!(
            object.keys().next(),
            Some(ObjectKey::Expression(Expression::FuncCall(_))),
        ));
    }
}
