// use proptest::array;
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
    pub message: String,
    pub expected: Option<String>,
    pub received: Option<String>,
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

pub fn validate(schema: &Schema, value: &Value) -> Result<(), ValidationError> {
    match (schema, value) {
        (Schema::String(string_schema), Value::String(s)) => {
            if let Some(min) = string_schema.min_length {
                if s.len() < min {
                    return Err(ValidationError {
                        message: format!("String is shorter that min_length {}", min),
                    });
                }
            }
            if let Some(max) = string_schema.max_length {
                if s.len() > max {
                    return Err(ValidationError {
                        message: format!("String is longer than max_length {}", max),
                    });
                }
            }
            Ok(())
        }

        (Schema::Number(number_schema), Value::Number(n)) => {
            let num = n.as_f64().unwrap();
            if let Some(min) = number_schema.min {
                if num < min {
                    return Err(ValidationError {
                        message: format!("Number is less than min {}", min),
                    });
                }
            }
            if let Some(max) = number_schema.max {
                if num > max {
                    return Err(ValidationError {
                        message: format!("Number is greater than max {}", max),
                    });
                }
            }
            Ok(())
        }

        (Schema::Object(object_schema), Value::Object(obj)) => {
            // Check required properties
            for required_key in &object_schema.required {
                if !obj.contains_key(required_key) {
                    return Err(ValidationError {
                        message: format!("Missing required property: {}", required_key),
                    });
                }
            }

            // Validate each property
            for (key, value) in obj {
                if let Some(prop_schema) = object_schema.properties.get(key) {
                    validate(prop_schema, value)?
                } else if !object_schema.additional_properties {
                    return Err(ValidationError {
                        message: format!("Unexpected property: {}", key),
                    });
                }
            }
            Ok(())
        }

        (Schema::Array(array_schema), Value::Array(arr)) => {
            if let Some(min) = array_schema.min_items {
                if arr.len() < min {
                    return Err(ValidationError {
                        message: format!("Array has fewer items than min_items {}", min),
                    });
                }
            }

            if let Some(max) = array_schema.max_items {
                if arr.len() > max {
                    return Err(ValidationError {
                        message: format!("Array has more items than max_items {}", max),
                    });
                }
            }
            if let Some(item_schema) = &array_schema.items {
                for item in arr {
                    validate(item_schema, item)?
                }
            }
            Ok(())
        }

        // (Schema::Number, Value::Number(_)) => Ok(()),
        (Schema::Boolean, Value::Bool(_)) => Ok(()),
        _ => Err(ValidationError {
            message: format!("Expected {:?}, got {:?}", schema, value),
        }),
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
        assert!(result.unwrap_err().message.contains("name"));

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
}
