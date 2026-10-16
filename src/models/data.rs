//! The command success-payload enum (one variant per command).
//!
//! Serialized as the envelope `data`. `#[serde(untagged)]` flattens each variant
//! to its fields directly, matching the per-command `data` shapes in `docs/specs/`.

use serde::Serialize;

use crate::models::common::RowError;
use crate::models::course::{CourseCounts, CourseListItem, CourseRow};
use crate::models::execute::{ExecError, ExecuteCells};
use crate::models::goal::GoalListItem;
use crate::models::issue::IssueListItem;
use crate::models::lesson::{
    CheckableTree, LessonConflict, LessonCounts, LessonListItem, LessonProgress, LessonRow,
    SectionTree, VerifyCheckable,
};
use crate::models::note::NoteItem;
use crate::models::plan::{PlanListItem, PlanRow};
use crate::models::progress::{GoalRollup, LessonRollup, NoteRollup, ProgressLesson, QuizRollup};
use crate::models::quiz::{CaseResult, QuizListItem, QuizRunItem};
use crate::models::venv::Package;

/// The success payload of a command (one variant per command).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Data {
    /// `howto` payload: the full generated manual text.
    Howto {
        /// The manual text scraped from the clap surface.
        howto: String,
    },
    // ---- course ----
    /// `course create`.
    CourseCreate {
        /// slug.
        slug: String,
        /// title.
        title: String,
        /// absolute course directory path.
        path: String,
    },
    /// `course list`.
    CourseList {
        /// courses.
        courses: Vec<CourseListItem>,
        /// corrupt-course entries (never silently dropped).
        errors: Vec<RowError>,
    },
    /// `course show`.
    CourseShow {
        /// slug.
        slug: String,
        /// title.
        title: String,
        /// goal.
        goal: String,
        /// description.
        description: String,
        /// per-table counts.
        counts: CourseCounts,
    },
    /// `course update`.
    CourseUpdate {
        /// slug.
        slug: String,
        /// the full new row as stored.
        updated: CourseRow,
    },
    /// `course delete`.
    CourseDelete {
        /// slug.
        slug: String,
        /// always `true`.
        deleted: bool,
    },
    /// `course switch`.
    CourseSwitch {
        /// the new active course slug.
        active_course: String,
    },
    // ---- plan ----
    /// `plan create` (draft).
    PlanCreate {
        /// id.
        id: String,
        /// scope.
        scope: String,
        /// scope_id.
        scope_id: String,
        /// title.
        title: String,
        /// stored body.
        content: String,
        /// always false on a draft.
        confirmed: bool,
    },
    /// `plan show`.
    PlanShow {
        /// id.
        id: String,
        /// scope.
        scope: String,
        /// scope_id.
        scope_id: String,
        /// title.
        title: String,
        /// stored body.
        content: String,
        /// `None` until confirmed.
        confirmed_at: Option<String>,
    },
    /// `plan list`.
    PlanList {
        /// plans.
        plans: Vec<PlanListItem>,
    },
    /// `plan confirm`.
    PlanConfirm {
        /// id.
        id: String,
        /// always true.
        confirmed: bool,
        /// when confirmed.
        confirmed_at: String,
        /// goals materialized (course scope).
        goals_created: Vec<String>,
    },
    /// `plan update`.
    PlanUpdate {
        /// id.
        id: String,
        /// the full new row as stored.
        updated: PlanRow,
    },
    /// `plan delete`.
    PlanDelete {
        /// id.
        id: String,
        /// always true.
        deleted: bool,
    },
    // ---- goal ----
    /// `goal add`.
    GoalAdd {
        /// id.
        id: String,
        /// goal text.
        text: String,
        /// covering lesson ids.
        covered_by: Vec<String>,
        /// always `pending` on add.
        status: String,
    },
    /// `goal list`.
    GoalList {
        /// goals.
        goals: Vec<GoalListItem>,
    },
    /// `goal update`.
    GoalUpdate {
        /// id.
        id: String,
        /// effective status.
        status: String,
        /// whether the status is pinned.
        #[serde(rename = "override")]
        override_field: bool,
        /// covering lesson ids.
        covered_by: Vec<String>,
    },
    /// `goal remove`.
    GoalRemove {
        /// id.
        id: String,
        /// always true.
        deleted: bool,
    },
    // ---- lesson ----
    /// `lesson create`.
    LessonCreate {
        /// id (slug).
        id: String,
        /// slug.
        slug: String,
        /// lesson directory path.
        path: String,
        /// counts.
        counts: LessonCounts,
    },
    /// `lesson get` (full tree).
    LessonGet {
        /// id.
        id: String,
        /// slug.
        slug: String,
        /// title.
        title: String,
        /// order.
        ord: i64,
        /// derived status.
        status: String,
        /// skip flag.
        skip: bool,
        /// sections (with practice).
        sections: Vec<SectionTree>,
        /// end-of-notebook quizzes.
        quizzes: Vec<CheckableTree>,
    },
    /// `lesson list`.
    LessonList {
        /// lessons.
        lessons: Vec<LessonListItem>,
        /// corrupt-row entries.
        errors: Vec<RowError>,
    },
    /// `lesson show`.
    LessonShow {
        /// id.
        id: String,
        /// title.
        title: String,
        /// status.
        status: String,
        /// skip flag.
        skip: bool,
        /// progress counts.
        progress: LessonProgress,
    },
    /// `lesson update`.
    LessonUpdate {
        /// id.
        id: String,
        /// the full new row as stored.
        updated: LessonRow,
    },
    /// `lesson delete`.
    LessonDelete {
        /// id.
        id: String,
        /// always true.
        deleted: bool,
    },
    /// `lesson sync`.
    LessonSync {
        /// id.
        id: String,
        /// always true.
        synced: bool,
        /// practice/quiz stub conflicts.
        conflicts: Vec<LessonConflict>,
    },
    /// `lesson execute` (`--allow-errors`).
    LessonExecute {
        /// id.
        id: String,
        /// always true.
        executed: bool,
        /// cell counts.
        cells: ExecuteCells,
        /// errored cells.
        errors: Vec<ExecError>,
    },
    /// `lesson verify`.
    LessonVerify {
        /// lesson id (`Some` in `<id>` mode; `None` in `--spec` mode).
        lesson_id: Option<String>,
        /// checkables with a reference `solution`.
        checked: i64,
        /// checkables whose solution passes all its cases.
        passing: i64,
        /// checkables whose solution fails ≥1 case.
        failing: i64,
        /// per-checkable results (cases nested under each checkable).
        checkables: Vec<VerifyCheckable>,
    },
    /// `lesson new`.
    LessonNew {
        /// the YAML template (stdout/print mode; `None` with `--out`).
        #[serde(skip_serializing_if = "Option::is_none")]
        yaml: Option<String>,
        /// file written (`--out`; `None` in print mode).
        #[serde(skip_serializing_if = "Option::is_none")]
        written_to: Option<String>,
    },
    // ---- quiz ----
    /// `quiz run`.
    QuizRun {
        /// lesson id.
        lesson_id: String,
        /// per-quiz live results.
        quizzes: Vec<QuizRunItem>,
        /// always true.
        saved: bool,
    },
    /// `quiz list`.
    QuizList {
        /// quizzes.
        quizzes: Vec<QuizListItem>,
    },
    /// `quiz show`.
    QuizShow {
        /// id.
        id: String,
        /// lesson id.
        lesson_id: String,
        /// function name.
        name: String,
        /// signature.
        signature: String,
        /// prompt.
        prompt: String,
        /// case count.
        cases: i64,
        /// skip flag.
        skip: bool,
        /// last-check pass flag.
        pass_or_fail: bool,
    },
    /// `quiz results`.
    QuizResults {
        /// quiz id.
        quiz_id: String,
        /// skip flag.
        skipped: bool,
        /// last-check pass flag.
        pass_or_fail: bool,
        /// cases passed.
        passed: i64,
        /// total cases.
        total: i64,
        /// per-case results.
        cases: Vec<CaseResult>,
    },
    // ---- venv ----
    /// `venv create`.
    VenvCreate {
        /// course slug.
        course: String,
        /// python version (or `default`).
        python: String,
        /// `.venv` path.
        path: String,
        /// base deps installed.
        deps: Vec<String>,
    },
    /// `venv sync`.
    VenvSync {
        /// course slug.
        course: String,
        /// always true.
        synced: bool,
    },
    /// `venv list`.
    VenvList {
        /// course slug.
        course: String,
        /// installed packages.
        packages: Vec<Package>,
    },
    /// `venv add`.
    VenvAdd {
        /// course slug.
        course: String,
        /// packages added.
        added: Vec<String>,
        /// installed packages after add.
        packages: Vec<Package>,
    },
    // ---- skip ----
    /// `skip` (top-level; adr/011).
    Skip {
        /// `lesson` | `quiz` | `practice`.
        scope: String,
        /// the row id.
        id: String,
        /// the new skip flag value.
        skip: bool,
    },
    // ---- progress ----
    /// `progress show`.
    ProgressShow {
        /// live per-lesson state.
        lessons: Vec<ProgressLesson>,
    },
    /// `progress summary`.
    ProgressSummary {
        /// lesson status roll-up.
        lessons: LessonRollup,
        /// non-skipped quiz roll-up.
        quizzes: QuizRollup,
        /// goal roll-up.
        goals: GoalRollup,
        /// note roll-up (incl. `by_kind`).
        notes: NoteRollup,
    },
    // ---- notes ----
    /// `notes add`.
    NotesAdd {
        /// id.
        id: String,
        /// kind.
        kind: String,
        /// tags.
        tags: Vec<String>,
        /// always `open` on add.
        status: String,
        /// authored recurrence (never auto-changed).
        recurrence: String,
        /// free lesson/quiz ref.
        related: String,
        /// the note body.
        text: String,
        /// open notes sharing ≥1 tag (advisory hint).
        related_open: Vec<String>,
    },
    /// `notes show`.
    NotesShow {
        /// the note (single element).
        notes: Vec<NoteItem>,
    },
    /// `notes list`.
    NotesList {
        /// notes.
        notes: Vec<NoteItem>,
        /// corrupt-row entries (never silently dropped).
        errors: Vec<RowError>,
    },
    /// `notes update`.
    NotesUpdate {
        /// id.
        id: String,
        /// the full new row as stored.
        updated: NoteItem,
    },
    /// `notes resolve`.
    NotesResolve {
        /// id.
        id: String,
        /// always `resolved`.
        status: String,
    },
    /// `notes remove`.
    NotesRemove {
        /// id.
        id: String,
        /// always true.
        deleted: bool,
    },
    // ---- bug/feature (issues) ----
    /// `bug file` / `feature file`.
    IssueFile {
        /// id (`b1`/`f1`…).
        id: String,
        /// absolute file path.
        path: String,
        /// always `open` on file.
        status: String,
    },
    /// `bug list` / `feature list`.
    IssueList {
        /// items.
        items: Vec<IssueListItem>,
        /// corrupt-file entries (never silently dropped).
        errors: Vec<RowError>,
    },
    /// `bug show` / `feature show`.
    IssueShow {
        /// id.
        id: String,
        /// title.
        title: String,
        /// description.
        description: String,
        /// repro (bug only).
        repro: Option<String>,
        /// rationale (feature only).
        rationale: Option<String>,
        /// `open` | `resolved`.
        status: String,
        /// set when resolved.
        resolved_ts: Option<String>,
    },
    /// `bug resolve` / `feature resolve`.
    IssueResolve {
        /// id.
        id: String,
        /// always `resolved`.
        status: String,
        /// when resolved.
        resolved_ts: String,
    },
    // ---- config ----
    /// `config get` (all keys).
    ConfigAll {
        /// where `install` places the binary.
        bin_dir: String,
        /// python version (`None` = uv default).
        python: Option<String>,
        /// per-cell execution timeout.
        timeout_secs: u64,
        /// active course slug.
        active_course: Option<String>,
        /// carpenter source checkout (used by `upgrade`).
        source_dir: Option<String>,
    },
    /// `config get <key>`.
    ConfigGet {
        /// the key.
        key: String,
        /// the coerced value.
        value: serde_json::Value,
    },
    /// `config set <key> <value>`.
    ConfigSet {
        /// the key.
        key: String,
        /// the coerced value.
        value: serde_json::Value,
    },
    // ---- register / deregister (skill integration) ----
    /// `register`.
    Register {
        /// app name.
        app: String,
        /// skill file path.
        path: String,
        /// embedded version.
        version: String,
        /// always true.
        installed: bool,
    },
    /// `register --print-skill`.
    PrintSkill {
        /// the rendered SKILL.md bytes.
        skill: String,
    },
    /// `deregister`.
    Deregister {
        /// app name.
        app: String,
        /// skill file path that was removed.
        path: String,
        /// always true.
        removed: bool,
    },
    // ---- build / install / upgrade ----
    /// `build <path>`.
    Build {
        /// the built course directory.
        path: String,
        /// slug (path basename).
        slug: String,
        /// artifacts created.
        created: Vec<String>,
    },
    /// `install`.
    Install {
        /// always true.
        installed: bool,
        /// whether the target binary already existed (re-install over an old copy).
        already_present: bool,
        /// installed binary path.
        bin: String,
        /// whether `bin_dir` is on `$PATH`.
        on_path: bool,
    },
    /// `upgrade`.
    Upgrade {
        /// always true.
        upgraded: bool,
        /// new version.
        version: String,
        /// upgraded binary path.
        bin: String,
        /// upgrade origin: source dir rebuilt or release tarball URL.
        source: String,
        /// skill refresh outcome (`null` with `--no-skill`).
        skill: Option<serde_json::Value>,
    },
    /// `uninstall`.
    Uninstall {
        /// always true.
        uninstalled: bool,
        /// removed binary path (`null` if it was not present).
        bin: Option<String>,
        /// skill removal outcome: `{removed:true,app,path}` or
        /// `{removed:false,reason:"not_registered"}`.
        skill: serde_json::Value,
        /// whether the config file was removed (`--purge-config`).
        config_purged: bool,
    },
    /// `installs list`.
    InstallsList {
        /// one row per discovered install, sorted by tag.
        installs: Vec<crate::models::installs::InstallRow>,
    },
    /// `installs remove <tag>`.
    InstallsRemove {
        /// always true.
        removed: bool,
        /// the install tag that was removed.
        tag: String,
        /// removed binary path (`null` if it was not present).
        bin: Option<String>,
        /// skill removal outcome (same shape as `uninstall`).
        skill: serde_json::Value,
        /// whether the config file was removed (`--purge-config`).
        config_purged: bool,
    },
    // ---- link ----
    /// `link register`.
    LinkRegister {
        /// manifest name.
        name: String,
        /// carpenter version.
        version: String,
        /// binary path.
        bin: String,
        /// one-line summary.
        summary: String,
        /// short howto excerpt.
        howto_excerpt: String,
        /// top-level command names.
        commands: Vec<String>,
    },
    // ---- dev (adr/016; cfg-gated — never in a release binary) ----
    /// (dev) `carpenter dev check` — prerequisite probe.
    #[cfg(feature = "dev")]
    DevCheck {
        /// the prerequisite checks (uv, …).
        checks: Vec<crate::models::dev::DevCheckItem>,
    },
    /// (dev) `carpenter dev setup` — create the validation sandbox.
    #[cfg(feature = "dev")]
    DevSetup {
        /// absolute sandbox path.
        path: String,
        /// whether the directory was newly created.
        created: bool,
    },
    /// (dev) `carpenter dev clean` — remove the validation sandbox.
    #[cfg(feature = "dev")]
    DevClean {
        /// whether a sandbox was removed.
        removed: bool,
        /// the removed path.
        path: String,
    },
    // ---- tui (adr/028; cfg-gated — never in a release binary) ----
    /// (tui) `carpenter tui` — session summary on exit.
    #[cfg(feature = "tui")]
    TuiExit {
        /// screens visited, in navigation order (first is `home`).
        screens: Vec<String>,
        /// command-fn actions applied successfully through the UI.
        actions: u32,
    },
}

impl Data {
    /// The human-readable envelope `message` for this payload.
    pub fn message(&self) -> String {
        match self {
            Self::Howto { .. } => String::from("manual printed"),
            Self::CourseCreate { slug, .. } => format!("course created: {slug}"),
            Self::CourseList { .. } => String::from("courses listed"),
            Self::CourseShow { slug, .. } => format!("course shown: {slug}"),
            Self::CourseUpdate { slug, .. } => format!("course updated: {slug}"),
            Self::CourseDelete { slug, .. } => format!("course deleted: {slug}"),
            Self::CourseSwitch { active_course } => {
                format!("active course switched: {active_course}")
            }
            Self::PlanCreate { id, .. } => format!("plan draft created: {id}"),
            Self::PlanShow { id, .. } => format!("plan shown: {id}"),
            Self::PlanList { .. } => String::from("plans listed"),
            Self::PlanConfirm { id, .. } => format!("plan confirmed: {id}"),
            Self::PlanUpdate { id, .. } => format!("plan updated: {id}"),
            Self::PlanDelete { id, .. } => format!("plan deleted: {id}"),
            Self::GoalAdd { id, .. } => format!("goal added: {id}"),
            Self::GoalList { .. } => String::from("goals listed"),
            Self::GoalUpdate { id, .. } => format!("goal updated: {id}"),
            Self::GoalRemove { id, .. } => format!("goal removed: {id}"),
            Self::LessonCreate { slug, .. } => format!("lesson created: {slug}"),
            Self::LessonGet { id, .. } => format!("lesson shown: {id}"),
            Self::LessonList { .. } => String::from("lessons listed"),
            Self::LessonShow { id, .. } => format!("lesson shown: {id}"),
            Self::LessonUpdate { id, .. } => format!("lesson updated: {id}"),
            Self::LessonDelete { id, .. } => format!("lesson deleted: {id}"),
            Self::LessonSync { id, .. } => format!("lesson synced: {id}"),
            Self::LessonExecute { id, .. } => format!("lesson executed: {id}"),
            Self::LessonVerify { lesson_id, .. } => format!(
                "lesson verified: {}",
                lesson_id.as_deref().unwrap_or("(spec)")
            ),
            Self::LessonNew {
                written_to: Some(p),
                ..
            } => format!("lesson template written: {p}"),
            Self::LessonNew { .. } => String::from("lesson template printed"),
            Self::QuizRun { lesson_id, .. } => format!("quizzes run: {lesson_id}"),
            Self::QuizList { .. } => String::from("quizzes listed"),
            Self::QuizShow { id, .. } => format!("quiz shown: {id}"),
            Self::QuizResults { quiz_id, .. } => format!("quiz results: {quiz_id}"),
            Self::VenvCreate { course, .. } => format!("venv created: {course}"),
            Self::VenvSync { course, .. } => format!("venv synced: {course}"),
            Self::VenvList { course, .. } => format!("venv listed: {course}"),
            Self::VenvAdd { course, .. } => format!("venv added: {course}"),
            Self::Skip { scope, id, skip } => {
                if *skip {
                    format!("skip set: {scope} {id}")
                } else {
                    format!("skip cleared: {scope} {id}")
                }
            }
            Self::ProgressShow { .. } => String::from("progress shown"),
            Self::ProgressSummary { .. } => String::from("progress summarized"),
            Self::NotesAdd { id, .. } => format!("note added: {id}"),
            Self::NotesShow { .. } => String::from("note shown"),
            Self::NotesList { .. } => String::from("notes listed"),
            Self::NotesUpdate { id, .. } => format!("note updated: {id}"),
            Self::NotesResolve { id, .. } => format!("note resolved: {id}"),
            Self::NotesRemove { id, .. } => format!("note removed: {id}"),
            Self::IssueFile { id, .. } => format!("issue filed: {id}"),
            Self::IssueList { .. } => String::from("issues listed"),
            Self::IssueShow { id, .. } => format!("issue shown: {id}"),
            Self::IssueResolve { id, .. } => format!("issue resolved: {id}"),
            Self::ConfigAll { .. } => String::from("config listed"),
            Self::ConfigGet { key, .. } => format!("config shown: {key}"),
            Self::ConfigSet { key, .. } => format!("config set: {key}"),
            Self::Register { app, .. } => format!("skill registered: {app}"),
            Self::PrintSkill { .. } => String::from("skill printed"),
            Self::Deregister { app, .. } => format!("skill deregistered: {app}"),
            Self::Build { slug, .. } => format!("course built: {slug}"),
            Self::Install { bin, .. } => format!("installed: {bin}"),
            Self::Upgrade { version, .. } => format!("upgraded: {version}"),
            Self::Uninstall { bin, .. } => match bin {
                Some(b) => format!("uninstalled: {b}"),
                None => String::from("uninstalled"),
            },
            Self::InstallsList { .. } => String::from("installs listed"),
            Self::InstallsRemove { tag, .. } => format!("install removed: {tag}"),
            Self::LinkRegister { .. } => String::from("link manifest emitted"),
            #[cfg(feature = "dev")]
            Self::DevCheck { .. } => String::from("dev prerequisites checked"),
            #[cfg(feature = "dev")]
            Self::DevSetup { path, .. } => format!("dev sandbox set up: {path}"),
            #[cfg(feature = "dev")]
            Self::DevClean { removed, path } => {
                if *removed {
                    format!("dev sandbox cleaned: {path}")
                } else {
                    format!("dev sandbox already absent: {path}")
                }
            }
            #[cfg(feature = "tui")]
            Self::TuiExit { .. } => String::from("tui session ended"),
        }
    }
}
