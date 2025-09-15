//! JSON/YAML schema parser

use serde_json::{Value as JsonValue};
use serde_yaml::{Value as YamlValue};
use anyhow::{Result, Context};
use std::path::PathBuf;

use crate::parser::model::*;

/// Parser for JSON and YAML schema definitions
#[derive(Debug)]
pub struct SchemaParser {}

impl SchemaParser {
    /// Create a new schema parser
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a JSON schema document
    pub fn parse_json(&self, content: &str) -> Result<Specification> {
        let mut spec = Specification::new(PathBuf::new());
        
        let value: JsonValue = serde_json::from_str(content)
            .context("Failed to parse JSON")?;
        
        // Check for JSON Schema format first
        if value.get("$schema").is_some() {
            self.parse_json_schema(&mut spec, &value)?;
        } else {
            // Extract data models from JSON schema
            if let Some(model) = self.extract_data_model_from_json(&value)? {
                spec.data_models.push(model);
            }
        }
        
        // Check for configuration or specification format
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                if key.contains("spec") || key.contains("config") {
                    self.extract_specifications_from_json(&mut spec, key, val)?;
                }
            }
        }
        
        Ok(spec)
    }

    /// Parse a YAML schema document
    pub fn parse_yaml(&self, content: &str) -> Result<Specification> {
        let mut spec = Specification::new(PathBuf::new());
        
        let value: YamlValue = serde_yaml::from_str(content)
            .context("Failed to parse YAML")?;
        
        // Convert YAML to JSON for uniform processing
        let json_str = serde_json::to_string(&value)?;
        let json_value: JsonValue = serde_json::from_str(&json_str)?;
        
        // Extract data models
        if let Some(model) = self.extract_data_model_from_json(&json_value)? {
            spec.data_models.push(model);
        }
        
        // Check for specific YAML patterns
        self.extract_yaml_specifications(&mut spec, &value)?;
        
        Ok(spec)
    }

    /// Parse JSON Schema format
    fn parse_json_schema(&self, spec: &mut Specification, value: &JsonValue) -> Result<()> {
        let title = value.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Schema");
        
        let description = value.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if let Some(properties) = value.get("properties").and_then(|v| v.as_object()) {
            let mut fields = Vec::new();
            
            let required_fields = value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            
            for (name, prop) in properties {
                let field = self.extract_field_from_property(name, prop, &required_fields)?;
                fields.push(field);
            }
            
            let model = DataModel {
                name: title.to_string(),
                fields,
                description: description.to_string(),
                json_schema: Some(value.clone()),
            };
            
            spec.data_models.push(model);
        }
        
        Ok(())
    }

    /// Extract a field from a JSON Schema property
    fn extract_field_from_property(
        &self,
        name: &str,
        property: &JsonValue,
        required_fields: &[String],
    ) -> Result<Field> {
        let data_type = property.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("any")
            .to_string();
        
        let description = property.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let default_value = property.get("default")
            .map(|v| v.to_string());
        
        let mut validation = Vec::new();
        
        // Extract validation rules
        if let Some(min) = property.get("minimum") {
            validation.push(format!("minimum: {}", min));
        }
        if let Some(max) = property.get("maximum") {
            validation.push(format!("maximum: {}", max));
        }
        if let Some(pattern) = property.get("pattern").and_then(|v| v.as_str()) {
            validation.push(format!("pattern: {}", pattern));
        }
        if let Some(enum_values) = property.get("enum").and_then(|v| v.as_array()) {
            let values: Vec<String> = enum_values.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect();
            validation.push(format!("enum: [{}]", values.join(", ")));
        }
        
        Ok(Field {
            name: name.to_string(),
            data_type,
            required: required_fields.contains(&name.to_string()),
            description,
            default_value,
            validation,
        })
    }

    /// Extract data model from JSON value
    fn extract_data_model_from_json(&self, value: &JsonValue) -> Result<Option<DataModel>> {
        if !value.is_object() {
            return Ok(None);
        }
        
        let obj = value.as_object().unwrap();
        
        // Check if this looks like a data model definition
        if obj.is_empty() {
            return Ok(None);
        }
        
        let mut fields = Vec::new();
        
        for (key, val) in obj {
            // Skip metadata fields
            if key.starts_with('$') || key.starts_with('_') {
                continue;
            }
            
            let field = Field {
                name: key.clone(),
                data_type: self.infer_type_from_value(val),
                required: true,
                description: String::new(),
                default_value: None,
                validation: Vec::new(),
            };
            
            fields.push(field);
        }
        
        if fields.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(DataModel {
            name: "DataModel".to_string(),
            fields,
            description: String::new(),
            json_schema: Some(value.clone()),
        }))
    }

    /// Infer data type from JSON value
    fn infer_type_from_value(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(_) => "boolean".to_string(),
            JsonValue::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    "integer".to_string()
                } else {
                    "number".to_string()
                }
            }
            JsonValue::String(_) => "string".to_string(),
            JsonValue::Array(_) => "array".to_string(),
            JsonValue::Object(_) => "object".to_string(),
        }
    }

    /// Extract specifications from JSON configuration
    fn extract_specifications_from_json(
        &self,
        spec: &mut Specification,
        key: &str,
        value: &JsonValue,
    ) -> Result<()> {
        // Extract requirements if present
        if key.contains("requirement") {
            if let Some(arr) = value.as_array() {
                for (idx, item) in arr.iter().enumerate() {
                    if let Some(text) = item.as_str() {
                        let req = Requirement::new(
                            format!("REQ-{:03}", idx + 1),
                            text.to_string(),
                        );
                        spec.requirements.push(req);
                    }
                }
            }
        }
        
        // Extract constraints if present
        if key.contains("constraint") || key.contains("rule") {
            if let Some(text) = value.as_str() {
                let constraint = Constraint {
                    id: format!("CONST-{:03}", spec.constraints.len() + 1),
                    description: text.to_string(),
                    constraint_type: ConstraintType::BusinessRule,
                    rule: text.to_string(),
                };
                spec.constraints.push(constraint);
            }
        }
        
        Ok(())
    }

    /// Extract specifications from YAML value
    fn extract_yaml_specifications(&self, spec: &mut Specification, value: &YamlValue) -> Result<()> {
        match value {
            YamlValue::Mapping(map) => {
                for (key, val) in map {
                    if let Some(key_str) = key.as_str() {
                        // Check for requirements section
                        if key_str.to_lowercase().contains("requirement") {
                            self.extract_requirements_from_yaml(spec, val)?;
                        }
                        
                        // Check for API definitions
                        if key_str.to_lowercase().contains("api") || 
                           key_str.to_lowercase().contains("endpoint") {
                            self.extract_api_from_yaml(spec, val)?;
                        }
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    /// Extract requirements from YAML value
    fn extract_requirements_from_yaml(&self, spec: &mut Specification, value: &YamlValue) -> Result<()> {
        match value {
            YamlValue::Sequence(seq) => {
                for (idx, item) in seq.iter().enumerate() {
                    if let Some(text) = item.as_str() {
                        let req = Requirement::new(
                            format!("REQ-{:03}", spec.requirements.len() + 1),
                            text.to_string(),
                        );
                        spec.requirements.push(req);
                    }
                }
            }
            YamlValue::String(text) => {
                let req = Requirement::new(
                    format!("REQ-{:03}", spec.requirements.len() + 1),
                    text.clone(),
                );
                spec.requirements.push(req);
            }
            _ => {}
        }
        
        Ok(())
    }

    /// Extract API definition from YAML value
    fn extract_api_from_yaml(&self, spec: &mut Specification, value: &YamlValue) -> Result<()> {
        if let YamlValue::Mapping(map) = value {
            for (key, val) in map {
                if let Some(endpoint) = key.as_str() {
                    // Simple API extraction
                    let api = ApiDefinition {
                        endpoint: endpoint.to_string(),
                        method: HttpMethod::Get,
                        request_schema: None,
                        response_schema: None,
                        query_params: Vec::new(),
                        path_params: Vec::new(),
                        headers: Vec::new(),
                        description: val.as_str().unwrap_or("").to_string(),
                        examples: Vec::new(),
                    };
                    spec.apis.push(api);
                }
            }
        }
        
        Ok(())
    }
}

impl Default for SchemaParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_parser_creation() {
        let parser = SchemaParser::new();
        let _ = format!("{:?}", parser);
    }

    #[test]
    fn test_parse_json_schema() {
        let parser = SchemaParser::new();
        let schema = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "User",
            "type": "object",
            "properties": {
                "id": {
                    "type": "integer",
                    "description": "User ID"
                },
                "name": {
                    "type": "string",
                    "description": "User name"
                },
                "email": {
                    "type": "string",
                    "format": "email"
                }
            },
            "required": ["id", "email"]
        }"#;
        
        let spec = parser.parse_json(schema).unwrap();
        assert_eq!(spec.data_models.len(), 1);
        
        let model = &spec.data_models[0];
        assert_eq!(model.name, "User");
        assert_eq!(model.fields.len(), 3);
        
        let email_field = model.fields.iter()
            .find(|f| f.name == "email")
            .unwrap();
        assert!(email_field.required);
    }

    #[test]
    fn test_parse_yaml_config() {
        let parser = SchemaParser::new();
        let yaml = r#"
requirements:
  - User must be able to login
  - System should validate email format
  - Password must be encrypted
        "#;
        
        let spec = parser.parse_yaml(yaml).unwrap();
        assert_eq!(spec.requirements.len(), 3);
    }

    #[test]
    fn test_infer_types() {
        let parser = SchemaParser::new();
        
        assert_eq!(parser.infer_type_from_value(&JsonValue::Bool(true)), "boolean");
        assert_eq!(parser.infer_type_from_value(&JsonValue::from(42)), "integer");
        assert_eq!(parser.infer_type_from_value(&JsonValue::from(3.14)), "number");
        assert_eq!(parser.infer_type_from_value(&JsonValue::from("text")), "string");
        assert_eq!(parser.infer_type_from_value(&JsonValue::Array(vec![])), "array");
        assert_eq!(parser.infer_type_from_value(&JsonValue::Object(serde_json::Map::new())), "object");
    }
}