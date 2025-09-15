//! Integration test generation strategy

use super::{TestContext, TestStrategy};
use crate::test_gen::{Assertion, AssertionType, ExpectedOutcome, TestCase, TestType};
use anyhow::Result;

/// Strategy for generating integration tests
pub struct IntegrationTestStrategy {
    include_database_tests: bool,
    include_api_tests: bool,
    include_service_tests: bool,
    test_isolation: bool,
}

impl IntegrationTestStrategy {
    pub fn new() -> Self {
        Self {
            include_database_tests: true,
            include_api_tests: true,
            include_service_tests: true,
            test_isolation: true,
        }
    }

    pub fn with_database_tests(mut self, include: bool) -> Self {
        self.include_database_tests = include;
        self
    }

    pub fn with_api_tests(mut self, include: bool) -> Self {
        self.include_api_tests = include;
        self
    }

    fn detect_integration_points(&self, context: &TestContext) -> Vec<IntegrationPoint> {
        let mut points = Vec::new();
        let desc = context.description.to_lowercase();
        let fn_name = context.function_name.to_lowercase();

        // Detect database integration
        if self.include_database_tests
            && (desc.contains("database")
                || desc.contains("db")
                || desc.contains("repository")
                || desc.contains("dao"))
        {
            points.push(IntegrationPoint::Database);
        }

        // Detect API integration
        if self.include_api_tests
            && (desc.contains("api")
                || desc.contains("endpoint")
                || desc.contains("http")
                || desc.contains("rest"))
        {
            points.push(IntegrationPoint::Api);
        }

        // Detect service integration
        if self.include_service_tests
            && (desc.contains("service")
                || desc.contains("client")
                || fn_name.contains("_service")
                || fn_name.contains("fetch"))
        {
            points.push(IntegrationPoint::Service);
        }

        // Detect messaging integration
        if desc.contains("queue")
            || desc.contains("message")
            || desc.contains("event")
            || desc.contains("pubsub")
        {
            points.push(IntegrationPoint::Messaging);
        }

        // Detect cache integration
        if desc.contains("cache") || desc.contains("redis") || desc.contains("memcache") {
            points.push(IntegrationPoint::Cache);
        }

        // Detect file system integration
        if desc.contains("file")
            || desc.contains("fs")
            || desc.contains("disk")
            || desc.contains("storage")
        {
            points.push(IntegrationPoint::FileSystem);
        }

        points
    }

    fn generate_database_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_database_integration", context.function_name),
            TestType::Integration,
        );

        test.description = format!("Integration test: {} with database", context.function_name);

        // Database setup would be added to TestSuite, not TestCase

        // Add assertions
        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(true),
            actual: "db.is_connected()".to_string(),
            message: Some("Database should be connected".to_string()),
        });

        test.assertions.push(Assertion {
            assertion_type: AssertionType::DoesNotThrow,
            expected: serde_json::json!(true),
            actual: "function_with_db_call()".to_string(),
            message: Some("Database operations should not throw".to_string()),
        });

        test
    }

    fn generate_api_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_api_integration", context.function_name),
            TestType::Integration,
        );

        test.description = format!("Integration test: {} API endpoint", context.function_name);

        // API setup would be added to TestSuite, not TestCase

        // Add API test assertions
        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(200),
            actual: "response.status()".to_string(),
            message: Some("API should return 200 OK".to_string()),
        });

        test.assertions.push(Assertion {
            assertion_type: AssertionType::Contains,
            expected: serde_json::json!("application/json"),
            actual: "response.content_type()".to_string(),
            message: Some("Response should be JSON".to_string()),
        });

        test
    }

    fn generate_service_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_service_integration", context.function_name),
            TestType::Integration,
        );

        test.description =
            format!("Integration test: {} with external service", context.function_name);

        // Service mocking setup would be added to TestSuite, not TestCase

        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(true),
            actual: "service_call_succeeded".to_string(),
            message: Some("Service call should succeed".to_string()),
        });

        test
    }

    fn generate_end_to_end_test(&self, context: &TestContext) -> TestCase {
        let mut test =
            TestCase::new(format!("test_{}_e2e", context.function_name), TestType::Integration);

        test.description = format!("End-to-end test: complete {} workflow", context.function_name);

        // Comprehensive setup would be added to TestSuite, not TestCase

        // Add workflow assertions
        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(true),
            actual: "workflow_completed".to_string(),
            message: Some("Complete workflow should succeed".to_string()),
        });

        test.expected = ExpectedOutcome {
            success: true,
            value: Some(serde_json::json!({"status": "completed"})),
            error: None,
        };

        test
    }
}

#[derive(Debug, Clone, PartialEq)]
enum IntegrationPoint {
    Database,
    Api,
    Service,
    Messaging,
    Cache,
    FileSystem,
}

impl TestStrategy for IntegrationTestStrategy {
    fn generate(&self, context: &TestContext) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();

        // Detect integration points
        let integration_points = self.detect_integration_points(context);

        // Generate tests for each integration point
        for point in &integration_points {
            match point {
                IntegrationPoint::Database => {
                    tests.push(self.generate_database_test(context));
                }
                IntegrationPoint::Api => {
                    tests.push(self.generate_api_test(context));
                }
                IntegrationPoint::Service => {
                    tests.push(self.generate_service_test(context));
                }
                _ => {
                    // Generate generic integration test
                    tests.push(self.generate_end_to_end_test(context));
                }
            }
        }

        // If multiple integration points, add end-to-end test
        if integration_points.len() > 1 {
            tests.push(self.generate_end_to_end_test(context));
        }

        Ok(tests)
    }

    fn test_type(&self) -> TestType {
        TestType::Integration
    }

    fn applies_to(&self, context: &TestContext) -> bool {
        // Integration tests apply when multiple components are involved
        let desc = context.description.to_lowercase();

        desc.contains("integration") ||
        desc.contains("database") ||
        desc.contains("api") ||
        desc.contains("service") ||
        desc.contains("endpoint") ||
        desc.contains("workflow") ||
        desc.contains("system") ||
        // Multiple components detected
        self.detect_integration_points(context).len() > 0
    }
}

impl Default for IntegrationTestStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_strategy_creation() {
        let strategy = IntegrationTestStrategy::new();
        assert!(strategy.include_database_tests);
        assert!(strategy.include_api_tests);
        assert!(strategy.include_service_tests);
        assert!(strategy.test_isolation);
    }

    #[test]
    fn test_integration_point_detection() {
        let strategy = IntegrationTestStrategy::new();

        let db_context = TestContext::new("save_user")
            .with_return_type("Result<User>")
            .with_parameter(super::super::ParameterInfo {
                name: "user".to_string(),
                param_type: "User".to_string(),
                nullable: false,
                default_value: None,
                constraints: Vec::new(),
            });
        let mut db_context = db_context;
        db_context.description = "Save user to database".to_string();

        let points = strategy.detect_integration_points(&db_context);
        assert!(points.contains(&IntegrationPoint::Database));

        let api_context = TestContext {
            function_name: "fetch_data".to_string(),
            description: "Fetch data from API endpoint".to_string(),
            parameters: Vec::new(),
            return_type: String::new(),
            constraints: Vec::new(),
            examples: Vec::new(),
        };

        let points = strategy.detect_integration_points(&api_context);
        assert!(points.contains(&IntegrationPoint::Api));
    }

    #[test]
    fn test_applies_to_integration_scenarios() {
        let strategy = IntegrationTestStrategy::new();

        let integration_context = TestContext {
            function_name: "process_order".to_string(),
            description: "Process order with database and API calls".to_string(),
            parameters: Vec::new(),
            return_type: String::new(),
            constraints: Vec::new(),
            examples: Vec::new(),
        };

        assert!(strategy.applies_to(&integration_context));

        let unit_context = TestContext {
            function_name: "calculate_sum".to_string(),
            description: "Calculate sum of two numbers".to_string(),
            parameters: Vec::new(),
            return_type: String::new(),
            constraints: Vec::new(),
            examples: Vec::new(),
        };

        assert!(!strategy.applies_to(&unit_context));
    }

    #[test]
    fn test_database_test_generation() {
        let strategy = IntegrationTestStrategy::new();
        let context = TestContext {
            function_name: "save_user".to_string(),
            description: "Save user to database".to_string(),
            parameters: Vec::new(),
            return_type: "Result<User>".to_string(),
            constraints: Vec::new(),
            examples: Vec::new(),
        };

        let test = strategy.generate_database_test(&context);
        assert_eq!(test.test_type, TestType::Integration);
        assert!(test.name.contains("database_integration"));
        // Fixtures would be on TestSuite, not TestCase
        assert!(!test.assertions.is_empty());
    }
}
