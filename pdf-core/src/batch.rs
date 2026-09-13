use std::path::PathBuf;

use rayon::prelude::*;

/// Outcome of processing a single item in a batch.
#[derive(Debug)]
pub struct BatchItemResult {
    /// The input path this result corresponds to.
    pub input: PathBuf,
    /// `Ok` with the produced output path(s), or `Err` with a message.
    pub outcome: std::result::Result<Vec<PathBuf>, String>,
}

impl BatchItemResult {
    pub fn is_ok(&self) -> bool {
        self.outcome.is_ok()
    }
}

/// Aggregate report for a batch run.
#[derive(Debug)]
pub struct BatchReport {
    pub results: Vec<BatchItemResult>,
}

impl BatchReport {
    pub fn succeeded(&self) -> usize {
        self.results.iter().filter(|r| r.is_ok()).count()
    }

    pub fn failed(&self) -> usize {
        self.results.len() - self.succeeded()
    }

    pub fn all_ok(&self) -> bool {
        self.failed() == 0
    }
}

/// Run `op` over every input in parallel, isolating failures: one file's error
/// never aborts the others. Results are returned in the original input order.
///
/// `op` returns the output path(s) it produced for that input.
pub fn run_batch<I, F>(inputs: Vec<I>, op: F) -> BatchReport
where
    I: Into<PathBuf> + Send,
    F: Fn(&std::path::Path) -> crate::error::Result<Vec<PathBuf>> + Sync + Send,
{
    let inputs: Vec<PathBuf> = inputs.into_iter().map(Into::into).collect();

    let results = inputs
        .par_iter()
        .map(|input| {
            let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| op(input)))
            {
                Ok(Ok(outputs)) => Ok(outputs),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err("operation panicked".to_string()),
            };
            BatchItemResult {
                input: input.clone(),
                outcome,
            }
        })
        .collect();

    BatchReport { results }
}
