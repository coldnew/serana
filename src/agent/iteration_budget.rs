//! Iteration budget tracking for agent loops.
//!
//! Prevents infinite loops by capping the number of agent turns.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Default maximum iterations for a parent agent.
pub const DEFAULT_MAX_ITERATIONS: u32 = 90;

/// Default maximum iterations for a delegated subagent.
pub const DEFAULT_SUBAGENT_MAX_ITERATIONS: u32 = 50;

/// Tracks iteration count with atomic operations for thread safety.
#[derive(Debug, Clone)]
pub struct IterationBudget {
    current: Arc<AtomicU32>,
    max: u32,
}

impl IterationBudget {
    /// Create a new budget with the given maximum.
    pub fn new(max: u32) -> Self {
        Self {
            current: Arc::new(AtomicU32::new(0)),
            max,
        }
    }

    /// Create a budget with default parent agent limits.
    pub fn default_parent() -> Self {
        Self::new(DEFAULT_MAX_ITERATIONS)
    }

    /// Create a budget with default subagent limits.
    pub fn default_subagent() -> Self {
        Self::new(DEFAULT_SUBAGENT_MAX_ITERATIONS)
    }

    /// Increment the iteration count. Returns true if budget exceeded.
    pub fn increment(&self) -> bool {
        let prev = self.current.fetch_add(1, Ordering::SeqCst);
        prev.saturating_add(1) >= self.max
    }

    /// Check if budget is exceeded without incrementing.
    pub fn is_exceeded(&self) -> bool {
        self.current.load(Ordering::SeqCst) >= self.max
    }

    /// Get current iteration count.
    pub fn current(&self) -> u32 {
        self.current.load(Ordering::SeqCst)
    }

    /// Get maximum allowed iterations.
    pub fn max(&self) -> u32 {
        self.max
    }

    /// Get remaining iterations.
    pub fn remaining(&self) -> u32 {
        self.max.saturating_sub(self.current())
    }

    /// Get percentage of budget used (0-100).
    pub fn percentage(&self) -> u32 {
        if self.max == 0 {
            return 100;
        }
        (self.current() * 100 / self.max).min(100)
    }

    /// Reset the budget (for reuse).
    pub fn reset(&self) {
        self.current.store(0, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_tracking() {
        let budget = IterationBudget::new(5);
        assert_eq!(budget.current(), 0);
        assert!(!budget.is_exceeded());

        for i in 1..=4 {
            assert!(!budget.increment(), "Should not exceed at iteration {}", i);
        }
        assert_eq!(budget.current(), 4);
        assert!(!budget.is_exceeded());

        assert!(budget.increment(), "Should exceed at iteration 5");
        assert!(budget.is_exceeded());
    }

    #[test]
    fn test_percentage() {
        let budget = IterationBudget::new(100);
        assert_eq!(budget.percentage(), 0);

        budget.increment();
        assert_eq!(budget.percentage(), 1);

        for _ in 0..49 {
            budget.increment();
        }
        assert_eq!(budget.percentage(), 50);
    }

    #[test]
    fn test_default_budgets() {
        let parent = IterationBudget::default_parent();
        assert_eq!(parent.max(), DEFAULT_MAX_ITERATIONS);

        let subagent = IterationBudget::default_subagent();
        assert_eq!(subagent.max(), DEFAULT_SUBAGENT_MAX_ITERATIONS);
    }
}
