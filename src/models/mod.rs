//! Serde data models. Plain structs (no behavior) — composition by value.

pub mod build;
pub mod common;
pub mod config;
pub mod course;
pub mod data;
pub mod examples;
pub mod execute;
pub mod goal;
pub mod installs;
pub mod issue;
pub mod lesson;
pub mod link;
pub mod note;
pub mod plan;
pub mod progress;
pub mod quiz;
pub mod register;
pub mod skip;
pub mod venv;

// Dev-only models (adr/016). Compiled solely under the `dev` feature.
#[cfg(feature = "dev")]
pub mod dev;

pub use self::common::RowError;
pub use self::course::{CourseCounts, CourseListItem, CourseRow, CourseSpec};
pub use self::data::Data;
pub use self::execute::{ExecError, ExecuteCells};
pub use self::goal::{GoalListItem, GoalRow, GoalSpec};
pub use self::issue::{IssueListItem, IssueSpec};
pub use self::lesson::{
    CheckableTree, LessonConflict, LessonCounts, LessonListItem, LessonProgress, LessonRow,
    LessonSpec, SectionTree,
};
pub use self::note::{NoteItem, NoteSpec};
pub use self::plan::{PlanListItem, PlanRow, PlanSpec};
pub use self::progress::{
    GoalRollup, LessonRollup, NoteRollup, NotesByKind, ProgressLesson, QuizRollup,
};
pub use self::quiz::{CaseResult, QuizListItem, QuizRunItem};
pub use self::venv::Package;
