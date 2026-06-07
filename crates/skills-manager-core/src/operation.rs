use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{InstallCandidate, InstallPreview, InstallRequest, Result, SkillScope};

#[derive(Debug, Clone, Serialize)]
pub struct OperationPlan {
    pub id: String,
    pub request: InstallRequest,
    pub scope: SkillScope,
    pub destination_root: PathBuf,
    pub candidates: Vec<InstallCandidate>,
    pub created_at: DateTime<Utc>,
}

impl OperationPlan {
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

    pub fn preview(&self) -> InstallPreview {
        InstallPreview {
            scope: self.scope,
            destination_root: self.destination_root.clone(),
            candidates: self.candidates.clone(),
            operation_plan: Some(self.clone()),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OperationJournal {
    created_roots: Vec<PathBuf>,
    moved_roots: Vec<(PathBuf, PathBuf)>,
}

impl OperationJournal {
    pub fn record_created(&mut self, path: PathBuf) {
        self.created_roots.push(path);
    }

    pub fn record_move(&mut self, from: PathBuf, to: PathBuf) {
        self.moved_roots.push((from, to));
    }

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

pub(crate) fn rollback_on_error<T>(journal: &mut OperationJournal, result: Result<T>) -> Result<T> {
    if result.is_err() {
        let _ = journal.rollback();
    }

    result
}
