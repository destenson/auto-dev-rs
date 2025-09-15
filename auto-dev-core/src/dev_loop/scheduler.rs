//! Task scheduling for the development loop

use anyhow::Result;
use chrono::{DateTime, Utc};
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, trace};
use uuid::Uuid;

/// Manages scheduled tasks
pub struct TaskScheduler {
    tasks: Arc<RwLock<PriorityQueue<ScheduledTask, TaskPriority>>>,
    recurring_tasks: Arc<RwLock<HashMap<String, RecurringTask>>>,
    task_history: Arc<RwLock<Vec<TaskExecution>>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(PriorityQueue::new())),
            recurring_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Schedule a one-time task
    pub async fn schedule_task(&self, task: ScheduledTask) -> Result<String> {
        let task_id = task.id.clone();
        let priority = TaskPriority::from(&task);
        
        debug!("Scheduling task: {} at {:?}", task.name, task.scheduled_time);
        
        let mut tasks = self.tasks.write().await;
        tasks.push(task, priority);
        
        Ok(task_id)
    }

    /// Schedule a recurring task
    pub async fn schedule_recurring(
        &self, 
        name: String, 
        interval: Duration,
        task_fn: Arc<dyn TaskFunction>,
    ) -> Result<String> {
        let task_id = Uuid::new_v4().to_string();
        
        info!("Scheduling recurring task: {} every {:?}", name, interval);
        
        let recurring = RecurringTask {
            id: task_id.clone(),
            name: name.clone(),
            interval,
            last_run: None,
            next_run: Utc::now(),
            task_fn,
            enabled: true,
        };
        
        let mut recurring_tasks = self.recurring_tasks.write().await;
        recurring_tasks.insert(task_id.clone(), recurring);
        
        Ok(task_id)
    }

    /// Get next task to execute
    pub async fn next_task(&self) -> Option<ScheduledTask> {
        // Check recurring tasks first
        if let Some(task) = self.check_recurring_tasks().await {
            return Some(task);
        }
        
        // Check scheduled tasks
        let mut tasks = self.tasks.write().await;
        
        if let Some((task, _)) = tasks.peek() {
            if task.scheduled_time <= Utc::now() {
                return tasks.pop().map(|(task, _)| task);
            }
        }
        
        None
    }

    /// Check and execute recurring tasks
    async fn check_recurring_tasks(&self) -> Option<ScheduledTask> {
        let mut recurring_tasks = self.recurring_tasks.write().await;
        let now = Utc::now();
        
        for task in recurring_tasks.values_mut() {
            if task.enabled && task.next_run <= now {
                task.last_run = Some(now);
                task.next_run = now + chrono::Duration::from_std(task.interval).unwrap();
                
                return Some(ScheduledTask {
                    id: task.id.clone(),
                    name: task.name.clone(),
                    scheduled_time: now,
                    task_type: TaskType::Recurring,
                    task_fn: task.task_fn.clone(),
                    metadata: HashMap::new(),
                });
            }
        }
        
        None
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        // Check scheduled tasks
        {
            let mut tasks = self.tasks.write().await;
            let tasks_vec: Vec<_> = tasks.clone().into_sorted_vec();
            tasks.clear();
            
            let mut found = false;
            for (task, priority) in tasks_vec {
                if task.id != task_id {
                    tasks.push(task, priority);
                } else {
                    found = true;
                    debug!("Cancelled scheduled task: {}", task_id);
                }
            }
            
            if found {
                return Ok(true);
            }
        }
        
        // Check recurring tasks
        {
            let mut recurring = self.recurring_tasks.write().await;
            if recurring.remove(task_id).is_some() {
                debug!("Cancelled recurring task: {}", task_id);
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Pause a recurring task
    pub async fn pause_recurring(&self, task_id: &str) -> Result<bool> {
        let mut recurring = self.recurring_tasks.write().await;
        
        if let Some(task) = recurring.get_mut(task_id) {
            task.enabled = false;
            debug!("Paused recurring task: {}", task_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Resume a recurring task
    pub async fn resume_recurring(&self, task_id: &str) -> Result<bool> {
        let mut recurring = self.recurring_tasks.write().await;
        
        if let Some(task) = recurring.get_mut(task_id) {
            task.enabled = true;
            task.next_run = Utc::now();
            debug!("Resumed recurring task: {}", task_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Record task execution
    pub async fn record_execution(
        &self, 
        task_id: String, 
        success: bool, 
        duration: Duration,
        error: Option<String>,
    ) {
        let mut history = self.task_history.write().await;
        
        history.push(TaskExecution {
            task_id,
            timestamp: Utc::now(),
            success,
            duration,
            error,
        });
        
        // Keep only last 1000 executions
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    /// Get task statistics
    pub async fn get_statistics(&self) -> TaskStatistics {
        let history = self.task_history.read().await;
        let tasks = self.tasks.read().await;
        let recurring = self.recurring_tasks.read().await;
        
        let total_executions = history.len();
        let successful_executions = history.iter().filter(|e| e.success).count();
        let failed_executions = total_executions - successful_executions;
        
        let avg_duration = if !history.is_empty() {
            let total: u64 = history.iter().map(|e| e.duration.as_millis() as u64).sum();
            Duration::from_millis(total / history.len() as u64)
        } else {
            Duration::from_secs(0)
        };
        
        TaskStatistics {
            pending_tasks: tasks.len(),
            recurring_tasks: recurring.len(),
            total_executions,
            successful_executions,
            failed_executions,
            average_duration: avg_duration,
        }
    }
}

/// A scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub scheduled_time: DateTime<Utc>,
    pub task_type: TaskType,
    #[serde(skip)]
    pub task_fn: Arc<dyn TaskFunction>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ScheduledTask {
    pub fn new(name: String, scheduled_time: DateTime<Utc>, task_fn: Arc<dyn TaskFunction>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            scheduled_time,
            task_type: TaskType::OneTime,
            task_fn,
            metadata: HashMap::new(),
        }
    }

    /// Execute the task
    pub async fn execute(&self) -> Result<()> {
        trace!("Executing task: {}", self.name);
        self.task_fn.execute().await
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScheduledTask {}

impl std::hash::Hash for ScheduledTask {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Task type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    OneTime,
    Recurring,
    Delayed,
    Conditional,
}

/// Task priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskPriority {
    scheduled_time: i64,
    priority_level: i32,
}

impl From<&ScheduledTask> for TaskPriority {
    fn from(task: &ScheduledTask) -> Self {
        Self {
            scheduled_time: -task.scheduled_time.timestamp(), // Negative for min-heap behavior
            priority_level: match task.task_type {
                TaskType::OneTime => 0,
                TaskType::Recurring => 1,
                TaskType::Delayed => 2,
                TaskType::Conditional => 3,
            },
        }
    }
}

impl Ord for TaskPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.scheduled_time
            .cmp(&other.scheduled_time)
            .then(self.priority_level.cmp(&other.priority_level))
    }
}

impl PartialOrd for TaskPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Recurring task definition
struct RecurringTask {
    id: String,
    name: String,
    interval: Duration,
    last_run: Option<DateTime<Utc>>,
    next_run: DateTime<Utc>,
    task_fn: Arc<dyn TaskFunction>,
    enabled: bool,
}

/// Task function trait
#[async_trait::async_trait]
pub trait TaskFunction: Send + Sync {
    async fn execute(&self) -> Result<()>;
}

/// Simple task function implementation
struct SimpleTaskFunction<F>
where
    F: Fn() -> Result<()> + Send + Sync,
{
    func: F,
}

#[async_trait::async_trait]
impl<F> TaskFunction for SimpleTaskFunction<F>
where
    F: Fn() -> Result<()> + Send + Sync,
{
    async fn execute(&self) -> Result<()> {
        (self.func)()
    }
}

/// Task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExecution {
    task_id: String,
    timestamp: DateTime<Utc>,
    success: bool,
    duration: Duration,
    error: Option<String>,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatistics {
    pub pending_tasks: usize,
    pub recurring_tasks: usize,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTask;
    
    #[async_trait::async_trait]
    impl TaskFunction for TestTask {
        async fn execute(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_task_scheduler() {
        let scheduler = TaskScheduler::new();
        
        // Schedule a task
        let task = ScheduledTask::new(
            "test_task".to_string(),
            Utc::now() + chrono::Duration::seconds(1),
            Arc::new(TestTask),
        );
        
        let task_id = scheduler.schedule_task(task).await.unwrap();
        assert!(!task_id.is_empty());
        
        // Check statistics
        let stats = scheduler.get_statistics().await;
        assert_eq!(stats.pending_tasks, 1);
    }

    #[tokio::test]
    async fn test_recurring_task() {
        let scheduler = TaskScheduler::new();
        
        // Schedule recurring task
        let task_id = scheduler.schedule_recurring(
            "recurring_test".to_string(),
            Duration::from_secs(60),
            Arc::new(TestTask),
        ).await.unwrap();
        
        // Pause and resume
        assert!(scheduler.pause_recurring(&task_id).await.unwrap());
        assert!(scheduler.resume_recurring(&task_id).await.unwrap());
        
        // Cancel
        assert!(scheduler.cancel_task(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_task_execution() {
        let scheduler = TaskScheduler::new();
        
        // Schedule immediate task
        let task = ScheduledTask::new(
            "immediate_task".to_string(),
            Utc::now(),
            Arc::new(TestTask),
        );
        
        scheduler.schedule_task(task).await.unwrap();
        
        // Get next task
        let next = scheduler.next_task().await;
        assert!(next.is_some());
        
        if let Some(task) = next {
            assert_eq!(task.name, "immediate_task");
            task.execute().await.unwrap();
        }
    }
}