// use proptest::array;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ObjectSchema {
    pub properties: HashMap<String, Schema>,
    pub required: Vec<String>,
    pub additional_properties: bool,
}

#[derive(Debug, Clone)]
pub struct ArraySchema {
    pub items: Option<Box<Schema>>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct StringSchema {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
}
#[derive(Debug, Clone)]
pub struct NumberSchema {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Schema {
    String(StringSchema),
    Number(NumberSchema),
    Boolean,
    Object(ObjectSchema),
    Array(ArraySchema),
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: Vec<PathSegment>,
    pub code: ErrorCode,
    pub message: String, // to change it back to normal just leave this and delete the rest
    pub expected: Option<Value>,
    pub received: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    InvalidType,
    MinLength,
    MaxLength,
    Min,
    Max,
    Required,
    MinItems,
    MaxItems,
    AdditionalProperty,
}

impl Schema {
    // Entry point - like z.string()
    pub fn string() -> StringBuilder {
        StringBuilder {
            min_length: None,
            max_length: None,
        }
    }

    pub fn number() -> NumberBuilder {
        NumberBuilder {
            min: None,
            max: None,
        }
    }

    pub fn object() -> ObjectBuilder {
        ObjectBuilder {
            properties: HashMap::new(),
            required: Vec::new(),
            additional_properties: true,
        }
    }

    pub fn array() -> ArrayBuilder {
        ArrayBuilder {
            items: None,
            min_items: None,
            max_items: None,
        }
    }
}

// The builder for strings
pub struct StringBuilder {
    min_length: Option<usize>,
    max_length: Option<usize>,
}

impl StringBuilder {
    // Fluent methods - like .min(5)
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    // Convert builder to final schema
    pub fn build(self) -> Schema {
        Schema::String(StringSchema {
            min_length: self.min_length,
            max_length: self.max_length,
        })
    }
}

pub struct NumberBuilder {
    min: Option<f64>,
    max: Option<f64>,
}

impl NumberBuilder {
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn build(self) -> Schema {
        Schema::Number(NumberSchema {
            min: self.min,
            max: self.max,
        })
    }
}

pub struct ObjectBuilder {
    properties: HashMap<String, Schema>,
    required: Vec<String>,
    additional_properties: bool,
}

impl ObjectBuilder {
    pub fn property(mut self, name: impl Into<String>, schema: Schema) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }
    pub fn required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    pub fn strict(mut self) -> Self {
        self.additional_properties = false;
        self
    }

    pub fn build(self) -> Schema {
        Schema::Object(ObjectSchema {
            properties: self.properties,
            required: self.required,
            additional_properties: self.additional_properties,
        })
    }
}

pub struct ArrayBuilder {
    items: Option<Box<Schema>>,
    min_items: Option<usize>,
    max_items: Option<usize>,
}

impl ArrayBuilder {
    pub fn items(mut self, schema: Schema) -> Self {
        self.items = Some(Box::new(schema));
        self
    }

    pub fn min_items(mut self, min: usize) -> Self {
        self.min_items = Some(min);
        self
    }

    pub fn max_items(mut self, max: usize) -> Self {
        self.max_items = Some(max);
        self
    }

    pub fn build(self) -> Schema {
        Schema::Array(ArraySchema {
            items: self.items,
            min_items: self.min_items,
            max_items: self.max_items,
        })
    }
}

fn validate_recursive(
    schema: &Schema,
    value: &Value,
    path: &mut Vec<PathSegment>,
    errors: &mut Vec<ValidationError>,
) {
    match (schema, value) {
        (Schema::String(string_schema), Value::String(s)) => {
            if let Some(min) = string_schema.min_length {
                if s.len() < min {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::MinLength,
                        message: format!("String must be at least {} characters", min),
                        expected: Some(json!({"min": min})),
                        received: Some(json!(s)),
                    });
                }
            }
            if let Some(max) = string_schema.max_length {
                if s.len() > max {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::MaxLength,
                        message: format!("String must be at most {} characters", max),
                        expected: Some(json!({"max": max})),
                        received: Some(json!(s)),
                    });
                }
            }
        }

        (Schema::Number(number_schema), Value::Number(n)) => {
            let num = n.as_f64().unwrap();
            if let Some(min) = number_schema.min {
                if num < min {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::Min,
                        message: format!("Number must be at least {}", min),
                        expected: Some(json!({"min": min})),
                        received: Some(json!(num)),
                    });
                }
            }
            if let Some(max) = number_schema.max {
                if num > max {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::Max,
                        message: format!("Number must be at most {}", max),
                        expected: Some(json!({"max": max})),
                        received: Some(json!(num)),
                    });
                }
            }
        }

        (Schema::Boolean, Value::Bool(_)) => {
            // Boolean always valid if type matches
        }

        (Schema::Object(object_schema), Value::Object(obj)) => {
            // Check required properties
            for required_key in &object_schema.required {
                if !obj.contains_key(required_key) {
                    path.push(PathSegment::Key(required_key.clone()));
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::Required,
                        message: format!("Required property '{}' is missing", required_key),
                        expected: None,
                        received: None,
                    });
                    path.pop();
                }
            }

            // Validate each property
            for (key, val) in obj {
                path.push(PathSegment::Key(key.clone()));

                if let Some(prop_schema) = object_schema.properties.get(key) {
                    validate_recursive(prop_schema, val, path, errors);
                } else if !object_schema.additional_properties {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::AdditionalProperty,
                        message: format!("Additional property '{}' is not allowed", key),
                        expected: None,
                        received: Some(val.clone()),
                    });
                }

                path.pop();
            }
        }

        (Schema::Array(array_schema), Value::Array(arr)) => {
            // Check min/max items
            if let Some(min) = array_schema.min_items {
                if arr.len() < min {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::MinItems,
                        message: format!("Array must have at least {} items", min),
                        expected: Some(json!({"minItems": min})),
                        received: Some(json!(arr.len())),
                    });
                }
            }

            if let Some(max) = array_schema.max_items {
                if arr.len() > max {
                    errors.push(ValidationError {
                        path: path.clone(),
                        code: ErrorCode::MaxItems,
                        message: format!("Array must have at most {} items", max),
                        expected: Some(json!({"maxItems": max})),
                        received: Some(json!(arr.len())),
                    });
                }
            }

            // Validate each item
            if let Some(item_schema) = &array_schema.items {
                for (i, item) in arr.iter().enumerate() {
                    path.push(PathSegment::Index(i));
                    validate_recursive(item_schema, item, path, errors);
                    path.pop();
                }
            }
        }

        _ => {
            errors.push(ValidationError {
                path: path.clone(),
                code: ErrorCode::InvalidType,
                message: format!(
                    "Expected {:?}, received {:?}",
                    schema_type_name(schema),
                    value_type_name(value)
                ),
                expected: None,
                received: Some(value.clone()),
            });
        }
    }
}

fn schema_type_name(schema: &Schema) -> &'static str {
    match schema {
        Schema::String(_) => "string",
        Schema::Number(_) => "number",
        Schema::Boolean => "boolean",
        Schema::Object(_) => "object",
        Schema::Array(_) => "array",
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::Null => "null",
    }
}

pub fn validate(schema: &Schema, value: &Value) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut path = Vec::new();

    validate_recursive(schema, value, &mut path, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fluent_api() {
        let schema = Schema::string().min_length(3).max_length(10).build();

        // This should work for now (we haven't implemented the constraints yet)
        assert!(validate(&schema, &json!("hello")).is_ok());
    }

    #[test]
    fn test_basic_validation() {
        let string_schema = Schema::string().build();
        let number_schema = Schema::number().build();
        let boolean_schema = Schema::Boolean;

        // Valid cases
        assert!(validate(&string_schema, &json!("hello")).is_ok());
        assert!(validate(&number_schema, &json!(42)).is_ok());
        assert!(validate(&boolean_schema, &json!(true)).is_ok());

        // Invalid cases
        assert!(validate(&number_schema, &json!("not a number")).is_err());
        assert!(validate(&string_schema, &json!(123)).is_err());
    }

    #[test]
    fn test_object_validation() {
        let schema = Schema::object()
            .property("name", Schema::string().min_length(1).build())
            .property("age", Schema::number().min(0.0).build())
            .required("name")
            .build();

        // Valid object
        let valid = json!({
            "name": "John",
            "age": 25
        });
        assert!(validate(&schema, &valid).is_ok());

        // Missing required field
        let missing = json!({
            "age": 25
        });
        let result = validate(&schema, &missing);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].message.contains("name"));

        // Invalid property type
        let invalid = json!({
            "name": "John",
            "age": "not a number"
        });
        assert!(validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_array_validation() {
        let schema = Schema::array()
            .items(Schema::string().min_length(1).build())
            .min_items(1)
            .max_items(5)
            .build();

        // Valid array
        assert!(validate(&schema, &json!(["hello", "world"])).is_ok());

        // Empty array (violates min_items)
        let result = validate(&schema, &json!([]));
        assert!(result.is_err());

        // Too many items
        let result = validate(&schema, &json!(["a", "b", "c", "d", "e", "f"]));
        assert!(result.is_err());

        // Invalid item (empty string violates min_length)
        let result = validate(&schema, &json!(["hello", ""]));
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_structures() {
        // Array of objects!
        let schema = Schema::array()
            .items(
                Schema::object()
                    .property("name", Schema::string().min_length(1).build())
                    .property("age", Schema::number().min(0.0).build())
                    .required("name")
                    .build(),
            )
            .build();

        let valid = json!([
            {"name": "John", "age": 25},
            {"name": "Jane", "age": 30}
        ]);
        assert!(validate(&schema, &valid).is_ok());

        // One object missing required field
        let invalid = json!([
            {"name": "John", "age": 25},
            {"age": 30}  // missing name
        ]);
        assert!(validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_multiple_errors() {
        let schema = Schema::object()
            .property("name", Schema::string().min_length(5).build())
            .property("age", Schema::number().min(18.0).build())
            .required("name")
            .required("age")
            .build();

        let data = json!({
            "name": "Jo",  // Too short (error 1)
            "age": 10.0    // Too young (error 2)
        });

        let result = validate(&schema, &data);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        println!("Found {} errors:", errors.len());
        for error in &errors {
            println!("  - {:?}: {}", error.code, error.message);
        }

        // We should have 2 errors now!
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_error_paths() {
        let schema = Schema::array()
            .items(
                Schema::object()
                    .property("email", Schema::string().min_length(5).build())
                    .required("email")
                    .build(),
            )
            .build();

        let data = json!([
            {"email": "good@example.com"},
            {"email": "bad"}  // Too short!
        ]);

        let result = validate(&schema, &data);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let error = &errors[0];

        // Path should be [1, "email"]
        assert_eq!(error.path.len(), 2);
        match &error.path[0] {
            PathSegment::Index(i) => assert_eq!(*i, 1),
            _ => panic!("Expected Index"),
        }
        match &error.path[1] {
            PathSegment::Key(k) => assert_eq!(k, "email"),
            _ => panic!("Expected Key"),
        }
    }
}
