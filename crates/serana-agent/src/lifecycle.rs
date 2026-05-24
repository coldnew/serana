use serana_core::{AgentCallbacks, AgentStatus, CancelToken, IterationBudget, Result};

pub struct AgentLifecycle<'a> {
    budget: &'a IterationBudget,
    cancel_token: Option<&'a CancelToken>,
    callbacks: &'a AgentCallbacks,
}

impl<'a> AgentLifecycle<'a> {
    pub fn new(
        budget: &'a IterationBudget,
        cancel_token: Option<&'a CancelToken>,
        callbacks: &'a AgentCallbacks,
    ) -> Self {
        Self {
            budget,
            cancel_token,
            callbacks,
        }
    }

    pub fn can_continue(&self) -> bool {
        self.budget.can_continue()
    }

    pub fn check_cancelled(&self) -> Result<()> {
        if self
            .cancel_token
            .map(|token| token.is_cancelled())
            .unwrap_or(false)
        {
            self.callbacks.fire_status(AgentStatus::Idle);
            anyhow::bail!("Execution cancelled by user");
        }
        Ok(())
    }

    pub fn complete_tool_iteration(&self) -> bool {
        let exhausted = self.budget.increment();
        if exhausted {
            self.fire_budget_exhausted();
        }
        exhausted
    }

    pub fn fire_budget_exhausted(&self) {
        self.callbacks.fire_status(AgentStatus::BudgetExhausted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn reports_cancelled_runs() {
        let budget = IterationBudget::new(1);
        let token = CancelToken::new();
        token.cancel();
        let callbacks = AgentCallbacks::new();
        let lifecycle = AgentLifecycle::new(&budget, Some(&token), &callbacks);

        let err = lifecycle.check_cancelled().unwrap_err();

        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn completes_tool_iteration_and_fires_budget_status() {
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_cb = statuses.clone();
        let callbacks = AgentCallbacks::new().with_status(Arc::new(move |status| {
            statuses_for_cb.lock().unwrap().push(status);
        }));
        let budget = IterationBudget::new(1);
        let lifecycle = AgentLifecycle::new(&budget, None, &callbacks);

        assert!(lifecycle.complete_tool_iteration());
        assert_eq!(
            statuses.lock().unwrap().as_slice(),
            &[AgentStatus::BudgetExhausted]
        );
    }
}
