//! Increment planning and dependency analysis

use super::{
    Increment, SpecFragment, Result, IncrementalError, Complexity,
    ValidationCriteria, TestCase, TestType, ExpectedOutcome,
};
use crate::parser::model::{Specification, Requirement};
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

/// Plans increments from specifications
pub struct IncrementPlanner {
    max_increment_size: usize,
    test_first: bool,
}

impl IncrementPlanner {
    /// Create a new increment planner
    pub fn new(max_increment_size: usize, test_first: bool) -> Self {
        Self {
            max_increment_size,
            test_first,
        }
    }
    
    /// Plan increments from a specification
    pub fn plan_increments(&self, spec: &Specification) -> Result<IncrementPlan> {
        let mut increments = Vec::new();
        let mut dependency_graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        // Break down requirements into increments
        for requirement in &spec.requirements {
            let fragments = self.break_down_requirement(requirement)?;
            
            for fragment in fragments {
                let increment = self.create_increment(fragment, spec);
                let node_idx = dependency_graph.add_node(increment.id);
                node_map.insert(increment.id, node_idx);
                increments.push(increment);
            }
        }
        
        // Analyze and add dependencies
        self.add_dependencies(&mut increments, &mut dependency_graph, &node_map)?;
        
        // Topological sort for execution order
        let sorted = toposort(&dependency_graph, None)
            .map_err(|_| IncrementalError::DependencyError("Circular dependency detected".to_string()))?;
        
        // Calculate critical path
        let critical_path = self.find_critical_path(&increments, &dependency_graph);
        
        // Estimate total duration
        let estimated_duration = self.estimate_duration(&increments);
        
        let execution_order = sorted.into_iter()
            .filter_map(|idx| {
                // Find the increment with matching node index
                increments.iter()
                    .find(|inc| node_map.get(&inc.id) == Some(&idx))
                    .map(|inc| inc.id)
            })
            .collect();
        
        Ok(IncrementPlan {
            increments,
            dependency_graph,
            critical_path,
            estimated_duration,
            execution_order,
        })
    }
    
    /// Break down a requirement into spec fragments
    fn break_down_requirement(&self, requirement: &Requirement) -> Result<Vec<SpecFragment>> {
        let mut fragments = Vec::new();
        
        // Analyze requirement complexity
        let complexity = self.assess_complexity(&requirement.description);
        
        // If simple enough, create single fragment
        if complexity <= Complexity::Simple {
            fragments.push(SpecFragment {
                id: requirement.id.clone(),
                description: requirement.description.clone(),
                requirements: vec![requirement.description.clone()],
                context: String::new(),
                examples: requirement.acceptance_criteria.clone(),
            });
        } else {
            // Break down complex requirements
            let sub_tasks = self.identify_subtasks(&requirement.description);
            
            for (idx, task) in sub_tasks.iter().enumerate() {
                fragments.push(SpecFragment {
                    id: format!("{}_{}", requirement.id, idx),
                    description: task.clone(),
                    requirements: vec![task.clone()],
                    context: requirement.description.clone(),
                    examples: requirement.acceptance_criteria.clone(),
                });
            }
        }
        
        Ok(fragments)
    }
    
    /// Create an increment from a spec fragment
    fn create_increment(&self, fragment: SpecFragment, _spec: &Specification) -> Increment {
        let mut increment = Increment::new(fragment, Vec::new());
        
        // Set complexity
        increment.implementation.estimated_complexity = self.assess_complexity(&increment.specification.description);
        
        // Set approach
        increment.implementation.approach = if self.test_first {
            "Test-Driven Development".to_string()
        } else {
            "Implementation First".to_string()
        };
        
        // Generate test cases if test-first
        if self.test_first {
            increment.tests = self.generate_test_cases(&increment.specification);
        }
        
        // Set validation criteria
        increment.validation = ValidationCriteria {
            must_compile: true,
            tests_must_pass: increment.tests.iter().map(|t| t.id.clone()).collect(),
            performance_criteria: None,
            security_checks: Vec::new(),
        };
        
        increment
    }
    
    /// Generate test cases for a spec fragment
    fn generate_test_cases(&self, spec: &SpecFragment) -> Vec<TestCase> {
        let mut tests = Vec::new();
        
        // Unit test for the main functionality
        tests.push(TestCase {
            id: format!("test_{}", spec.id),
            name: format!("Test {}", spec.description),
            test_type: TestType::Unit,
            command: format!("cargo test test_{}", spec.id),
            expected_outcome: ExpectedOutcome::Success,
        });
        
        // Add tests for examples
        for (idx, example) in spec.examples.iter().enumerate() {
            tests.push(TestCase {
                id: format!("test_{}_{}", spec.id, idx),
                name: format!("Test example: {}", example),
                test_type: TestType::Unit,
                command: format!("cargo test test_{}_{}", spec.id, idx),
                expected_outcome: ExpectedOutcome::Success,
            });
        }
        
        // Compilation test
        tests.push(TestCase {
            id: format!("compile_{}", spec.id),
            name: "Compilation check".to_string(),
            test_type: TestType::Compilation,
            command: "cargo check".to_string(),
            expected_outcome: ExpectedOutcome::Success,
        });
        
        tests
    }
    
    /// Assess the complexity of a task
    fn assess_complexity(&self, description: &str) -> Complexity {
        let word_count = description.split_whitespace().count();
        let has_api = description.to_lowercase().contains("api");
        let has_database = description.to_lowercase().contains("database") || 
                          description.to_lowercase().contains("persist");
        let has_integration = description.to_lowercase().contains("integrate");
        
        let complexity_score = word_count / 10 + 
            if has_api { 2 } else { 0 } +
            if has_database { 3 } else { 0 } +
            if has_integration { 2 } else { 0 };
        
        match complexity_score {
            0..=2 => Complexity::Trivial,
            3..=5 => Complexity::Simple,
            6..=10 => Complexity::Moderate,
            11..=20 => Complexity::Complex,
            _ => Complexity::VeryComplex,
        }
    }
    
    /// Identify subtasks in a complex requirement
    fn identify_subtasks(&self, description: &str) -> Vec<String> {
        let mut tasks = Vec::new();
        
        // Split by common task indicators
        let indicators = ["and", "then", "also", "with", "including"];
        let mut current_task = String::new();
        
        for word in description.split_whitespace() {
            if indicators.contains(&word.to_lowercase().as_str()) && !current_task.is_empty() {
                tasks.push(current_task.trim().to_string());
                current_task = String::new();
            } else {
                if !current_task.is_empty() {
                    current_task.push(' ');
                }
                current_task.push_str(word);
            }
        }
        
        if !current_task.is_empty() {
            tasks.push(current_task.trim().to_string());
        }
        
        // If no natural split, create function-level tasks
        if tasks.len() == 1 && tasks[0].len() > 100 {
            let single_task = tasks[0].clone();
            tasks.clear();
            
            // Look for function/method indicators
            if single_task.contains("function") || single_task.contains("method") {
                tasks.push(format!("Create function signature for {}", single_task));
                tasks.push(format!("Implement function body for {}", single_task));
                tasks.push(format!("Add tests for {}", single_task));
            } else {
                // Generic breakdown
                tasks.push(format!("Create structure for {}", single_task));
                tasks.push(format!("Implement logic for {}", single_task));
                tasks.push(format!("Validate {}", single_task));
            }
        }
        
        tasks
    }
    
    /// Add dependencies between increments
    fn add_dependencies(
        &self,
        increments: &mut [Increment],
        graph: &mut DiGraph<Uuid, ()>,
        node_map: &HashMap<Uuid, NodeIndex>,
    ) -> Result<()> {
        // Analyze dependencies based on spec fragments
        for i in 0..increments.len() {
            for j in 0..increments.len() {
                if i != j {
                    let depends = self.check_dependency(&increments[i], &increments[j]);
                    if depends {
                        increments[i].dependencies.push(increments[j].id);
                        if let (Some(&from), Some(&to)) = (node_map.get(&increments[j].id), node_map.get(&increments[i].id)) {
                            graph.add_edge(from, to, ());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if increment A depends on increment B
    fn check_dependency(&self, a: &Increment, b: &Increment) -> bool {
        // Simple heuristic: check if A references B's output
        let a_desc = a.specification.description.to_lowercase();
        let b_desc = b.specification.description.to_lowercase();
        
        // Look for explicit references
        if a_desc.contains("after") && a_desc.contains(&b.specification.id) {
            return true;
        }
        
        // Data models before APIs
        if b_desc.contains("model") && a_desc.contains("api") {
            return true;
        }
        
        // Database before business logic
        if b_desc.contains("database") && a_desc.contains("service") {
            return true;
        }
        
        false
    }
    
    /// Find the critical path through increments
    fn find_critical_path(&self, increments: &[Increment], graph: &DiGraph<Uuid, ()>) -> Vec<Uuid> {
        // Simple implementation: find longest path
        let mut critical_path = Vec::new();
        
        // Find nodes with no incoming edges (start nodes)
        let start_nodes: Vec<_> = increments
            .iter()
            .filter(|inc| inc.dependencies.is_empty())
            .map(|inc| inc.id)
            .collect();
        
        // For each start node, find longest path
        for start in start_nodes {
            let path = self.find_longest_path_from(start, increments, graph);
            if path.len() > critical_path.len() {
                critical_path = path;
            }
        }
        
        critical_path
    }
    
    /// Find longest path from a given node
    fn find_longest_path_from(
        &self,
        start: Uuid,
        increments: &[Increment],
        _graph: &DiGraph<Uuid, ()>,
    ) -> Vec<Uuid> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut queue = VecDeque::new();
        
        queue.push_back((start, vec![start]));
        
        while let Some((node, current_path)) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            
            // Find dependents
            let dependents: Vec<_> = increments
                .iter()
                .filter(|inc| inc.dependencies.contains(&node))
                .map(|inc| inc.id)
                .collect();
            
            if dependents.is_empty() {
                // End of path
                if current_path.len() > path.len() {
                    path = current_path;
                }
            } else {
                for dependent in dependents {
                    let mut new_path = current_path.clone();
                    new_path.push(dependent);
                    queue.push_back((dependent, new_path));
                }
            }
        }
        
        path
    }
    
    /// Estimate total duration for all increments
    fn estimate_duration(&self, increments: &[Increment]) -> std::time::Duration {
        let total_complexity: usize = increments
            .iter()
            .map(|inc| match inc.implementation.estimated_complexity {
                Complexity::Trivial => 1,
                Complexity::Simple => 3,
                Complexity::Moderate => 10,
                Complexity::Complex => 30,
                Complexity::VeryComplex => 60,
            })
            .sum();
        
        std::time::Duration::from_secs(total_complexity as u64 * 60)
    }
}

/// Plan for incremental implementation
#[derive(Debug, Serialize, Deserialize)]
pub struct IncrementPlan {
    pub increments: Vec<Increment>,
    #[serde(skip)]
    pub dependency_graph: DiGraph<Uuid, ()>,
    pub critical_path: Vec<Uuid>,
    pub estimated_duration: std::time::Duration,
    pub execution_order: Vec<Uuid>,
}

impl Default for IncrementPlan {
    fn default() -> Self {
        Self {
            increments: Vec::new(),
            dependency_graph: DiGraph::new(),
            critical_path: Vec::new(),
            estimated_duration: std::time::Duration::from_secs(0),
            execution_order: Vec::new(),
        }
    }
}

impl IncrementPlan {
    /// Get next ready increment
    pub fn get_next_ready(&self, completed: &[Uuid]) -> Option<&Increment> {
        self.increments
            .iter()
            .find(|inc| inc.is_ready(completed))
    }
    
    /// Get progress statistics
    pub fn get_stats(&self) -> PlanStats {
        let total = self.increments.len();
        let completed = self.increments.iter().filter(|i| i.status == super::IncrementStatus::Completed).count();
        let failed = self.increments.iter().filter(|i| i.status == super::IncrementStatus::Failed).count();
        let in_progress = self.increments.iter().filter(|i| i.status == super::IncrementStatus::InProgress).count();
        
        PlanStats {
            total,
            completed,
            failed,
            in_progress,
            pending: total - completed - failed - in_progress,
            success_rate: if completed > 0 {
                completed as f32 / (completed + failed) as f32
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::model::{Priority, RequirementType};
    
    #[test]
    fn test_complexity_assessment() {
        let planner = IncrementPlanner::new(50, true);
        
        assert_eq!(planner.assess_complexity("Simple task"), Complexity::Trivial);
        assert_eq!(planner.assess_complexity("Create an API endpoint for user authentication"), Complexity::Trivial);
        assert_eq!(planner.assess_complexity("Implement database persistence layer with caching"), Complexity::Simple);
    }
    
    #[test]
    fn test_subtask_identification() {
        let planner = IncrementPlanner::new(50, true);
        
        let tasks = planner.identify_subtasks("Create user model and implement authentication");
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].contains("user model"));
        assert!(tasks[1].contains("authentication"));
    }
}