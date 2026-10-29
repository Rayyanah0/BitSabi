//! Bitcoin learning content and device-local progress.

mod lessons;
mod progress;

pub use lessons::{find_lesson, Lesson, LessonIcon, LESSONS};
pub use progress::{provide_progress, use_progress, LessonStatus};
