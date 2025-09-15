//! Main orchestration logic for the continuous development loop

use super::*;
use super::scheduler;
use anyhow::Result;
use priority_queue::PriorityQueue;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Main orchestrator for the development loop
pub struct Orchestrator {
    state: Arc<RwLock<LoopState>>,
    event_queue: Arc<Mutex<PriorityQueue<Event, Priority>>>,
    decision_engine: Arc<DecisionEngine>,
    llm_optimizer: Arc<LLMOptimizer>,
    health_monitor: Arc<HealthMonitor>,
    event_processor: Arc<EventProcessor>,
    scheduler: Arc<scheduler::TaskScheduler>,
    config: LoopConfig,
    metrics: Arc<RwLock<LoopMetrics>>,
    shutdown_signal: mpsc::Receiver<()>,
}

impl Orchestrator {
    pub fn new(
        config: LoopConfig,
        shutdown_signal: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(LoopState::Idle)),
            event_queue: Arc::new(Mutex::new(PriorityQueue::new())),
            decision_engine: Arc::new(DecisionEngine::new()),
            llm_optimizer: Arc::new(LLMOptimizer::new(config.llm_optimization.clone())),
            health_monitor: Arc::new(HealthMonitor::new()),
            event_processor: Arc::new(EventProcessor::new()),
            scheduler: Arc::new(scheduler::TaskScheduler::new()),
            config,
            metrics: Arc::new(RwLock::new(LoopMetrics::default())),
            shutdown_signal,
        }
    }

    /// Main run loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting development loop orchestrator");
        
        // Start health monitoring
        let health_interval = interval(Duration::from_secs(self.config.health_check_interval));
        tokio::pin!(health_interval);
        
        // Main event loop
        loop {
            tokio::select! {
                // Process events from queue
                _ = self.process_next_event() => {},
                
                // Scheduled tasks
                Some(task) = self.scheduler.next_task() => {
                    self.execute_scheduled_task(task).await?;
                },
                
                // Health checks
                _ = health_interval.tick() => {
                    self.check_health().await?;
                },
                
                // Shutdown signal
                _ = self.shutdown_signal.recv() => {
                    info!("Received shutdown signal");
                    self.graceful_shutdown().await?;
                    break;
                }
            }
        }
        
        Ok(())
    }

    /// Process the next event from the queue
    async fn process_next_event(&self) -> Result<()> {
        let event = {
            let mut queue = self.event_queue.lock().await;
            queue.pop().map(|(event, _)| event)
        };
        
        if let Some(event) = event {
            debug!("Processing event: {:?}", event.event_type);
            
            // Update state
            {
                let mut state = self.state.write().await;
                *state = LoopState::Processing(format!("Processing {:?}", event.event_type));
            }
            
            // Make decision
            let decision = self.make_decision(&event).await?;
            
            // Execute decision
            self.execute_decision(decision).await?;
            
            // Update metrics
            {
                let mut metrics = self.metrics.write().await;
                metrics.events_processed += 1;
            }
            
            // Return to idle
            {
                let mut state = self.state.write().await;
                *state = LoopState::Idle;
            }
        }
        
        Ok(())
    }

    /// Make a decision based on an event
    async fn make_decision(&self, event: &Event) -> Result<Decision> {
        // First try non-LLM decision
        let decision = self.decision_engine.decide(event).await?;
        
        match decision {
            Decision::RequiresLLM(request) => {
                // Optimize LLM usage
                self.llm_optimizer.process_requirement(request).await
            },
            _ => Ok(decision),
        }
    }

    /// Execute a decision
    async fn execute_decision(&self, decision: Decision) -> Result<()> {
        match decision {
            Decision::Implement(task) => {
                info!("Implementing specification: {:?}", task.spec_path);
                self.implement_specification(task).await?;
            },
            Decision::UpdateTests(updates) => {
                info!("Updating {} tests", updates.len());
                self.update_tests(updates).await?;
            },
            Decision::Refactor(task) => {
                info!("Refactoring: {:?}", task.file_path);
                self.refactor_code(task).await?;
            },
            Decision::Skip(reason) => {
                debug!("Skipping: {}", reason);
            },
            Decision::UsePattern(pattern_id) => {
                info!("Using pattern: {}", pattern_id);
                self.apply_pattern(pattern_id).await?;
            },
            Decision::UseTemplate(template_id) => {
                info!("Using template: {}", template_id);
                self.apply_template(template_id).await?;
            },
            Decision::UseCached(cached) => {
                info!("Using cached response: {}", cached.request_hash);
                self.apply_cached_response(cached).await?;
            },
            Decision::AdaptSimilar(similar) => {
                info!("Adapting similar solution: {}", similar.solution_id);
                self.adapt_similar_solution(similar).await?;
            },
            _ => {},
        }
        
        Ok(())
    }

    /// Implement a specification
    async fn implement_specification(&self, task: ImplementationTask) -> Result<()> {
        // This would integrate with the synthesis engine
        debug!("Implementing: {:?}", task.spec_path);
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.implementations_completed += 1;
        
        Ok(())
    }

    /// Update tests
    async fn update_tests(&self, updates: Vec<TestUpdate>) -> Result<()> {
        for update in updates {
            debug!("Updating test: {:?}", update.test_path);
        }
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.tests_generated += 1;
        
        Ok(())
    }

    /// Refactor code
    async fn refactor_code(&self, task: RefactorTask) -> Result<()> {
        debug!("Refactoring: {:?}", task.file_path);
        Ok(())
    }

    /// Apply a known pattern
    async fn apply_pattern(&self, pattern_id: String) -> Result<()> {
        debug!("Applying pattern: {}", pattern_id);
        
        // Update metrics - avoided LLM call
        let mut metrics = self.metrics.write().await;
        metrics.llm_calls_avoided += 1;
        
        Ok(())
    }

    /// Apply a template
    async fn apply_template(&self, template_id: String) -> Result<()> {
        debug!("Applying template: {}", template_id);
        
        // Update metrics - avoided LLM call
        let mut metrics = self.metrics.write().await;
        metrics.llm_calls_avoided += 1;
        
        Ok(())
    }

    /// Apply cached response
    async fn apply_cached_response(&self, cached: CachedResponse) -> Result<()> {
        debug!("Using cached response: {}", cached.request_hash);
        
        // Update metrics - avoided LLM call
        let mut metrics = self.metrics.write().await;
        metrics.llm_calls_avoided += 1;
        
        Ok(())
    }

    /// Adapt similar solution
    async fn adapt_similar_solution(&self, similar: SimilarSolution) -> Result<()> {
        debug!("Adapting solution: {} (similarity: {})", similar.solution_id, similar.similarity_score);
        
        // Update metrics - avoided LLM call
        let mut metrics = self.metrics.write().await;
        metrics.llm_calls_avoided += 1;
        
        Ok(())
    }

    /// Execute scheduled task
    async fn execute_scheduled_task(&self, task: scheduler::ScheduledTask) -> Result<()> {
        debug!("Executing scheduled task: {}", task.name);
        task.execute().await
    }

    /// Check system health
    async fn check_health(&self) -> Result<()> {
        let status = self.health_monitor.check_health().await?;
        
        if !status.is_healthy {
            warn!("System health degraded: {:?}", status.warnings);
            self.health_monitor.take_corrective_action(&status).await?;
        }
        
        Ok(())
    }

    /// Graceful shutdown
    async fn graceful_shutdown(&self) -> Result<()> {
        info!("Performing graceful shutdown");
        
        // Update state
        let mut state = self.state.write().await;
        *state = LoopState::Shutdown;
        
        // Save state
        self.save_state().await?;
        
        // Save metrics
        self.save_metrics().await?;
        
        info!("Shutdown complete");
        Ok(())
    }

    /// Save current state
    async fn save_state(&self) -> Result<()> {
        // Save to .auto-dev/loop/state.json
        debug!("Saving loop state");
        Ok(())
    }

    /// Save metrics
    async fn save_metrics(&self) -> Result<()> {
        // Save to .auto-dev/loop/metrics.json
        debug!("Saving metrics");
        Ok(())
    }

    /// Add event to queue
    pub async fn queue_event(&self, event: Event) -> Result<()> {
        let priority = event.priority;
        let mut queue = self.event_queue.lock().await;
        queue.push(event, priority);
        Ok(())
    }

    /// Get current state
    pub async fn get_state(&self) -> LoopState {
        self.state.read().await.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> LoopMetrics {
        self.metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = LoopConfig::default();
        let (_tx, rx) = mpsc::channel(1);
        let orchestrator = Orchestrator::new(config, rx);
        
        let state = orchestrator.get_state().await;
        assert_eq!(state, LoopState::Idle);
    }

    #[tokio::test]
    async fn test_event_queueing() {
        let config = LoopConfig::default();
        let (_tx, rx) = mpsc::channel(1);
        let orchestrator = Orchestrator::new(config, rx);
        
        let event = Event::new(
            EventType::SpecificationChanged,
            PathBuf::from("test.md"),
        );
        
        orchestrator.queue_event(event).await.unwrap();
        // Event should be queued
    }
}