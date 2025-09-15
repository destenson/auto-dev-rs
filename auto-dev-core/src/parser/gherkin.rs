//! Gherkin/BDD scenario parser

use gherkin::{Feature, Scenario, StepType, GherkinEnv};
use anyhow::Result;
use std::path::PathBuf;

use crate::parser::model::*;

/// Parser for Gherkin feature files
#[derive(Debug)]
pub struct GherkinParser {}

impl GherkinParser {
    /// Create a new Gherkin parser
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a Gherkin feature file
    pub fn parse(&self, content: &str) -> Result<Specification> {
        let mut spec = Specification::new(PathBuf::new());
        
        // Parse the Gherkin content
        let env = GherkinEnv::default();
        let feature = Feature::parse(content, env)
            .map_err(|e| anyhow::anyhow!("Failed to parse Gherkin: {:?}", e))?;
        
        // Extract feature-level information
        self.extract_feature_info(&mut spec, &feature)?;
        
        // Extract scenarios as behavior specifications
        for scenario in &feature.scenarios {
            if let Some(behavior) = self.extract_behavior_spec(&feature, scenario)? {
                spec.behaviors.push(behavior);
                
                // Also create a requirement from the scenario
                let req = self.create_requirement_from_scenario(&feature, scenario);
                spec.requirements.push(req);
            }
        }
        
        // Extract acceptance criteria from scenarios
        self.extract_acceptance_criteria(&mut spec, &feature)?;
        
        Ok(spec)
    }

    /// Extract feature-level information
    fn extract_feature_info(&self, spec: &mut Specification, feature: &Feature) -> Result<()> {
        // Create a high-level requirement for the feature
        let feature_req = Requirement {
            id: format!("FEAT-{}", sanitize_id(&feature.name)),
            description: feature.description.clone().unwrap_or_else(|| feature.name.clone()),
            category: RequirementType::Functional,
            priority: Priority::High,
            acceptance_criteria: Vec::new(),
            source_location: SourceLocation::default(),
            related: Vec::new(),
            tags: feature.tags.clone(),
        };
        
        spec.requirements.push(feature_req);
        
        Ok(())
    }

    /// Extract behavior specification from a scenario
    fn extract_behavior_spec(
        &self,
        feature: &Feature,
        scenario: &Scenario,
    ) -> Result<Option<BehaviorSpec>> {
        let mut given_steps = Vec::new();
        let mut when_steps = Vec::new();
        let mut then_steps = Vec::new();
        
        for step in &scenario.steps {
            let step_text = format!("{} {}", step.keyword.trim(), step.value);
            
            match step.ty {
                StepType::Given => given_steps.push(step_text),
                StepType::When => when_steps.push(step_text),
                StepType::Then => then_steps.push(step_text),
            }
        }
        
        Ok(Some(BehaviorSpec {
            feature: feature.name.clone(),
            scenario: scenario.name.clone(),
            given: given_steps,
            when: when_steps,
            then: then_steps,
            tags: scenario.tags.clone(),
        }))
    }

    /// Create a requirement from a scenario
    fn create_requirement_from_scenario(
        &self,
        feature: &Feature,
        scenario: &Scenario,
    ) -> Requirement {
        let id = format!("SCEN-{}", sanitize_id(&scenario.name));
        
        // Build acceptance criteria from steps
        let mut acceptance_criteria = Vec::new();
        for step in &scenario.steps {
            acceptance_criteria.push(format!("{} {}", step.keyword.trim(), step.value));
        }
        
        Requirement {
            id,
            description: scenario.name.clone(),
            category: RequirementType::Behavior,
            priority: self.determine_priority_from_tags(&scenario.tags),
            acceptance_criteria,
            source_location: SourceLocation::default(),
            related: vec![format!("FEAT-{}", sanitize_id(&feature.name))],
            tags: scenario.tags.clone(),
        }
    }

    /// Extract acceptance criteria from all scenarios
    fn extract_acceptance_criteria(&self, spec: &mut Specification, feature: &Feature) -> Result<()> {
        // Find the feature requirement and add acceptance criteria
        if let Some(feature_req) = spec.requirements.iter_mut()
            .find(|r| r.id == format!("FEAT-{}", sanitize_id(&feature.name))) 
        {
            for scenario in &feature.scenarios {
                feature_req.acceptance_criteria.push(
                    format!("Scenario: {}", scenario.name)
                );
            }
        }
        
        Ok(())
    }

    /// Determine priority from scenario tags
    fn determine_priority_from_tags(&self, tags: &[String]) -> Priority {
        for tag in tags {
            let tag_lower = tag.to_lowercase();
            if tag_lower.contains("critical") || tag_lower.contains("must") {
                return Priority::Critical;
            } else if tag_lower.contains("high") || tag_lower.contains("should") {
                return Priority::High;
            } else if tag_lower.contains("medium") || tag_lower.contains("could") {
                return Priority::Medium;
            } else if tag_lower.contains("low") || tag_lower.contains("nice") {
                return Priority::Low;
            }
        }
        Priority::Medium // Default priority
    }
}

impl Default for GherkinParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize a string to be used as an ID
fn sanitize_id(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gherkin_parser_creation() {
        let parser = GherkinParser::new();
        let _ = format!("{:?}", parser);
    }

    #[test]
    fn test_parse_simple_feature() {
        let parser = GherkinParser::new();
        let gherkin = r#"Feature: User Authentication
  As a user
  I want to login to the system
  So that I can access my account

  @critical @auth
  Scenario: Successful login with valid credentials
    Given I am on the login page
    And I have a valid account
    When I enter my email "user@example.com"
    And I enter my password "secure123"
    And I click the login button
    Then I should be redirected to the dashboard
    And I should see a welcome message

  @high
  Scenario: Failed login with invalid credentials
    Given I am on the login page
    When I enter an invalid email "wrong@example.com"
    And I enter an invalid password "wrong"
    And I click the login button
    Then I should see an error message
    And I should remain on the login page
"#;
        
        let spec = parser.parse(gherkin).unwrap();
        
        // Check that feature was extracted
        assert!(!spec.requirements.is_empty());
        let feature_req = spec.requirements.iter()
            .find(|r| r.id.starts_with("FEAT-"));
        assert!(feature_req.is_some());
        
        // Check that scenarios were extracted as behaviors
        assert_eq!(spec.behaviors.len(), 2);
        
        // Check that scenarios were also extracted as requirements
        let scenario_reqs: Vec<_> = spec.requirements.iter()
            .filter(|r| r.id.starts_with("SCEN-"))
            .collect();
        assert_eq!(scenario_reqs.len(), 2);
        
        // Check behavior details
        let first_behavior = &spec.behaviors[0];
        assert_eq!(first_behavior.feature, "User Authentication");
        assert_eq!(first_behavior.scenario, "Successful login with valid credentials");
        assert!(!first_behavior.given.is_empty());
        assert!(!first_behavior.when.is_empty());
        assert!(!first_behavior.then.is_empty());
        assert!(first_behavior.tags.contains(&"critical".to_string()));
        
        // Check priority from tags
        let critical_req = spec.requirements.iter()
            .find(|r| r.tags.contains(&"critical".to_string()));
        assert!(critical_req.is_some());
        assert_eq!(critical_req.unwrap().priority, Priority::Critical);
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("User Login Flow"), "USERLOGINFLOW");
        assert_eq!(sanitize_id("Test-Case_123"), "TEST-CASE_123");
        assert_eq!(sanitize_id("Special!@#$%^&*()Chars"), "SPECIALCHARS");
    }

    #[test]
    fn test_priority_from_tags() {
        let parser = GherkinParser::new();
        
        assert_eq!(
            parser.determine_priority_from_tags(&vec!["critical".to_string()]),
            Priority::Critical
        );
        assert_eq!(
            parser.determine_priority_from_tags(&vec!["high".to_string(), "auth".to_string()]),
            Priority::High
        );
        assert_eq!(
            parser.determine_priority_from_tags(&vec!["feature".to_string()]),
            Priority::Medium
        );
    }
}