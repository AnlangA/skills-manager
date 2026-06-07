//! Installation operation journaling and rollback plan primitives.
//!
//! Provides [`OperationPlan`], a serializable representation of a staged
//! install operation, and an internal journal that records filesystem
//! mutations so they can be reversed on error.

use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    InstallCandidate, InstallPreview, InstallRequest, Result, SkillScope, SkillsManagerError,
};

/// Serializable plan describing a staged install operation.
#[derive(Debug, Clone, Serialize)]
pub struct OperationPlan {
    /// Deterministic identifier for logs and persistence.
    pub id: String,
    /// Original install request used to build the plan.
    pub request: InstallRequest,
    /// Computed target scope.
    pub scope: SkillScope,
    /// Computed destination root.
    pub destination_root: PathBuf,
    /// Planned candidates, one per discovered skill.
    pub candidates: Vec<InstallCandidate>,
    /// Timestamp when the plan was created.
    pub created_at: DateTime<Utc>,
}

impl OperationPlan {
    /// Creates a new operation plan from request parameters.
    pub fn new(
        request: InstallRequest,
        scope: SkillScope,
        destination_root: PathBuf,
        candidates: Vec<InstallCandidate>,
    ) -> Self {
        let created_at = Utc::now();
        let id = format!(
            "{}:{}:{}",
            scope.id_prefix(),
            destination_root.display(),
            created_at.timestamp_millis()
        );

        Self {
            id,
            request,
            scope,
            destination_root,
            candidates,
            created_at,
        }
    }

    /// Returns a preview payload that carries the same candidate list with context.
    pub fn preview(&self) -> InstallPreview {
        InstallPreview {
            scope: self.scope,
            destination_root: self.destination_root.clone(),
            candidates: self.candidates.clone(),
            operation_plan: Some(self.clone()),
        }
    }
}

/// Tracks files and directories created/moved during an operation for rollback.
#[derive(Debug, Default)]
pub(crate) struct OperationJournal {
    created_roots: Vec<PathBuf>,
    moved_roots: Vec<(PathBuf, PathBuf)>,
}

impl OperationJournal {
    /// Records a newly created path that should be removed on rollback.
    pub fn record_created(&mut self, path: PathBuf) {
        self.created_roots.push(path);
    }

    /// Records a move from `from` to `to` that should be reversed on rollback.
    pub fn record_move(&mut self, from: PathBuf, to: PathBuf) {
        self.moved_roots.push((from, to));
    }

    /// Revert all tracked operations in reverse order and return failures.
    pub fn rollback(&mut self) -> Vec<String> {
        let mut failures = Vec::new();

        for path in self.created_roots.iter().rev() {
            if path.exists()
                && let Err(error) = fs::remove_dir_all(path)
            {
                failures.push(format!("{}: {error}", path.display()));
            }
        }

        for (from, to) in self.moved_roots.iter().rev() {
            if to.exists()
                && !from.exists()
                && let Err(error) = fs::rename(to, from)
            {
                failures.push(format!("{} -> {}: {error}", to.display(), from.display()));
            }
        }

        failures
    }
}

/// Applies `result`, returning success values unchanged and rolling back on errors.
pub(crate) fn rollback_on_error<T>(journal: &mut OperationJournal, result: Result<T>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let failures = journal.rollback();
            if failures.is_empty() {
                Err(error)
            } else {
                Err(SkillsManagerError::RollbackFailed {
                    source: Box::new(error),
                    failures: failures.join("; "),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn rollback_failures_are_reported() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("created-file");
        fs::write(&file_path, "not a directory").unwrap();
        let mut journal = OperationJournal::default();
        journal.record_created(file_path.clone());

        let error = rollback_on_error::<()>(&mut journal, Err(SkillsManagerError::NoSkillsFound))
            .unwrap_err();

        match error {
            SkillsManagerError::RollbackFailed { failures, .. } => {
                assert!(failures.contains(file_path.to_string_lossy().as_ref()));
            }
            other => panic!("expected rollback failure, got {other:?}"),
        }
    }
}
