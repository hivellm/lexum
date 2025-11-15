//! Work stealing queue for efficient task distribution
//!
//! This module provides a work-stealing queue implementation for distributing
//! tasks across multiple workers efficiently.

use crossbeam::deque::{Injector, Steal, Stealer, Worker};
use std::sync::Arc;

/// Work stealing queue for task distribution
pub struct WorkStealingQueue<T> {
    /// Global injector for new tasks
    injector: Arc<Injector<T>>,
    /// Number of workers
    num_workers: usize,
    /// Worker queues (one per worker)
    workers: Vec<Arc<Worker<T>>>,
    /// Stealers for other workers (for work stealing)
    stealers: Vec<Stealer<T>>,
}

impl<T> WorkStealingQueue<T> {
    /// Create a new work stealing queue with the specified number of workers
    pub fn new(num_workers: usize) -> Self {
        let injector = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(num_workers);
        let mut stealers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let worker = Arc::new(Worker::new_fifo());
            stealers.push(worker.stealer());
            workers.push(worker);
        }

        Self {
            injector,
            num_workers,
            workers,
            stealers,
        }
    }

    /// Push a task to the queue
    pub fn push(&self, task: T) {
        // Push to global injector for work stealing
        // Workers will steal from injector when their local queue is empty
        self.injector.push(task);
    }

    /// Pop a task from the queue (for a specific worker)
    pub fn pop(&self, worker_id: usize) -> Option<T> {
        // Try local queue first
        if let Some(task) = self.workers[worker_id].pop() {
            return Some(task);
        }

        // Try global injector
        loop {
            match self.injector.steal_batch_and_pop(&self.workers[worker_id]) {
                Steal::Success(task) => return Some(task),
                Steal::Retry => {}
                Steal::Empty => break,
            }
        }

        // Try stealing from other workers
        for (idx, stealer) in self.stealers.iter().enumerate() {
            if idx != worker_id {
                match stealer.steal_batch_and_pop(&self.workers[worker_id]) {
                    Steal::Success(task) => return Some(task),
                    Steal::Retry | Steal::Empty => {}
                }
            }
        }

        None
    }

    /// Get the number of workers
    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        // Check all workers
        for worker in &self.workers {
            if !worker.is_empty() {
                return false;
            }
        }
        // Check injector
        self.injector.is_empty()
    }

    /// Get approximate queue size
    pub fn len(&self) -> usize {
        let mut size = 0;
        for worker in &self.workers {
            size += worker.len();
        }
        // Injector doesn't expose len, so we approximate
        size
    }
}

impl<T> Default for WorkStealingQueue<T> {
    fn default() -> Self {
        let num_workers = num_cpus::get();
        Self::new(num_workers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_stealing_queue_creation() {
        let queue = WorkStealingQueue::<usize>::new(4);
        assert_eq!(queue.num_workers(), 4);
    }

    #[test]
    fn test_work_stealing_queue_push_pop() {
        let queue = WorkStealingQueue::new(2);
        queue.push(1);
        queue.push(2);
        queue.push(3);

        // Worker 0 should be able to pop tasks
        assert!(queue.pop(0).is_some());
        assert!(queue.pop(0).is_some());
        assert!(queue.pop(0).is_some());
    }

    #[test]
    fn test_work_stealing_queue_empty() {
        let queue = WorkStealingQueue::<usize>::new(2);
        assert!(queue.is_empty());

        queue.push(1);
        assert!(!queue.is_empty());

        let _ = queue.pop(0);
        assert!(queue.is_empty());
    }
}
