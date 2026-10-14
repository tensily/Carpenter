//! Clap wiring + the emit harness. No logic lives here.
//!
//! [`cli`] builds the full command tree — the single scrape target for
//! `xtask gen-howto`. [`run`] parses args, resolves paths + the active course,
//! dispatches to a command fn, and emits exactly one envelope on stdout.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::commands;
use crate::core;
use crate::core::store::Paths;
use crate::models::Data;

/// Build the full clap command tree (the self-documentation scrape target).
pub fn cli() -> Command {
    let cmd = Command::new("carpenter")
        .version(VERSION)
        .about("Agent-driven CLI that builds Python/Jupyter learning material.")
        .arg(
            Arg::new("root")
                .long("root")
                .value_name("PATH")
                .global(true)
                .help("Workspace root (default: current directory)."),
        )
        .arg(
            Arg::new("course")
                .long("course")
                .short('c')
                .value_name("SLUG")
                .global(true)
                .help("Active course slug (default: active_course in config)."),
        )
        .subcommand(Command::new("howto").about("Print the generated command manual."))
        .subcommand(course_group())
        .subcommand(plan_group())
        .subcommand(goal_group())
        .subcommand(lesson_group())
        .subcommand(quiz_group())
        .subcommand(venv_group())
        .subcommand(skip_cmd_group())
        .subcommand(progress_group())
        .subcommand(notes_group())
        .subcommand(bug_group())
        .subcommand(feature_group())
        .subcommand(config_group())
        .subcommand(
            Command::new("build")
                .about("Scaffold a course at a path (course.json + course.db + lessons/).")
                .arg(positional("path", "Target directory.")),
        )
        .subcommand(self_group());
    // `--capture-example <PATH>` exists only in a `--features dev` build
    // (adr/016). gen-howto runs against a non-dev build of carpenter, so this
    // arg — and thus the whole dev authoring surface — is invisible to the
    // committed howto and the inlined SKILL.md.
    #[cfg(feature = "dev")]
    let cmd = cmd.arg(
        Arg::new(crate::core::dev::CAPTURE_FLAG)
            .long("capture-example")
            .value_name("PATH")
            .global(true)
            .help(
                "DEV ONLY (adr/016): run normally and also write the \
                 worked-example atom for this invocation to PATH.",
            ),
    );
    #[cfg(feature = "dev")]
    let cmd = cmd.subcommand(dev_group());
    cmd
}

/// The `--version` string. Suffixes `(dev build)` under the `dev` feature so an
/// agent can detect it is driving a relaxed-gate binary. `&'static str` because
/// clap's `.version()` requires it (no runtime formatting).
#[cfg(not(feature = "dev"))]
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "dev")]
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (dev build)");

fn course_group() -> Command {
    Command::new("course")
        .about("Course commands.")
        .subcommand(
            Command::new("create")
                .about("Create a course from a spec.")
                .arg(spec_arg()),
        )
        .subcommand(Command::new("list").about("List courses."))
        .subcommand(
            Command::new("show")
                .about("Show a course.")
                .arg(positional("slug", "Course slug.")),
        )
        .subcommand(
            Command::new("update")
                .about("Update a course from a spec (requires --force).")
                .arg(positional("slug", "Course slug."))
                .arg(spec_arg())
                .arg(force_arg()),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a course (requires --force).")
                .arg(positional("slug", "Course slug."))
                .arg(force_arg()),
        )
        .subcommand(
            Command::new("switch")
                .about("Switch the active course.")
                .arg(positional("slug", "Course slug.")),
        )
}

fn plan_group() -> Command {
    Command::new("plan")
        .about("Plan commands (human-in-the-loop).")
        .subcommand(
            Command::new("create")
                .about("Create a plan draft from a spec.")
                .arg(
                    Arg::new("scope")
                        .long("scope")
                        .value_parser(["course", "lesson"])
                        .default_value("course")
                        .help("Plan scope."),
                )
                .arg(
                    Arg::new("lesson")
                        .long("lesson")
                        .value_name("ID")
                        .help("Lesson id (required for --scope lesson)."),
                )
                .arg(spec_arg()),
        )
        .subcommand(
            Command::new("show")
                .about("Show a plan.")
                .arg(positional("id", "Plan id.")),
        )
        .subcommand(
            Command::new("list").about("List plans.").arg(
                Arg::new("scope")
                    .long("scope")
                    .value_parser(["course", "lesson"])
                    .help("Filter by scope."),
            ),
        )
        .subcommand(
            Command::new("confirm")
                .about("Confirm a plan.")
                .arg(positional("id", "Plan id.")),
        )
        .subcommand(
            Command::new("update")
                .about("Update a plan from a spec.")
                .arg(positional("id", "Plan id."))
                .arg(spec_arg()),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a plan.")
                .arg(positional("id", "Plan id."))
                .arg(force_arg()),
        )
}

fn goal_group() -> Command {
    Command::new("goal")
        .about("Goal commands.")
        .subcommand(
            Command::new("add")
                .about("Add a goal from a spec.")
                .arg(spec_arg()),
        )
        .subcommand(Command::new("list").about("List goals."))
        .subcommand(
            Command::new("update")
                .about("Update a goal.")
                .arg(positional("id", "Goal id."))
                .arg(
                    Arg::new("status")
                        .long("status")
                        .value_parser(["pending", "achieved", "skipped", "derived"])
                        .help("Pin a status, or `derived` to resume derivation."),
                )
                .arg(
                    Arg::new("covered-by")
                        .long("covered-by")
                        .value_name("IDS")
                        .help("Comma-separated covering lesson ids."),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a goal (requires --force).")
                .arg(positional("id", "Goal id."))
                .arg(force_arg()),
        )
}

fn spec_arg() -> Arg {
    Arg::new("spec")
        .long("spec")
        .value_name("FILE|-")
        .required(true)
        .help("Spec input: a file path or - for stdin (YAML).")
}

fn lesson_group() -> Command {
    Command::new("lesson")
        .about("Lesson commands.")
        .subcommand(
            Command::new("create")
                .about("Create a lesson from a spec (renders notebook + helper).")
                .arg(spec_arg()),
        )
        .subcommand(
            Command::new("get")
                .about("Show the full lesson tree.")
                .arg(positional("id", "Lesson id.")),
        )
        .subcommand(Command::new("list").about("List lessons."))
        .subcommand(
            Command::new("show")
                .about("Show a lesson's status + progress.")
                .arg(positional("id", "Lesson id.")),
        )
        .subcommand(
            Command::new("update")
                .about("Update a lesson from a spec (requires --force).")
                .arg(positional("id", "Lesson id."))
                .arg(spec_arg())
                .arg(force_arg()),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a lesson (requires --force).")
                .arg(positional("id", "Lesson id."))
                .arg(force_arg()),
        )
        .subcommand(
            Command::new("sync")
                .about("Sync the notebook against the DB (3-way stub preservation).")
                .arg(positional("id", "Lesson id."))
                .arg(force_arg()),
        )
        .subcommand(
            Command::new("execute")
                .about("Execute the lesson notebook in the course venv.")
                .arg(positional("id", "Lesson id."))
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .value_name("SECS")
                        .default_value("30")
                        .help("Per-cell execution timeout."),
                )
                .arg(
                    Arg::new("allow-errors")
                        .long("allow-errors")
                        .action(ArgAction::SetTrue)
                        .help(
                            "Run every cell; return all errors instead of aborting on the first.",
                        ),
                ),
        )
        .subcommand(
            Command::new("verify")
                .about("Verify author solutions against their own cases (answer-key lock).")
                .arg(
                    Arg::new("id")
                        .required(false)
                        .help("Lesson id (--spec mode omits it)."),
                )
                .arg(
                    Arg::new("spec")
                        .long("spec")
                        .value_name("FILE|-")
                        .required(false)
                        .help("Pre-create verify: spec input (file or - for stdin, YAML)."),
                )
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .value_name("SECS")
                        .default_value("30")
                        .help("Per-case timeout."),
                ),
        )
        .subcommand(
            Command::new("new")
                .about("Emit a YAML lesson-spec template (stdout, or --out <FILE>).")
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("FILE")
                        .required(false)
                        .help("Write to a file instead of stdout."),
                ),
        )
}

fn quiz_group() -> Command {
    Command::new("quiz")
        .about("Quiz commands.")
        .subcommand(
            Command::new("run")
                .about("Run a lesson's quizzes (nbconvert in the course venv).")
                .arg(positional("lesson", "Lesson id."))
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .value_name("SECS")
                        .default_value("30")
                        .help("Per-quiz execution timeout."),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List quizzes.")
                .arg(positional("lesson", "Optional lesson id filter.").required(false)),
        )
        .subcommand(
            Command::new("show")
                .about("Show a quiz.")
                .arg(positional("id", "Quiz id.")),
        )
        .subcommand(
            Command::new("results")
                .about("Show a quiz's last-check results.")
                .arg(positional("id", "Quiz id.")),
        )
}

fn venv_group() -> Command {
    Command::new("venv")
        .about("Course venv commands (uv).")
        .subcommand(
            Command::new("create").about("Create the course venv.").arg(
                Arg::new("python")
                    .long("python")
                    .value_name("X.Y")
                    .help("Python version."),
            ),
        )
        .subcommand(Command::new("sync").about("uv sync."))
        .subcommand(Command::new("list").about("List installed packages."))
        .subcommand(
            Command::new("add").about("Add packages.").arg(
                Arg::new("pkg")
                    .num_args(1..)
                    .required(true)
                    .help("Package(s) to add."),
            ),
        )
}

fn skip_cmd_group() -> Command {
    Command::new("skip")
        .about("Set (or clear) the skip flag on a lesson, quiz, or practice item.")
        .arg(positional("id", "Lesson, quiz, or practice id."))
        .arg(
            Arg::new("scope")
                .long("scope")
                .value_parser(["lesson", "quiz", "practice"])
                .required(true)
                .help("Scope of the id."),
        )
        .arg(
            Arg::new("off")
                .long("off")
                .action(ArgAction::SetTrue)
                .help("Clear the skip flag instead of setting it."),
        )
}

fn force_arg() -> Arg {
    Arg::new("force")
        .long("force")
        .action(ArgAction::SetTrue)
        .help("Confirm the destructive operation.")
}

fn yes_arg() -> Arg {
    Arg::new("yes")
        .long("yes")
        .action(ArgAction::SetTrue)
        .help("Skip the confirmation prompt (non-interactive removal).")
}

fn progress_group() -> Command {
    Command::new("progress")
        .about("Progress commands.")
        .subcommand(Command::new("show").about("Per-lesson live progress."))
        .subcommand(Command::new("summary").about("Course roll-up (lessons/quizzes/goals/notes)."))
}

fn notes_group() -> Command {
    Command::new("notes")
        .about("Note commands.")
        .subcommand(
            Command::new("add")
                .about("Add a note from a spec.")
                .arg(spec_arg()),
        )
        .subcommand(
            Command::new("show")
                .about("Show a note.")
                .arg(positional("id", "Note id.")),
        )
        .subcommand(Command::new("list").about("List notes."))
        .subcommand(
            Command::new("update")
                .about("Update a note from a spec.")
                .arg(positional("id", "Note id."))
                .arg(spec_arg()),
        )
        .subcommand(
            Command::new("resolve")
                .about("Resolve a note.")
                .arg(positional("id", "Note id.")),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a note (requires --force).")
                .arg(positional("id", "Note id."))
                .arg(force_arg()),
        )
}

fn bug_group() -> Command {
    Command::new("bug")
        .about("Bug commands (file-backed under ~/.config/carpenter/bug/).")
        .subcommand(
            Command::new("file")
                .about("File a bug from a spec.")
                .arg(spec_arg()),
        )
        .subcommand(Command::new("list").about("List bugs."))
        .subcommand(
            Command::new("show")
                .about("Show a bug.")
                .arg(positional("id", "Bug id.")),
        )
        .subcommand(
            Command::new("resolve")
                .about("Resolve a bug.")
                .arg(positional("id", "Bug id.")),
        )
}

fn feature_group() -> Command {
    Command::new("feature")
        .about("Feature request commands (file-backed under ~/.config/carpenter/feature_request/).")
        .subcommand(
            Command::new("file")
                .about("File a feature request from a spec.")
                .arg(spec_arg()),
        )
        .subcommand(Command::new("list").about("List feature requests."))
        .subcommand(
            Command::new("show")
                .about("Show a feature request.")
                .arg(positional("id", "Feature id.")),
        )
        .subcommand(
            Command::new("resolve")
                .about("Resolve a feature request.")
                .arg(positional("id", "Feature id.")),
        )
}

fn config_group() -> Command {
    Command::new("config")
        .about("App config commands (~/.config/carpenter/config.json).")
        .subcommand(
            Command::new("get")
                .about("Show config values (all, or one key).")
                .arg(
                    Arg::new("key")
                        .required(false)
                        .help("Config key (all if omitted)."),
                ),
        )
        .subcommand(
            Command::new("set")
                .about("Set a config value (coerced to the key's type).")
                .arg(positional("key", "Config key."))
                .arg(positional("value", "Config value.")),
        )
}

fn self_group() -> Command {
    Command::new("self")
        .about("Install, update, register, and remove carpenter installs (the tool itself).")
        .subcommand(
            Command::new("install")
                .about("Place the carpenter binary into a bin dir.")
                .arg(
                    Arg::new("bin-dir")
                        .long("bin-dir")
                        .value_name("PATH")
                        .help("Bin dir (default: config bin_dir -> ~/.local/bin)."),
                ),
        )
        .subcommand(
            Command::new("upgrade")
                .about("Replace the installed binary from a published release (or a source checkout) and refresh the skill.")
                .arg(
                    Arg::new("channel")
                        .long("channel")
                        .value_name("CHANNEL")
                        .default_value("stable")
                        .help("Release channel to fetch (stable|nightly)."),
                )
                .arg(
                    Arg::new("source")
                        .long("source")
                        .value_name("PATH")
                        .help("Source checkout (default: config source_dir)."),
                )
                .arg(
                    Arg::new("bin-dir")
                        .long("bin-dir")
                        .value_name("PATH")
                        .help("Install target (default: config bin_dir)."),
                )
                .arg(
                    Arg::new("no-skill")
                        .long("no-skill")
                        .action(ArgAction::SetTrue)
                        .help("Skip the skill auto-refresh (skill:null)."),
                ),
        )
        .subcommand(
            Command::new("uninstall")
                .about("Remove the carpenter skill, the installed binary, and (with --purge-config) the config. Prints a removal plan and asks for confirmation; --yes skips.")
                .arg(
                    Arg::new("bin-dir")
                        .long("bin-dir")
                        .value_name("PATH")
                        .help("Bin dir (default: config bin_dir -> ~/.local/bin)."),
                )
                .arg(
                    Arg::new("purge-config")
                        .long("purge-config")
                        .action(ArgAction::SetTrue)
                        .help("Also remove the carpenter config file."),
                )
                .arg(yes_arg()),
        )
        .subcommand(
            Command::new("register")
                .about("Register the carpenter skill with an agent app (writes SKILL.md; opencode also gets a permission entry).")
                .arg(
                    Arg::new("app")
                        .long("app")
                        .default_value("opencode")
                        .help("Agent app (opencode|claude-code)."),
                )
                .arg(
                    Arg::new("print-skill")
                        .long("print-skill")
                        .action(ArgAction::SetTrue)
                        .help("Print the rendered SKILL.md to stdout instead of writing."),
                ),
        )
        .subcommand(
            Command::new("deregister")
                .about("Deregister the carpenter skill (removes SKILL.md; opencode also loses its permission entry). Prints a plan and asks for confirmation; --yes skips.")
                .arg(
                    Arg::new("app")
                        .long("app")
                        .default_value("opencode")
                        .help("Agent app (opencode|claude-code)."),
                )
                .arg(yes_arg()),
        )
        .subcommand(
            Command::new("list")
                .about("List carpenter installs on this machine (binaries, skills, data dirs).")
                .arg(
                    Arg::new("bin-dir")
                        .long("bin-dir")
                        .value_name("PATH")
                        .help("Bin dir (default: config bin_dir -> ~/.local/bin)."),
                ),
        )
        .subcommand(
            Command::new("remove")
                .about("Uninstall any carpenter install by tag (skill + binary + config purge). Prints a removal plan and asks for confirmation; --yes skips.")
                .arg(
                    Arg::new("tag")
                        .value_name("TAG")
                        .required(true)
                        .help("Install tag: the binary stem (carpenter, carpenter-<branch>)."),
                )
                .arg(
                    Arg::new("bin-dir")
                        .long("bin-dir")
                        .value_name("PATH")
                        .help("Bin dir (default: config bin_dir -> ~/.local/bin)."),
                )
                .arg(
                    Arg::new("purge-config")
                        .long("purge-config")
                        .action(ArgAction::SetTrue)
                        .help("Also remove the target install's config file."),
                )
                .arg(yes_arg()),
        )
        .subcommand(
            Command::new("link")
                .about("Emit the future CLI registry manifest."),
        )
}

/// (dev only, adr/016) the `dev` group: validation-sandbox lifecycle. Compiled
/// only under the `dev` feature, so it never appears in the release clap surface
/// the howto is scraped from.
#[cfg(feature = "dev")]
fn dev_group() -> Command {
    Command::new("dev")
        .about(
            "DEV ONLY (adr/016): validation lifecycle — check/setup/clean the sandbox \
             and register/upgrade the local skill.",
        )
        .subcommand(Command::new("check").about("DEV ONLY: probe prerequisites (uv, …)."))
        .subcommand(
            Command::new("setup").about("DEV ONLY: create the .sandbox validation workspace."),
        )
        .subcommand(
            Command::new("clean").about("DEV ONLY: remove the .sandbox validation workspace."),
        )
        .subcommand(Command::new("register").about(
            "DEV ONLY: write the local .opencode carpenter skill (dev analog of `register`).",
        ))
        .subcommand(Command::new("upgrade").about(
            "DEV ONLY: rebuild the dev binary + refresh the local skill (dev analog of `upgrade`).",
        ))
}

fn positional(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name).required(true).help(help)
}

/// Run the CLI: parse args, dispatch, emit exactly one envelope on stdout.
/// Never panics. Under the `dev` feature, `--capture-example <PATH>` runs the
/// command normally and additionally writes its worked-example atom (adr/016).
pub fn run() -> ExitCode {
    let matches = cli().get_matches();
    if matches.subcommand().is_none() {
        let _ = cli().print_help();
        println!();
        return ExitCode::FAILURE;
    }
    let paths = paths_from(&matches);
    let result = dispatch(&paths, &matches);
    let (stdout, is_error) = core::output::render(result);
    #[cfg(feature = "dev")]
    if let Some(path) = matches
        .get_one::<String>(crate::core::dev::CAPTURE_FLAG)
        .map(|s| s.as_str())
    {
        crate::core::dev::write_capture_example(&matches, path, &stdout);
    }
    println!("{stdout}");
    if is_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Resolve the subcommand to a `Result<Data, _>`. The single envelope is
/// rendered by [`run`]; splitting dispatch out lets the `--capture-example` hook
/// observe the result once (instead of re-rendering) and keeps the emit path in
/// one place.
fn dispatch(paths: &Paths, matches: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match matches.subcommand() {
        Some(("howto", _)) => commands::howto::howto(),
        Some(("course", sub)) => course_cmd(paths, sub),
        Some(("plan", sub)) => match active_course(paths, matches) {
            Ok(course) => plan_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("goal", sub)) => match active_course(paths, matches) {
            Ok(course) => goal_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("lesson", sub)) => {
            // `lesson new` is course-agnostic (pure template emitter); skip
            // course resolution so it works without `-c`/active_course.
            if sub.subcommand_name() == Some("new") {
                lesson_cmd(paths, "", sub)
            } else {
                match active_course(paths, matches) {
                    Ok(course) => lesson_cmd(paths, &course, sub),
                    Err(e) => Err(e),
                }
            }
        }
        Some(("quiz", sub)) => match active_course(paths, matches) {
            Ok(course) => quiz_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("venv", sub)) => match active_course(paths, matches) {
            Ok(course) => venv_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("skip", m)) => match active_course(paths, matches) {
            Ok(course) => skip_cmd(paths, &course, m),
            Err(e) => Err(e),
        },
        Some(("progress", sub)) => match active_course(paths, matches) {
            Ok(course) => progress_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("notes", sub)) => match active_course(paths, matches) {
            Ok(course) => notes_cmd(paths, &course, sub),
            Err(e) => Err(e),
        },
        Some(("bug", sub)) => bug_cmd(paths, sub),
        Some(("feature", sub)) => feature_cmd(paths, sub),
        Some(("config", sub)) => config_cmd(paths, sub),
        Some(("build", m)) => commands::build::build(paths, &arg_string(m, "path")),
        Some(("self", sub)) => self_cmd(paths, sub),
        #[cfg(feature = "dev")]
        Some(("dev", sub)) => dev_cmd(sub),
        _ => Err(core::error::CarpenterError::ValidationError(
            "no subcommand".into(),
        )),
    }
}

fn paths_from(matches: &ArgMatches) -> Paths {
    let root = matches
        .get_one::<String>("root")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    Paths {
        root,
        config_dir: core::store::config_dir(),
        home: core::store::home_dir(),
    }
}

/// Resolve the active course slug from `--course` or `config.active_course`.
fn active_course(
    paths: &Paths,
    matches: &ArgMatches,
) -> Result<String, core::error::CarpenterError> {
    if let Some(c) = matches.get_one::<String>("course") {
        return Ok(c.clone());
    }
    if let Some(p) = paths.config_file() {
        if let Some(a) = core::config::load_from(&p).active_course {
            return Ok(a);
        }
    }
    Err(core::error::CarpenterError::ValidationError(
        "no active course (use `course switch` or pass --course)".into(),
    ))
}

fn course_cmd(paths: &Paths, sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("create", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::course::create(paths, &spec)
        }
        Some(("list", _)) => commands::course::list(paths),
        Some(("show", m)) => commands::course::show(paths, &arg_string(m, "slug")),
        Some(("update", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::course::update(paths, &arg_string(m, "slug"), &spec, m.get_flag("force"))
        }
        Some(("delete", m)) => {
            commands::course::delete(paths, &arg_string(m, "slug"), m.get_flag("force"))
        }
        Some(("switch", m)) => commands::course::switch(paths, &arg_string(m, "slug")),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown course subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn plan_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("create", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            let scope = m
                .get_one::<String>("scope")
                .map(|s| s.as_str())
                .unwrap_or("course");
            let lesson = m.get_one::<String>("lesson").map(|s| s.as_str());
            commands::plan::create(paths, course, scope, lesson, &spec)
        }
        Some(("show", m)) => commands::plan::show(paths, course, &arg_string(m, "id")),
        Some(("list", m)) => {
            let scope = m.get_one::<String>("scope").map(|s| s.as_str());
            commands::plan::list(paths, course, scope)
        }
        Some(("confirm", m)) => commands::plan::confirm(paths, course, &arg_string(m, "id")),
        Some(("update", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::plan::update(paths, course, &arg_string(m, "id"), &spec)
        }
        Some(("delete", m)) => {
            commands::plan::delete(paths, course, &arg_string(m, "id"), m.get_flag("force"))
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown plan subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn goal_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("add", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::goal::add(paths, course, &spec)
        }
        Some(("list", _)) => commands::goal::list(paths, course),
        Some(("update", m)) => {
            let status = m.get_one::<String>("status").map(|s| s.as_str());
            let covered: Option<Vec<String>> = m.get_one::<String>("covered-by").map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            });
            commands::goal::update(
                paths,
                course,
                &arg_string(m, "id"),
                status,
                covered.as_deref(),
            )
        }
        Some(("remove", m)) => {
            commands::goal::remove(paths, course, &arg_string(m, "id"), m.get_flag("force"))
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown goal subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn arg_string(m: &ArgMatches, name: &str) -> String {
    m.get_one::<String>(name).cloned().unwrap_or_default()
}

fn lesson_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("create", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::lesson::create(paths, course, &spec)
        }
        Some(("new", m)) => commands::lesson::new(m.get_one::<String>("out").map(|s| s.as_str())),
        Some(("get", m)) => commands::lesson::get(paths, course, &arg_string(m, "id")),
        Some(("list", _)) => commands::lesson::list(paths, course),
        Some(("show", m)) => commands::lesson::show(paths, course, &arg_string(m, "id")),
        Some(("update", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::lesson::update(
                paths,
                course,
                &arg_string(m, "id"),
                &spec,
                m.get_flag("force"),
            )
        }
        Some(("delete", m)) => {
            commands::lesson::delete(paths, course, &arg_string(m, "id"), m.get_flag("force"))
        }
        Some(("sync", m)) => {
            commands::lesson::sync(paths, course, &arg_string(m, "id"), m.get_flag("force"))
        }
        Some(("execute", m)) => {
            let timeout = m
                .get_one::<String>("timeout")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);
            commands::lesson::execute(
                paths,
                course,
                &arg_string(m, "id"),
                timeout,
                m.get_flag("allow-errors"),
            )
        }
        Some(("verify", m)) => {
            let id = arg_string(m, "id");
            let spec = m
                .get_one::<String>("spec")
                .map(|s| s.as_str())
                .map(core::store::read_spec)
                .transpose()?;
            let timeout = m
                .get_one::<String>("timeout")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);
            commands::lesson::verify(
                paths,
                course,
                if id.is_empty() { None } else { Some(&id) },
                spec.as_deref(),
                timeout,
            )
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown lesson subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn quiz_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("run", m)) => {
            let timeout = m
                .get_one::<String>("timeout")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);
            commands::quiz::run(paths, course, &arg_string(m, "lesson"), timeout)
        }
        Some(("list", m)) => {
            let lesson = m.get_one::<String>("lesson").map(|s| s.as_str());
            commands::quiz::list(paths, course, lesson)
        }
        Some(("show", m)) => commands::quiz::show(paths, course, &arg_string(m, "id")),
        Some(("results", m)) => commands::quiz::results(paths, course, &arg_string(m, "id")),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown quiz subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn venv_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("create", m)) => {
            let py = m.get_one::<String>("python").map(|s| s.as_str());
            commands::venv::create(paths, course, py)
        }
        Some(("sync", _)) => commands::venv::sync(paths, course),
        Some(("list", _)) => commands::venv::list(paths, course),
        Some(("add", m)) => {
            let pkgs: Vec<String> = m
                .get_many::<String>("pkg")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            commands::venv::add(paths, course, &pkgs)
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown venv subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn skip_cmd(
    paths: &Paths,
    course: &str,
    m: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    let scope = m
        .get_one::<String>("scope")
        .map(|s| s.as_str())
        .unwrap_or_default();
    commands::skip::skip(
        paths,
        course,
        scope,
        &arg_string(m, "id"),
        m.get_flag("off"),
    )
}

fn progress_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("show", _)) => commands::progress::show(paths, course),
        Some(("summary", _)) => commands::progress::summary(paths, course),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown progress subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn notes_cmd(
    paths: &Paths,
    course: &str,
    sub: &ArgMatches,
) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("add", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::notes::add(paths, course, &spec)
        }
        Some(("show", m)) => commands::notes::show(paths, course, &arg_string(m, "id")),
        Some(("list", _)) => commands::notes::list(paths, course),
        Some(("update", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::notes::update(paths, course, &arg_string(m, "id"), &spec)
        }
        Some(("resolve", m)) => commands::notes::resolve(paths, course, &arg_string(m, "id")),
        Some(("remove", m)) => {
            commands::notes::remove(paths, course, &arg_string(m, "id"), m.get_flag("force"))
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown notes subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn bug_cmd(paths: &Paths, sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("file", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::bug::file(paths, &spec)
        }
        Some(("list", _)) => commands::bug::list(paths),
        Some(("show", m)) => commands::bug::show(paths, &arg_string(m, "id")),
        Some(("resolve", m)) => commands::bug::resolve(paths, &arg_string(m, "id")),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown bug subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn feature_cmd(paths: &Paths, sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("file", m)) => {
            let spec = core::store::read_spec(&arg_string(m, "spec"))?;
            commands::feature::file(paths, &spec)
        }
        Some(("list", _)) => commands::feature::list(paths),
        Some(("show", m)) => commands::feature::show(paths, &arg_string(m, "id")),
        Some(("resolve", m)) => commands::feature::resolve(paths, &arg_string(m, "id")),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown feature subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn config_cmd(paths: &Paths, sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("get", m)) => {
            let key = m.get_one::<String>("key").map(|s| s.as_str());
            commands::config::get(paths, key)
        }
        Some(("set", m)) => {
            commands::config::set(paths, &arg_string(m, "key"), &arg_string(m, "value"))
        }
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown config subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

fn self_cmd(paths: &Paths, sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    fn bin_dir(m: &ArgMatches) -> Option<&str> {
        m.get_one::<String>("bin-dir").map(|s| s.as_str())
    }
    match sub.subcommand() {
        Some(("install", m)) => commands::install::install(paths, bin_dir(m)),
        Some(("upgrade", m)) => commands::upgrade::upgrade(
            paths,
            m.get_one::<String>("source").map(|s| s.as_str()),
            bin_dir(m),
            m.get_flag("no-skill"),
            m.get_one::<String>("channel")
                .map(|s| s.as_str())
                .unwrap_or("stable"),
            &core::store::install_name(),
        ),
        Some(("uninstall", m)) => commands::uninstall::uninstall(
            paths,
            bin_dir(m),
            m.get_flag("purge-config"),
            m.get_flag("yes"),
            &core::store::install_name(),
        ),
        Some(("register", m)) => commands::register::register(
            paths,
            &arg_string(m, "app"),
            m.get_flag("print-skill"),
            &core::store::install_name(),
        ),
        Some(("deregister", m)) => commands::deregister::deregister(
            paths,
            &arg_string(m, "app"),
            m.get_flag("yes"),
            &core::store::install_name(),
        ),
        Some(("list", m)) => commands::installs::list(paths, bin_dir(m)),
        Some(("remove", m)) => commands::installs::remove(
            paths,
            &arg_string(m, "tag"),
            bin_dir(m),
            m.get_flag("purge-config"),
            m.get_flag("yes"),
        ),
        Some(("link", _)) => commands::link::register(paths),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown self subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

/// (dev only, adr/016) dispatch for the `dev` group. cfg-gated alongside the
/// group itself.
#[cfg(feature = "dev")]
fn dev_cmd(sub: &ArgMatches) -> Result<Data, core::error::CarpenterError> {
    match sub.subcommand() {
        Some(("check", _)) => crate::core::dev::check(),
        Some(("setup", _)) => crate::core::dev::setup(),
        Some(("clean", _)) => crate::core::dev::clean(),
        Some(("register", _)) => crate::core::dev::register_local(),
        Some(("upgrade", _)) => crate::core::dev::upgrade_local(),
        _ => Err(core::error::CarpenterError::ValidationError(format!(
            "unknown dev subcommand: {:?}",
            sub.subcommand_name().unwrap_or("(none)")
        ))),
    }
}

#[cfg(test)]
#[test]
fn cli_has_all_top_level_subcommands() {
    let tree = cli();
    let names: Vec<&str> = tree.get_subcommands().map(|c| c.get_name()).collect();
    for expected in [
        "howto", "course", "plan", "goal", "lesson", "quiz", "venv", "skip", "progress", "notes",
        "bug", "feature", "config", "build", "self",
    ] {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }
}

/// (dev only) the `dev` group + its three leaves must exist in a dev build
/// (absent in release — pinned by the inverse of this in the howto guardrail).
#[cfg(all(test, feature = "dev"))]
#[test]
fn dev_group_present_in_dev_build() {
    let tree = cli();
    let dev = tree
        .get_subcommands()
        .find(|c| c.get_name() == "dev")
        .expect("dev group missing in dev build");
    let leaves: Vec<&str> = dev.get_subcommands().map(|c| c.get_name()).collect();
    for expected in ["check", "setup", "clean", "register", "upgrade"] {
        assert!(
            leaves.contains(&expected),
            "missing dev {expected}: {leaves:?}"
        );
    }
}
