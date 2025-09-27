#![allow(unused)]
//! OpenAPI specification parser

use anyhow::{Context, Result};
use openapiv3::{OpenAPI, ReferenceOr, Schema as OpenApiSchema};
use std::path::PathBuf;

use crate::parser::model::*;

/// Parser for OpenAPI specifications
#[derive(Debug)]
pub struct OpenApiParser {}

impl OpenApiParser {
    /// Create a new OpenAPI parser
    pub fn new() -> Self {
        Self {}
    }

    /// Parse OpenAPI specification from JSON
    pub fn parse_json(&self, content: &str) -> Result<Specification> {
        let openapi: OpenAPI =
            serde_json::from_str(content).context("Failed to parse OpenAPI JSON")?;

        self.extract_from_openapi(openapi)
    }

    /// Parse OpenAPI specification from YAML
    pub fn parse_yaml(&self, content: &str) -> Result<Specification> {
        let openapi: OpenAPI =
            serde_yaml::from_str(content).context("Failed to parse OpenAPI YAML")?;

        self.extract_from_openapi(openapi)
    }

    /// Extract specification from OpenAPI document
    fn extract_from_openapi(&self, openapi: OpenAPI) -> Result<Specification> {
        let mut spec = Specification::new(PathBuf::new());

        // Extract API information
        for (path, path_item) in &openapi.paths.paths {
            let path_item = match path_item {
                ReferenceOr::Reference { .. } => continue,
                ReferenceOr::Item(item) => item,
            };

            // Process each HTTP method
            if let Some(operation) = &path_item.get {
                self.extract_api_operation(&mut spec, path, HttpMethod::Get, operation)?;
            }
            if let Some(operation) = &path_item.post {
                self.extract_api_operation(&mut spec, path, HttpMethod::Post, operation)?;
            }
            if let Some(operation) = &path_item.put {
                self.extract_api_operation(&mut spec, path, HttpMethod::Put, operation)?;
            }
            if let Some(operation) = &path_item.delete {
                self.extract_api_operation(&mut spec, path, HttpMethod::Delete, operation)?;
            }
            if let Some(operation) = &path_item.patch {
                self.extract_api_operation(&mut spec, path, HttpMethod::Patch, operation)?;
            }
        }

        // Extract schemas/data models
        if let Some(components) = &openapi.components {
            for (name, schema_ref) in &components.schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    if let Some(model) = self.extract_data_model(name, schema)? {
                        spec.data_models.push(model);
                    }
                }
            }
        }

        // Extract requirements from info section
        if let Some(description) = &openapi.info.description {
            self.extract_requirements_from_description(&mut spec, description);
        }

        Ok(spec)
    }

    /// Extract API operation details
    fn extract_api_operation(
        &self,
        spec: &mut Specification,
        path: &str,
        method: HttpMethod,
        operation: &openapiv3::Operation,
    ) -> Result<()> {
        let mut api = ApiDefinition {
            endpoint: path.to_string(),
            method,
            request_schema: None,
            response_schema: None,
            query_params: Vec::new(),
            path_params: Vec::new(),
            headers: Vec::new(),
            description: operation.summary.clone().unwrap_or_default(),
            examples: Vec::new(),
        };

        // Extract parameters
        for param_ref in &operation.parameters {
            match param_ref {
                ReferenceOr::Reference { .. } => continue,
                ReferenceOr::Item(param) => {
                    let param_data = match &param {
                        openapiv3::Parameter::Query { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Path { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Header { parameter_data, .. } => parameter_data,
                        openapiv3::Parameter::Cookie { .. } => continue,
                    };

                    let api_param = Parameter {
                        name: param_data.name.clone(),
                        data_type: "string".to_string(), // Simplified for now
                        required: param_data.required,
                        description: param_data.description.clone().unwrap_or_default(),
                        default_value: None,
                    };

                    match param {
                        openapiv3::Parameter::Query { .. } => api.query_params.push(api_param),
                        openapiv3::Parameter::Path { .. } => api.path_params.push(api_param),
                        openapiv3::Parameter::Header { .. } => api.headers.push(api_param),
                        _ => {}
                    }
                }
            }
        }

        // Extract request body schema
        if let Some(request_body_ref) = &operation.request_body {
            if let ReferenceOr::Item(request_body) = request_body_ref {
                for (media_type, media) in &request_body.content {
                    if media_type == "application/json" {
                        if let Some(schema_ref) = &media.schema {
                            if let ReferenceOr::Item(schema) = schema_ref {
                                api.request_schema = self.extract_data_model("Request", schema)?;
                            }
                        }
                    }
                }
            }
        }

        // Extract response schemas
        for (status_code, response_ref) in &operation.responses.responses {
            let is_success = match status_code {
                openapiv3::StatusCode::Code(code) => *code == 200 || *code == 201,
                openapiv3::StatusCode::Range(_) => false,
            };

            if is_success {
                if let ReferenceOr::Item(response) = response_ref {
                    for (media_type, media) in &response.content {
                        if media_type == "application/json" {
                            if let Some(schema_ref) = &media.schema {
                                if let ReferenceOr::Item(schema) = schema_ref {
                                    api.response_schema =
                                        self.extract_data_model("Response", schema)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add operation ID as a requirement if present
        if let Some(operation_id) = &operation.operation_id {
            let req = Requirement::new(
                operation_id.clone(),
                format!(
                    "{} {} - {}",
                    method,
                    path,
                    operation.summary.as_ref().unwrap_or(&String::new())
                ),
            );
            spec.requirements.push(req);
        }

        spec.apis.push(api);
        Ok(())
    }

    /// Extract data model from OpenAPI schema
    fn extract_data_model(&self, name: &str, schema: &OpenApiSchema) -> Result<Option<DataModel>> {
        let schema_kind = &schema.schema_kind;

        match schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                let mut fields = Vec::new();

                for (prop_name, prop_schema_ref) in &obj.properties {
                    if let ReferenceOr::Item(prop_schema) = prop_schema_ref {
                        let field = Field {
                            name: prop_name.clone(),
                            data_type: self.get_schema_type(prop_schema),
                            required: obj.required.contains(prop_name),
                            description: prop_schema
                                .schema_data
                                .description
                                .clone()
                                .unwrap_or_default(),
                            default_value: prop_schema
                                .schema_data
                                .default
                                .as_ref()
                                .map(|v| v.to_string()),
                            validation: Vec::new(),
                        };
                        fields.push(field);
                    }
                }

                Ok(Some(DataModel {
                    name: name.to_string(),
                    fields,
                    description: schema.schema_data.description.clone().unwrap_or_default(),
                    json_schema: None,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Get the type of a schema
    fn get_schema_type(&self, schema: &OpenApiSchema) -> String {
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(t) => match t {
                openapiv3::Type::String(_) => "string".to_string(),
                openapiv3::Type::Number(_) => "number".to_string(),
                openapiv3::Type::Integer(_) => "integer".to_string(),
                openapiv3::Type::Boolean(_) => "boolean".to_string(),
                openapiv3::Type::Array(_) => "array".to_string(),
                openapiv3::Type::Object(_) => "object".to_string(),
            },
            _ => "any".to_string(),
        }
    }

    /// Extract requirements from description text
    fn extract_requirements_from_description(&self, spec: &mut Specification, description: &str) {
        let lines: Vec<&str> = description.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Look for requirement keywords
            if line.contains("MUST")
                || line.contains("SHALL")
                || line.contains("SHOULD")
                || line.contains("MAY")
            {
                let req = Requirement::new(
                    format!("REQ-API-{:03}", spec.requirements.len() + 1),
                    line.to_string(),
                );
                spec.requirements.push(req);
            }
        }
    }
}

impl Default for OpenApiParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_parser_creation() {
        let parser = OpenApiParser::new();
        let _ = format!("{:?}", parser);
    }

    #[test]
    fn test_parse_simple_openapi() {
        let parser = OpenApiParser::new();
        let openapi_yaml = r#"
openapi: 3.0.0
info:
  title: Sample API
  version: 1.0.0
  description: |
    This API MUST support user authentication.
    The system SHALL validate all inputs.
paths:
  /users:
    get:
      summary: Get all users
      operationId: getUsers
      responses:
        '200':
          description: List of users
          content:
            application/json:
              schema:
                type: array
                items:
                  type: object
                  properties:
                    id:
                      type: integer
                    name:
                      type: string
    post:
      summary: Create a new user
      operationId: createUser
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
                email:
                  type: string
              required:
                - name
                - email
      responses:
        '201':
          description: User created
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
          description: User ID
        name:
          type: string
          description: User name
        email:
          type: string
          description: User email
      required:
        - id
        - email
"#;

        let spec = parser.parse_yaml(openapi_yaml).unwrap();

        // Check APIs were extracted
        assert!(!spec.apis.is_empty());

        // Check requirements were extracted from description
        assert!(!spec.requirements.is_empty());

        // Check data models were extracted
        assert!(!spec.data_models.is_empty());

        // Verify specific API
        let get_api =
            spec.apis.iter().find(|api| api.method == HttpMethod::Get && api.endpoint == "/users");
        assert!(get_api.is_some());

        // Verify data model
        let user_model = spec.data_models.iter().find(|model| model.name == "User");
        assert!(user_model.is_some());

        if let Some(user) = user_model {
            assert_eq!(user.fields.len(), 3);
            let email_field = user.fields.iter().find(|f| f.name == "email");
            assert!(email_field.is_some());
            assert!(email_field.unwrap().required);
        }
    }
}
