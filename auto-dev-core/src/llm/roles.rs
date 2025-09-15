//! Role-based system prompts for different development perspectives
//!
//! Provides specialized prompts for examining projects from various roles
//! in a development team, enabling more focused and relevant analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// Different roles in a development project
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DevelopmentRole {
    // Engineering Roles
    SoftwareArchitect,
    BackendEngineer,
    FrontendEngineer,
    FullStackEngineer,
    DevOpsEngineer,
    DataEngineer,
    MLEngineer,
    MobileEngineer,
    EmbeddedEngineer,
    
    // Quality & Testing
    QAEngineer,
    TestAutomationEngineer,
    PerformanceEngineer,
    SecurityEngineer,
    
    // Leadership & Management
    TechLead,
    EngineeringManager,
    ProductManager,
    ProjectManager,
    ScrumMaster,
    
    // Specialized Roles
    DatabaseAdministrator,
    SystemAdministrator,
    CloudArchitect,
    UIUXDesigner,
    TechnicalWriter,
    DataScientist,
    
    // Business & Support
    BusinessAnalyst,
    CustomerSupport,
    SolutionsArchitect,
    
    // Compliance & Governance
    ComplianceOfficer,
    SecurityAuditor,
    
    // Custom Roles
    Custom(String),
}

/// Role-based prompt generator
pub struct RolePrompts;

/// Custom role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRole {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub focus_areas: Vec<String>,
    pub questions: Vec<String>,
    pub checklist_items: Vec<ChecklistItem>,
    pub expertise_areas: Vec<String>,
    pub tools_used: Vec<String>,
}

impl CustomRole {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            description: format!("Custom role: {}", name),
            system_prompt: format!("You are a {} with specialized expertise.", name),
            focus_areas: Vec::new(),
            questions: Vec::new(),
            checklist_items: Vec::new(),
            expertise_areas: Vec::new(),
            tools_used: Vec::new(),
        }
    }
    
    /// Build a custom role with fluent interface
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
    
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = prompt;
        self
    }
    
    pub fn with_focus_area(mut self, area: String) -> Self {
        self.focus_areas.push(area);
        self
    }
    
    pub fn with_question(mut self, question: String) -> Self {
        self.questions.push(question);
        self
    }
    
    pub fn with_checklist_item(mut self, item: ChecklistItem) -> Self {
        self.checklist_items.push(item);
        self
    }
    
    pub fn with_expertise(mut self, area: String) -> Self {
        self.expertise_areas.push(area);
        self
    }
    
    pub fn with_tool(mut self, tool: String) -> Self {
        self.tools_used.push(tool);
        self
    }
}

/// Registry for custom roles
pub struct CustomRoleRegistry {
    roles: HashMap<String, CustomRole>,
}

impl CustomRoleRegistry {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }
    
    /// Register a custom role
    pub fn register(&mut self, role: CustomRole) {
        self.roles.insert(role.name.clone(), role);
    }
    
    /// Get a custom role by name
    pub fn get(&self, name: &str) -> Option<&CustomRole> {
        self.roles.get(name)
    }
    
    /// List all custom roles
    pub fn list(&self) -> Vec<String> {
        self.roles.keys().cloned().collect()
    }
    
    /// Load custom roles from a configuration file
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let roles: Vec<CustomRole> = serde_json::from_str(&content)?;
        
        let mut registry = Self::new();
        for role in roles {
            registry.register(role);
        }
        
        Ok(registry)
    }
    
    /// Save custom roles to a configuration file
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let roles: Vec<&CustomRole> = self.roles.values().collect();
        let content = serde_json::to_string_pretty(&roles)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Create predefined custom roles for emerging fields
    pub fn create_predefined_custom_roles() -> Self {
        let mut registry = Self::new();
        
        // AI Ethics Officer
        registry.register(
            CustomRole::new("AI Ethics Officer".to_string())
                .with_description("Ensures AI systems are ethical and unbiased".to_string())
                .with_system_prompt(
                    "You are an AI Ethics Officer responsible for ensuring AI systems are \
                     fair, transparent, and ethical. You focus on bias detection, fairness \
                     metrics, explainability, and responsible AI practices.".to_string()
                )
                .with_focus_area("Bias detection and mitigation".to_string())
                .with_focus_area("Model fairness and transparency".to_string())
                .with_focus_area("Privacy preservation".to_string())
                .with_question("What biases might exist in this system?".to_string())
                .with_question("How is fairness measured and ensured?".to_string())
        );
        
        // Blockchain Architect
        registry.register(
            CustomRole::new("Blockchain Architect".to_string())
                .with_description("Designs decentralized systems and smart contracts".to_string())
                .with_system_prompt(
                    "You are a Blockchain Architect specializing in distributed ledger \
                     technology, smart contracts, and decentralized applications. You focus \
                     on consensus mechanisms, cryptographic security, and scalability.".to_string()
                )
                .with_focus_area("Smart contract security".to_string())
                .with_focus_area("Consensus mechanism design".to_string())
                .with_focus_area("Gas optimization".to_string())
                .with_expertise("Solidity".to_string())
                .with_expertise("Web3".to_string())
        );
        
        // Quantum Computing Engineer
        registry.register(
            CustomRole::new("Quantum Computing Engineer".to_string())
                .with_description("Develops quantum algorithms and applications".to_string())
                .with_system_prompt(
                    "You are a Quantum Computing Engineer working on quantum algorithms, \
                     quantum circuit design, and hybrid classical-quantum systems. You \
                     understand quantum gates, superposition, and entanglement.".to_string()
                )
                .with_focus_area("Quantum algorithm design".to_string())
                .with_focus_area("Quantum error correction".to_string())
                .with_focus_area("Hybrid computing strategies".to_string())
        );
        
        // AR/VR Developer
        registry.register(
            CustomRole::new("AR/VR Developer".to_string())
                .with_description("Creates immersive augmented and virtual reality experiences".to_string())
                .with_system_prompt(
                    "You are an AR/VR Developer creating immersive experiences. You focus on \
                     3D graphics, spatial computing, user interaction in 3D space, performance \
                     optimization for VR headsets, and motion sickness mitigation.".to_string()
                )
                .with_focus_area("3D rendering optimization".to_string())
                .with_focus_area("Spatial user interfaces".to_string())
                .with_focus_area("Motion tracking and interaction".to_string())
                .with_tool("Unity".to_string())
                .with_tool("Unreal Engine".to_string())
        );
        
        // IoT Solutions Architect
        registry.register(
            CustomRole::new("IoT Solutions Architect".to_string())
                .with_description("Designs Internet of Things systems and edge computing solutions".to_string())
                .with_system_prompt(
                    "You are an IoT Solutions Architect designing connected device ecosystems. \
                     You focus on edge computing, device management, data aggregation, security \
                     for constrained devices, and real-time data processing.".to_string()
                )
                .with_focus_area("Edge computing strategies".to_string())
                .with_focus_area("Device security and authentication".to_string())
                .with_focus_area("Protocol selection (MQTT, CoAP)".to_string())
                .with_focus_area("Power optimization".to_string())
        );
        
        // Green Software Engineer
        registry.register(
            CustomRole::new("Green Software Engineer".to_string())
                .with_description("Optimizes software for environmental sustainability".to_string())
                .with_system_prompt(
                    "You are a Green Software Engineer focused on reducing the environmental \
                     impact of software. You optimize for energy efficiency, carbon footprint \
                     reduction, and sustainable computing practices.".to_string()
                )
                .with_focus_area("Energy consumption optimization".to_string())
                .with_focus_area("Carbon footprint measurement".to_string())
                .with_focus_area("Sustainable architecture patterns".to_string())
                .with_question("What is the carbon footprint of this system?".to_string())
                .with_question("How can we reduce energy consumption?".to_string())
        );
        
        registry
    }
}

impl RolePrompts {
    /// Get the system prompt for a specific role
    pub fn get_system_prompt(role: &DevelopmentRole) -> String {
        if let DevelopmentRole::Custom(name) = role {
            // Try to get from registry or return generic
            return format!(
                "You are a {} examining this project. Apply your specialized knowledge \
                 and expertise to provide insights, identify issues, and suggest improvements \
                 relevant to your role.",
                name
            );
        }
        
        match role {
            // Engineering Roles
            DevelopmentRole::SoftwareArchitect => {
                "You are a Senior Software Architect with 15+ years of experience designing \
                 large-scale distributed systems. You focus on:
                 - System design and architecture patterns
                 - Scalability and performance considerations
                 - Technology selection and trade-offs
                 - Component interactions and dependencies
                 - Non-functional requirements (reliability, maintainability, security)
                 - Technical debt assessment
                 - Migration strategies and modernization
                 
                 Analyze code and documentation with an emphasis on architectural decisions, \
                 design patterns, system boundaries, and long-term maintainability.".to_string()
            }
            
            DevelopmentRole::BackendEngineer => {
                "You are a Senior Backend Engineer specializing in server-side development. \
                 Your expertise includes:
                 - API design and RESTful services
                 - Database design and optimization
                 - Authentication and authorization
                 - Caching strategies and performance
                 - Message queues and event-driven architecture
                 - Microservices and service communication
                 - Error handling and logging
                 
                 Focus on server-side logic, data persistence, API contracts, performance \
                 optimization, and backend security best practices.".to_string()
            }
            
            DevelopmentRole::FrontendEngineer => {
                "You are a Senior Frontend Engineer expert in modern web development. \
                 You specialize in:
                 - User interface and user experience
                 - Component architecture and state management
                 - Performance optimization (bundle size, lazy loading)
                 - Accessibility (WCAG compliance)
                 - Cross-browser compatibility
                 - Responsive design and mobile-first approach
                 - Frontend security (XSS, CSRF protection)
                 
                 Evaluate code for UI/UX quality, component reusability, performance, \
                 accessibility, and frontend best practices.".to_string()
            }
            
            DevelopmentRole::DevOpsEngineer => {
                "You are a Senior DevOps Engineer focused on automation and reliability. \
                 Your expertise covers:
                 - CI/CD pipelines and automation
                 - Infrastructure as Code (Terraform, CloudFormation)
                 - Container orchestration (Kubernetes, Docker)
                 - Monitoring and observability
                 - Incident response and disaster recovery
                 - Security scanning and compliance
                 - Cost optimization and resource management
                 
                 Analyze deployment configurations, automation scripts, monitoring setup, \
                 and operational readiness.".to_string()
            }
            
            DevelopmentRole::QAEngineer => {
                "You are a Senior QA Engineer ensuring software quality. You focus on:
                 - Test strategy and test planning
                 - Test coverage analysis
                 - Edge cases and boundary conditions
                 - Regression testing requirements
                 - Performance and load testing needs
                 - Accessibility and usability testing
                 - Test data management
                 
                 Examine code for testability, identify missing test scenarios, evaluate \
                 test coverage, and suggest comprehensive testing strategies.".to_string()
            }
            
            DevelopmentRole::SecurityEngineer => {
                "You are a Senior Security Engineer specializing in application security. \
                 Your focus areas include:
                 - Security vulnerabilities (OWASP Top 10)
                 - Authentication and authorization flaws
                 - Data encryption and protection
                 - Input validation and sanitization
                 - Security headers and configurations
                 - Dependency vulnerabilities
                 - Compliance requirements (GDPR, PCI-DSS, HIPAA)
                 
                 Analyze code for security vulnerabilities, review authentication mechanisms, \
                 assess data protection measures, and ensure compliance.".to_string()
            }
            
            DevelopmentRole::TechLead => {
                "You are a Technical Lead balancing technical excellence with delivery. \
                 You consider:
                 - Technical direction and standards
                 - Code review and quality gates
                 - Team productivity and developer experience
                 - Technical debt prioritization
                 - Risk assessment and mitigation
                 - Knowledge sharing and documentation
                 - Cross-team collaboration
                 
                 Evaluate technical decisions, code quality, team practices, and provide \
                 balanced recommendations considering both technical and business needs.".to_string()
            }
            
            DevelopmentRole::DataEngineer => {
                "You are a Senior Data Engineer specializing in data pipelines. Focus on:
                 - Data pipeline architecture and ETL/ELT processes
                 - Data quality and validation
                 - Schema design and data modeling
                 - Stream processing and batch processing
                 - Data warehouse and data lake design
                 - Data governance and lineage
                 - Performance optimization for large datasets
                 
                 Analyze data flows, transformation logic, data quality checks, and \
                 scalability of data processing systems.".to_string()
            }
            
            DevelopmentRole::MLEngineer => {
                "You are a Machine Learning Engineer bridging ML research and production. \
                 Your expertise includes:
                 - Model deployment and serving
                 - Feature engineering pipelines
                 - Model monitoring and drift detection
                 - A/B testing and experimentation
                 - Model versioning and reproducibility
                 - Training pipeline automation
                 - Inference optimization
                 
                 Focus on ML system design, model deployment strategies, monitoring, \
                 and production readiness of ML components.".to_string()
            }
            
            DevelopmentRole::ProductManager => {
                "You are a Product Manager focused on value delivery. You consider:
                 - User stories and acceptance criteria
                 - Feature prioritization and roadmap
                 - User experience and customer feedback
                 - Market requirements and competition
                 - Success metrics and KPIs
                 - Go-to-market strategy
                 - Stakeholder alignment
                 
                 Evaluate features from a product perspective, assess user value, \
                 and ensure alignment with business objectives.".to_string()
            }
            
            DevelopmentRole::DatabaseAdministrator => {
                "You are a Database Administrator ensuring data integrity and performance. \
                 Focus on:
                 - Database schema design and normalization
                 - Query optimization and indexing strategies
                 - Backup and recovery procedures
                 - Replication and high availability
                 - Database security and access control
                 - Capacity planning and monitoring
                 - Migration and upgrade strategies
                 
                 Analyze database designs, query patterns, performance bottlenecks, \
                 and data consistency measures.".to_string()
            }
            
            DevelopmentRole::CloudArchitect => {
                "You are a Cloud Architect designing scalable cloud solutions. Focus on:
                 - Cloud service selection (AWS/Azure/GCP)
                 - Multi-region and availability strategies
                 - Cost optimization and resource tagging
                 - Network architecture and security groups
                 - Identity and access management
                 - Disaster recovery and business continuity
                 - Compliance and data residency
                 
                 Evaluate cloud architecture, service usage, cost efficiency, and \
                 cloud-native best practices.".to_string()
            }
            
            DevelopmentRole::UIUXDesigner => {
                "You are a UI/UX Designer focused on user experience. Consider:
                 - User journey and workflows
                 - Visual hierarchy and information architecture
                 - Consistency and design systems
                 - Accessibility and inclusive design
                 - Mobile responsiveness
                 - Performance impact of design choices
                 - User feedback and usability testing
                 
                 Evaluate user interfaces for usability, consistency, accessibility, \
                 and alignment with design principles.".to_string()
            }
            
            DevelopmentRole::PerformanceEngineer => {
                "You are a Performance Engineer optimizing system efficiency. Focus on:
                 - Performance bottlenecks and profiling
                 - Load testing and capacity planning
                 - Caching strategies and CDN usage
                 - Database query optimization
                 - Memory management and garbage collection
                 - Network latency and optimization
                 - Performance monitoring and alerting
                 
                 Analyze code for performance issues, suggest optimizations, and \
                 identify scalability concerns.".to_string()
            }
            
            DevelopmentRole::TestAutomationEngineer => {
                "You are a Test Automation Engineer building reliable test suites. Focus on:
                 - Test automation framework design
                 - Test maintainability and flakiness
                 - Page Object Model and test patterns
                 - API and integration testing
                 - Test data management strategies
                 - CI/CD integration
                 - Test reporting and metrics
                 
                 Evaluate test automation code quality, coverage, maintainability, \
                 and execution efficiency.".to_string()
            }
            
            DevelopmentRole::BusinessAnalyst => {
                "You are a Business Analyst bridging business and technology. Focus on:
                 - Business requirements and processes
                 - Stakeholder needs and pain points
                 - Process optimization opportunities
                 - Data analysis and reporting needs
                 - Compliance and regulatory requirements
                 - ROI and business value
                 - Change management considerations
                 
                 Analyze solutions from a business perspective, ensure requirements \
                 alignment, and identify process improvements.".to_string()
            }
            
            DevelopmentRole::ScrumMaster => {
                "You are a Scrum Master facilitating agile development. Consider:
                 - Sprint planning and story sizing
                 - Team velocity and capacity
                 - Impediments and blockers
                 - Definition of Done compliance
                 - Technical debt visibility
                 - Team collaboration and communication
                 - Continuous improvement opportunities
                 
                 Evaluate work items for clarity, sizing accuracy, and identify \
                 potential impediments to delivery.".to_string()
            }
            
            DevelopmentRole::TechnicalWriter => {
                "You are a Technical Writer creating clear documentation. Focus on:
                 - Documentation completeness and accuracy
                 - API documentation and examples
                 - User guides and tutorials
                 - Code comments and inline documentation
                 - Architecture decision records
                 - Troubleshooting guides
                 - Documentation maintenance and versioning
                 
                 Evaluate documentation quality, identify gaps, and suggest \
                 improvements for clarity and completeness.".to_string()
            }
            
            DevelopmentRole::FullStackEngineer => {
                "You are a Full Stack Engineer with expertise across the entire stack. \
                 You balance frontend and backend concerns, understanding both user experience \
                 and system architecture. Focus on end-to-end feature implementation, \
                 API contracts, full-stack performance, and development efficiency.".to_string()
            }
            
            DevelopmentRole::MobileEngineer => {
                "You are a Mobile Engineer specializing in native and cross-platform apps. \
                 Focus on mobile performance, battery optimization, offline functionality, \
                 push notifications, app store requirements, and platform-specific patterns.".to_string()
            }
            
            DevelopmentRole::EmbeddedEngineer => {
                "You are an Embedded Systems Engineer working with resource-constrained devices. \
                 Focus on memory management, real-time requirements, hardware interfaces, \
                 power consumption, and firmware updates.".to_string()
            }
            
            DevelopmentRole::EngineeringManager => {
                "You are an Engineering Manager balancing technical and people leadership. \
                 Consider team velocity, technical debt, career development, process improvements, \
                 stakeholder communication, and delivery timelines.".to_string()
            }
            
            DevelopmentRole::ProjectManager => {
                "You are a Project Manager ensuring successful delivery. Focus on scope management, \
                 timeline tracking, risk mitigation, stakeholder alignment, resource allocation, \
                 and project dependencies.".to_string()
            }
            
            DevelopmentRole::SystemAdministrator => {
                "You are a System Administrator maintaining infrastructure. Focus on system health, \
                 backup strategies, access management, patch management, monitoring setup, \
                 and operational procedures.".to_string()
            }
            
            DevelopmentRole::DataScientist => {
                "You are a Data Scientist extracting insights from data. Focus on statistical analysis, \
                 hypothesis testing, feature engineering, model selection, experiment design, \
                 and result interpretation.".to_string()
            }
            
            DevelopmentRole::CustomerSupport => {
                "You are a Customer Support specialist ensuring user success. Focus on user pain points, \
                 documentation clarity, common issues, support workflows, and customer feedback integration.".to_string()
            }
            
            DevelopmentRole::SolutionsArchitect => {
                "You are a Solutions Architect designing customer-facing solutions. Focus on requirements \
                 alignment, integration patterns, scalability, cost optimization, and technical feasibility.".to_string()
            }
            
            DevelopmentRole::ComplianceOfficer => {
                "You are a Compliance Officer ensuring regulatory adherence. Focus on data privacy, \
                 audit trails, regulatory requirements, compliance documentation, and risk assessment.".to_string()
            }
            
            DevelopmentRole::SecurityAuditor => {
                "You are a Security Auditor performing comprehensive security assessments. Focus on \
                 vulnerability scanning, penetration testing results, security controls evaluation, \
                 compliance verification, and remediation recommendations.".to_string()
            }
            
            DevelopmentRole::Custom(_) => {
                // Handled at the beginning of the function
                unreachable!()
            }
        }
    }
    
    /// Get analysis focus areas for a role
    pub fn get_focus_areas(role: &DevelopmentRole) -> Vec<String> {
        match role {
            DevelopmentRole::SoftwareArchitect => vec![
                "System design patterns".to_string(),
                "Component coupling and cohesion".to_string(),
                "Scalability bottlenecks".to_string(),
                "Technology choices".to_string(),
                "Non-functional requirements".to_string(),
            ],
            
            DevelopmentRole::SecurityEngineer => vec![
                "Authentication and authorization".to_string(),
                "Input validation".to_string(),
                "Data encryption".to_string(),
                "Security vulnerabilities".to_string(),
                "Compliance requirements".to_string(),
            ],
            
            DevelopmentRole::DevOpsEngineer => vec![
                "Deployment automation".to_string(),
                "Infrastructure configuration".to_string(),
                "Monitoring and alerting".to_string(),
                "CI/CD pipelines".to_string(),
                "Disaster recovery".to_string(),
            ],
            
            DevelopmentRole::QAEngineer => vec![
                "Test coverage".to_string(),
                "Edge cases".to_string(),
                "Test strategies".to_string(),
                "Quality metrics".to_string(),
                "Bug patterns".to_string(),
            ],
            
            _ => vec!["General analysis".to_string()],
        }
    }
    
    /// Get questions a role would ask about the project
    pub fn get_role_questions(role: &DevelopmentRole) -> Vec<String> {
        match role {
            DevelopmentRole::SoftwareArchitect => vec![
                "What are the main architectural patterns used?".to_string(),
                "How is the system designed to scale?".to_string(),
                "What are the main technical risks?".to_string(),
                "How is technical debt being managed?".to_string(),
                "What are the integration points and dependencies?".to_string(),
            ],
            
            DevelopmentRole::SecurityEngineer => vec![
                "How is authentication implemented?".to_string(),
                "What sensitive data is being handled?".to_string(),
                "Are there any known security vulnerabilities?".to_string(),
                "How is data encrypted at rest and in transit?".to_string(),
                "What compliance requirements must be met?".to_string(),
            ],
            
            DevelopmentRole::ProductManager => vec![
                "What user problems does this solve?".to_string(),
                "What are the key success metrics?".to_string(),
                "Who are the target users?".to_string(),
                "What is the go-to-market strategy?".to_string(),
                "How does this compare to competitors?".to_string(),
            ],
            
            DevelopmentRole::DevOpsEngineer => vec![
                "How is the application deployed?".to_string(),
                "What monitoring is in place?".to_string(),
                "How are incidents handled?".to_string(),
                "What is the disaster recovery plan?".to_string(),
                "How is infrastructure managed?".to_string(),
            ],
            
            _ => vec!["What are the main challenges in this project?".to_string()],
        }
    }
    
    /// Get the review checklist for a role
    pub fn get_review_checklist(role: &DevelopmentRole) -> Vec<ChecklistItem> {
        match role {
            DevelopmentRole::SoftwareArchitect => vec![
                ChecklistItem::new("Clear separation of concerns", Priority::High),
                ChecklistItem::new("Appropriate design patterns used", Priority::High),
                ChecklistItem::new("Scalability considered", Priority::High),
                ChecklistItem::new("Dependencies well managed", Priority::Medium),
                ChecklistItem::new("Documentation of decisions", Priority::Medium),
            ],
            
            DevelopmentRole::SecurityEngineer => vec![
                ChecklistItem::new("No hardcoded secrets", Priority::Critical),
                ChecklistItem::new("Input validation present", Priority::Critical),
                ChecklistItem::new("Authentication properly implemented", Priority::Critical),
                ChecklistItem::new("Data encryption in place", Priority::High),
                ChecklistItem::new("Security headers configured", Priority::Medium),
            ],
            
            DevelopmentRole::QAEngineer => vec![
                ChecklistItem::new("Unit tests present", Priority::High),
                ChecklistItem::new("Integration tests written", Priority::High),
                ChecklistItem::new("Edge cases covered", Priority::Medium),
                ChecklistItem::new("Test data management", Priority::Medium),
                ChecklistItem::new("Performance tests defined", Priority::Low),
            ],
            
            _ => vec![ChecklistItem::new("Basic requirements met", Priority::High)],
        }
    }
}

/// Convert role enum to string representation
fn role_to_string(role: DevelopmentRole) -> String {
    match role {
        DevelopmentRole::SoftwareArchitect => "Software Architect".to_string(),
        DevelopmentRole::BackendEngineer => "Backend Engineer".to_string(),
        DevelopmentRole::FrontendEngineer => "Frontend Engineer".to_string(),
        DevelopmentRole::FullStackEngineer => "Full Stack Engineer".to_string(),
        DevelopmentRole::DevOpsEngineer => "DevOps Engineer".to_string(),
        DevelopmentRole::DataEngineer => "Data Engineer".to_string(),
        DevelopmentRole::MLEngineer => "ML Engineer".to_string(),
        DevelopmentRole::MobileEngineer => "Mobile Engineer".to_string(),
        DevelopmentRole::EmbeddedEngineer => "Embedded Engineer".to_string(),
        DevelopmentRole::QAEngineer => "QA Engineer".to_string(),
        DevelopmentRole::TestAutomationEngineer => "Test Automation Engineer".to_string(),
        DevelopmentRole::PerformanceEngineer => "Performance Engineer".to_string(),
        DevelopmentRole::SecurityEngineer => "Security Engineer".to_string(),
        DevelopmentRole::TechLead => "Tech Lead".to_string(),
        DevelopmentRole::EngineeringManager => "Engineering Manager".to_string(),
        DevelopmentRole::ProductManager => "Product Manager".to_string(),
        DevelopmentRole::ProjectManager => "Project Manager".to_string(),
        DevelopmentRole::ScrumMaster => "Scrum Master".to_string(),
        DevelopmentRole::DatabaseAdministrator => "Database Administrator".to_string(),
        DevelopmentRole::SystemAdministrator => "System Administrator".to_string(),
        DevelopmentRole::CloudArchitect => "Cloud Architect".to_string(),
        DevelopmentRole::UIUXDesigner => "UI/UX Designer".to_string(),
        DevelopmentRole::TechnicalWriter => "Technical Writer".to_string(),
        DevelopmentRole::DataScientist => "Data Scientist".to_string(),
        DevelopmentRole::BusinessAnalyst => "Business Analyst".to_string(),
        DevelopmentRole::CustomerSupport => "Customer Support".to_string(),
        DevelopmentRole::SolutionsArchitect => "Solutions Architect".to_string(),
        DevelopmentRole::ComplianceOfficer => "Compliance Officer".to_string(),
        DevelopmentRole::SecurityAuditor => "Security Auditor".to_string(),
        DevelopmentRole::Custom(name) => return name.clone(),
    }
}

/// Checklist item for role-based review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub description: String,
    pub priority: Priority,
    pub checked: bool,
}

impl ChecklistItem {
    pub fn new(description: &str, priority: Priority) -> Self {
        Self {
            description: description.to_string(),
            priority,
            checked: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Role-based analysis context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleContext {
    pub role: DevelopmentRole,
    pub focus_areas: Vec<String>,
    pub questions: Vec<String>,
    pub checklist: Vec<ChecklistItem>,
}

impl RoleContext {
    pub fn new(role: DevelopmentRole) -> Self {
        let focus_areas = RolePrompts::get_focus_areas(&role);
        let questions = RolePrompts::get_role_questions(&role);
        let checklist = RolePrompts::get_review_checklist(&role);
        Self {
            role,
            focus_areas,
            questions,
            checklist,
        }
    }
    
    /// Get the complete prompt for analysis
    pub fn get_analysis_prompt(&self, project_context: &str) -> String {
        format!(
            "{}\n\n\
             Project Context:\n{}\n\n\
             Focus Areas:\n{}\n\n\
             Key Questions to Address:\n{}\n\n\
             Please provide a comprehensive analysis from this role's perspective.",
            RolePrompts::get_system_prompt(&self.role),
            project_context,
            self.focus_areas.join("\n- "),
            self.questions.join("\n- ")
        )
    }
}

/// Multi-role analysis orchestrator
pub struct MultiRoleAnalyzer {
    roles: Vec<DevelopmentRole>,
}

impl MultiRoleAnalyzer {
    pub fn new(roles: Vec<DevelopmentRole>) -> Self {
        Self { roles }
    }
    
    /// Get all role contexts for analysis
    pub fn get_all_contexts(&self) -> Vec<RoleContext> {
        self.roles.iter()
            .map(|role| RoleContext::new(role.clone()))
            .collect()
    }
    
    /// Get critical roles for a project type
    pub fn get_critical_roles(project_type: &str) -> Vec<DevelopmentRole> {
        match project_type.to_lowercase().as_str() {
            "web" | "webapp" => vec![
                DevelopmentRole::FrontendEngineer,
                DevelopmentRole::BackendEngineer,
                DevelopmentRole::DevOpsEngineer,
                DevelopmentRole::SecurityEngineer,
            ],
            
            "api" | "backend" => vec![
                DevelopmentRole::BackendEngineer,
                DevelopmentRole::DatabaseAdministrator,
                DevelopmentRole::SecurityEngineer,
                DevelopmentRole::DevOpsEngineer,
            ],
            
            "mobile" => vec![
                DevelopmentRole::MobileEngineer,
                DevelopmentRole::BackendEngineer,
                DevelopmentRole::UIUXDesigner,
                DevelopmentRole::QAEngineer,
            ],
            
            "data" | "pipeline" => vec![
                DevelopmentRole::DataEngineer,
                DevelopmentRole::DatabaseAdministrator,
                DevelopmentRole::DevOpsEngineer,
                DevelopmentRole::DataScientist,
            ],
            
            "ml" | "ai" => vec![
                DevelopmentRole::MLEngineer,
                DevelopmentRole::DataEngineer,
                DevelopmentRole::DevOpsEngineer,
                DevelopmentRole::DataScientist,
            ],
            
            _ => vec![
                DevelopmentRole::SoftwareArchitect,
                DevelopmentRole::TechLead,
                DevelopmentRole::QAEngineer,
                DevelopmentRole::DevOpsEngineer,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_role_prompts() {
        let prompt = RolePrompts::get_system_prompt(DevelopmentRole::SecurityEngineer);
        assert!(prompt.contains("Security Engineer"));
        assert!(prompt.contains("OWASP"));
    }
    
    #[test]
    fn test_role_context() {
        let context = RoleContext::new(DevelopmentRole::SoftwareArchitect);
        assert!(!context.focus_areas.is_empty());
        assert!(!context.questions.is_empty());
        assert!(!context.checklist.is_empty());
    }
    
    #[test]
    fn test_critical_roles() {
        let web_roles = MultiRoleAnalyzer::get_critical_roles("web");
        assert!(web_roles.contains(&DevelopmentRole::FrontendEngineer));
        assert!(web_roles.contains(&DevelopmentRole::BackendEngineer));
        
        let ml_roles = MultiRoleAnalyzer::get_critical_roles("ml");
        assert!(ml_roles.contains(&DevelopmentRole::MLEngineer));
        assert!(ml_roles.contains(&DevelopmentRole::DataEngineer));
    }
}
