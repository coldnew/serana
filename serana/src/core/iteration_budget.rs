//! Iteration budget management for preventing infinite loops.
//!
//! Tracks the number of agent iterations and enforces configurable limits.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Default maximum iterations for parent agents.
pub const DEFAULT_MAX_ITERATIONS: usize = 50;

/// Default maximum iterations for subagents.
pub const DEFAULT_SUBAGENT_MAX_ITERATIONS: usize = 20;

/// Token cost tracking for budget calculations.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCost {
    pub input: usize,
    pub output: usize,
    pub cache_read: usize,
}

impl TokenCost {
    pub fn total(&self) -> usize {
        self.input + self.output + self.cache_read
    }
}

/// Iteration budget configuration and state.
#[derive(Debug)]
pub struct IterationBudget {
    max_iterations: usize,
    current: AtomicUsize,
    enabled: bool,
}

impl IterationBudget {
    /// Create a new iteration budget with the given limit.
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            current: AtomicUsize::new(0),
            enabled: true,
        }
    }

    /// Create a budget with default parent agent settings.
    pub fn default_parent() -> Self {
        Self::new(DEFAULT_MAX_ITERATIONS)
    }

    /// Create a budget with default subagent settings.
    pub fn default_subagent() -> Self {
        Self::new(DEFAULT_SUBAGENT_MAX_ITERATIONS)
    }

    /// Create an unlimited budget (for testing or manual control).
    pub fn unlimited() -> Self {
        Self {
            max_iterations: usize::MAX,
            current: AtomicUsize::new(0),
            enabled: false,
        }
    }

    /// Check if the budget allows another iteration.
    pub fn can_continue(&self) -> bool {
        !self.enabled || self.current.load(Ordering::SeqCst) < self.max_iterations
    }

    pub fn increment(&self) -> bool {
        let new_val = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        new_val >= self.max_iterations
    }

    /// Get the current iteration count.
    pub fn current(&self) -> usize {
        self.current.load(Ordering::SeqCst)
    }

    /// Get the maximum iteration limit.
    pub fn max(&self) -> usize {
        self.max_iterations
    }

    /// Get remaining iterations.
    pub fn remaining(&self) -> usize {
        self.max_iterations
            .saturating_sub(self.current.load(Ordering::SeqCst))
    }

    /// Check if budget is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.enabled && self.current.load(Ordering::SeqCst) >= self.max_iterations
    }

    /// Reset the budget (for new sessions).
    pub fn reset(&self) {
        self.current.store(0, Ordering::SeqCst);
    }

    /// Get a progress ratio (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.max_iterations == 0 {
            return 1.0;
        }
        (self.current.load(Ordering::SeqCst) as f64) / (self.max_iterations as f64)
    }
}

impl Clone for IterationBudget {
    fn clone(&self) -> Self {
        Self {
            max_iterations: self.max_iterations,
            current: AtomicUsize::new(self.current.load(Ordering::SeqCst)),
            enabled: self.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_allows_iterations_within_limit() {
        let budget = IterationBudget::new(5);

        for _ in 0..4 {
            assert!(budget.can_continue());
            assert!(!budget.increment());
        }

        assert!(budget.increment());
        assert!(!budget.can_continue());
        assert!(budget.is_exhausted());
    }

    #[test]
    fn unlimited_budget_never_exhausts() {
        let budget = IterationBudget::unlimited();

        for _ in 0..1000 {
            assert!(budget.can_continue());
            assert!(!budget.increment());
        }

        assert!(!budget.is_exhausted());
    }

    #[test]
    fn reset_clears_state() {
        let budget = IterationBudget::new(5);

        budget.increment();
        budget.increment();

        budget.reset();

        assert_eq!(budget.current(), 0);
        assert!(budget.can_continue());
    }

    #[test]
    fn calculates_progress() {
        let budget = IterationBudget::new(10);

        assert_eq!(budget.progress(), 0.0);

        for _ in 0..5 {
            budget.increment();
        }

        assert_eq!(budget.progress(), 0.5);

        for _ in 0..5 {
            budget.increment();
        }

        assert_eq!(budget.progress(), 1.0);
    }
}
