//! Device-local lesson progress. No learning data is sent to the API.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

use super::{Lesson, LESSONS};

const PROGRESS_KEY: &str = "bitsabi:lesson-progress:v1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LessonStatus {
    #[default]
    NotStarted,
    InProgress,
    Completed,
}

impl LessonStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::InProgress => "In progress",
            Self::Completed => "Completed",
        }
    }

    pub fn modifier(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::InProgress => "in-progress",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Copy)]
pub struct ProgressStore {
    statuses: Signal<BTreeMap<String, LessonStatus>>,
}

impl ProgressStore {
    pub fn status(self, slug: &str) -> LessonStatus {
        self.statuses.read().get(slug).copied().unwrap_or_default()
    }

    pub fn completed_count(self) -> usize {
        self.statuses
            .read()
            .values()
            .filter(|status| **status == LessonStatus::Completed)
            .count()
    }

    pub fn next_lesson(self) -> Option<&'static Lesson> {
        LESSONS
            .iter()
            .find(|lesson| self.status(lesson.slug) != LessonStatus::Completed)
    }

    pub fn mark_in_progress(mut self, slug: &str) {
        if self.status(slug) != LessonStatus::NotStarted {
            return;
        }

        let mut statuses = self.statuses.write();
        statuses.insert(slug.to_owned(), LessonStatus::InProgress);
        save_statuses(&statuses);
    }

    pub fn mark_completed(mut self, slug: &str) {
        let mut statuses = self.statuses.write();
        statuses.insert(slug.to_owned(), LessonStatus::Completed);
        save_statuses(&statuses);
    }
}

pub fn provide_progress() -> ProgressStore {
    let statuses = use_signal(load_statuses);

    use_context_provider(move || ProgressStore { statuses })
}

pub fn use_progress() -> ProgressStore {
    use_context()
}

fn load_statuses() -> BTreeMap<String, LessonStatus> {
    LocalStorage::get(PROGRESS_KEY).unwrap_or_default()
}

fn save_statuses(statuses: &BTreeMap<String, LessonStatus>) {
    let _ = LocalStorage::set(PROGRESS_KEY, statuses);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(LessonStatus::NotStarted.label(), "Not started");
        assert_eq!(LessonStatus::InProgress.label(), "In progress");
        assert_eq!(LessonStatus::Completed.label(), "Completed");
    }
}
