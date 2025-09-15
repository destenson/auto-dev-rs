//! Semantic model for specifications and requirements

use std::path::PathBuf;
use std::fmt;
use serde::{Deserialize, Serialize};

/// Unified specification representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    /// Source file path
    pub source: PathBuf,
    
    /// Extracted requirements
    pub requirements: Vec<Requirement>,
    
    /// API definitions
    pub apis: Vec<ApiDefinition>,
    
    /// Data models/schemas
    pub data_models: Vec<DataModel>,
    
    /// Behavioral specifications
    pub behaviors: Vec<BehaviorSpec>,
    
    /// Code examples
    pub examples: Vec<Example>,
    
    /// Constraints and rules
    pub constraints: Vec<Constraint>,
}

impl Specification {
    /// Create a new empty specification
    pub fn new(source: PathBuf) -> Self {
        Self {
            source,
            requirements: Vec::new(),
            apis: Vec::new(),
            data_models: Vec::new(),
            behaviors: Vec::new(),
            examples: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Merge another specification into this one
    pub fn merge(&mut self, other: Specification) {
        self.requirements.extend(other.requirements);
        self.apis.extend(other.apis);
        self.data_models.extend(other.data_models);
        self.behaviors.extend(other.behaviors);
        self.examples.extend(other.examples);
        self.constraints.extend(other.constraints);
    }

    /// Check if the specification is empty
    pub fn is_empty(&self) -> bool {
        self.requirements.is_empty() &&
        self.apis.is_empty() &&
        self.data_models.is_empty() &&
        self.behaviors.is_empty() &&
        self.examples.is_empty() &&
        self.constraints.is_empty()
    }
}

/// A single requirement extracted from documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    /// Unique identifier
    pub id: String,
    
    /// Requirement description
    pub description: String,
    
    /// Type of requirement
    pub category: RequirementType,
    
    /// Priority level
    pub priority: Priority,
    
    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
    
    /// Source location in document
    pub source_location: SourceLocation,
    
    /// Related requirements
    pub related: Vec<String>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Requirement {
    /// Create a new requirement
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            category: RequirementType::Functional,
            priority: Priority::Medium,
            acceptance_criteria: Vec::new(),
            source_location: SourceLocation::default(),
            related: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// Type of requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementType {
    Functional,
    Api,
    DataModel,
    Behavior,
    Performance,
    Security,
    Usability,
    Reliability,
    Compatibility,
}

impl fmt::Display for RequirementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Functional => write!(f, "Functional"),
            Self::Api => write!(f, "API"),
            Self::DataModel => write!(f, "Data Model"),
            Self::Behavior => write!(f, "Behavior"),
            Self::Performance => write!(f, "Performance"),
            Self::Security => write!(f, "Security"),
            Self::Usability => write!(f, "Usability"),
            Self::Reliability => write!(f, "Reliability"),
            Self::Compatibility => write!(f, "Compatibility"),
        }
    }
}

/// Requirement priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl From<&str> for Priority {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "MUST" | "SHALL" | "CRITICAL" => Self::Critical,
            "SHOULD" | "HIGH" => Self::High,
            "COULD" | "MAY" | "MEDIUM" => Self::Medium,
            _ => Self::Low,
        }
    }
}

/// API definition extracted from specs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDefinition {
    /// Endpoint path
    pub endpoint: String,
    
    /// HTTP method
    pub method: HttpMethod,
    
    /// Request schema
    pub request_schema: Option<DataModel>,
    
    /// Response schema
    pub response_schema: Option<DataModel>,
    
    /// Query parameters
    pub query_params: Vec<Parameter>,
    
    /// Path parameters
    pub path_params: Vec<Parameter>,
    
    /// Headers
    pub headers: Vec<Parameter>,
    
    /// Description
    pub description: String,
    
    /// Example requests/responses
    pub examples: Vec<Example>,
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "DELETE" => Self::Delete,
            "PATCH" => Self::Patch,
            "HEAD" => Self::Head,
            "OPTIONS" => Self::Options,
            _ => Self::Get,
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Patch => write!(f, "PATCH"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

/// API parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

/// Data model/schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModel {
    /// Model name
    pub name: String,
    
    /// Fields/properties
    pub fields: Vec<Field>,
    
    /// Description
    pub description: String,
    
    /// JSON Schema if available
    pub json_schema: Option<serde_json::Value>,
}

/// Field in a data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
    pub validation: Vec<String>,
}

/// Behavioral specification (e.g., from Gherkin)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSpec {
    /// Feature name
    pub feature: String,
    
    /// Scenario name
    pub scenario: String,
    
    /// Given conditions
    pub given: Vec<String>,
    
    /// When actions
    pub when: Vec<String>,
    
    /// Then assertions
    pub then: Vec<String>,
    
    /// Tags
    pub tags: Vec<String>,
}

/// Code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// Example title
    pub title: String,
    
    /// Programming language
    pub language: String,
    
    /// Code content
    pub code: String,
    
    /// Description
    pub description: String,
    
    /// Expected output
    pub expected_output: Option<String>,
}

/// Constraint or business rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint ID
    pub id: String,
    
    /// Description
    pub description: String,
    
    /// Type of constraint
    pub constraint_type: ConstraintType,
    
    /// Validation rule
    pub rule: String,
}

/// Type of constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    BusinessRule,
    Validation,
    Security,
    Performance,
    Compatibility,
}

/// Source location in a document
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceLocation {
    /// File path
    pub file: Option<PathBuf>,
    
    /// Line number
    pub line: Option<usize>,
    
    /// Column number
    pub column: Option<usize>,
    
    /// Section/heading
    pub section: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: PathBuf, line: usize) -> Self {
        Self {
            file: Some(file),
            line: Some(line),
            column: None,
            section: None,
        }
    }

    /// Create location with section
    pub fn with_section(mut self, section: String) -> Self {
        self.section = Some(section);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specification_creation() {
        let spec = Specification::new(PathBuf::from("test.md"));
        assert!(spec.is_empty());
        assert_eq!(spec.source, PathBuf::from("test.md"));
    }

    #[test]
    fn test_requirement_creation() {
        let req = Requirement::new(
            "REQ-001".to_string(),
            "System must support user login".to_string()
        );
        assert_eq!(req.id, "REQ-001");
        assert_eq!(req.category, RequirementType::Functional);
        assert_eq!(req.priority, Priority::Medium);
    }

    #[test]
    fn test_priority_from_string() {
        assert_eq!(Priority::from("MUST"), Priority::Critical);
        assert_eq!(Priority::from("should"), Priority::High);
        assert_eq!(Priority::from("could"), Priority::Medium);
        assert_eq!(Priority::from("nice to have"), Priority::Low);
    }

    #[test]
    fn test_http_method_from_string() {
        assert_eq!(HttpMethod::from("GET"), HttpMethod::Get);
        assert_eq!(HttpMethod::from("post"), HttpMethod::Post);
        assert_eq!(HttpMethod::from("PUT"), HttpMethod::Put);
    }

    #[test]
    fn test_specification_merge() {
        let mut spec1 = Specification::new(PathBuf::from("spec1.md"));
        spec1.requirements.push(Requirement::new(
            "REQ-001".to_string(),
            "First requirement".to_string()
        ));

        let mut spec2 = Specification::new(PathBuf::from("spec2.md"));
        spec2.requirements.push(Requirement::new(
            "REQ-002".to_string(),
            "Second requirement".to_string()
        ));

        spec1.merge(spec2);
        assert_eq!(spec1.requirements.len(), 2);
    }
}