//! Filesystem root resolution, the app config dir, slug derivation, atomic
//! writes, and `--spec` input reading.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::core::error::CarpenterError;

/// Resolved runtime paths: the workspace `root` and the app `config_dir`.
#[derive(Debug, Clone)]
pub struct Paths {
    /// Workspace root (from `--root`, else cwd).
    pub root: PathBuf,
    /// App config directory (`~/.config/carpenter`).
    pub config_dir: Option<PathBuf>,
}

impl Paths {
    /// The courses directory (`<root>/courses`).
    pub fn courses(&self) -> PathBuf {
        self.root.join("courses")
    }

    /// The config file path (`<config_dir>/config.json`), if a config dir exists.
    pub fn config_file(&self) -> Option<PathBuf> {
        self.config_dir.as_ref().map(|d| d.join("config.json"))
    }

    /// The config dir, or `StoreError` if none could be resolved (no meta-command
    /// state can be read/written).
    pub fn require_config_dir(&self) -> Result<&Path, CarpenterError> {
        self.config_dir
            .as_deref()
            .ok_or_else(|| CarpenterError::StoreError("no config directory resolved".into()))
    }

    /// The XDG root that contains both `carpenter/` and agent-app dirs (e.g.
    /// `opencode/`). carpenter's [`config_dir`](Self::config_dir) is always a
    /// `carpenter` leaf under it, so the root is its parent. Agent-app skill
    /// integration resolves here (opencode is a sibling of `carpenter/`).
    pub fn xdg_root(&self) -> Result<&Path, CarpenterError> {
        self.require_config_dir()?
            .parent()
            .ok_or_else(|| CarpenterError::StoreError("config dir has no parent".into()))
    }

    /// A course directory.
    pub fn course(&self, slug: &str) -> PathBuf {
        self.courses().join(slug)
    }
}

/// Resolve the workspace root: an explicit `--root`, else the current dir.
pub fn resolve_root(root: Option<&Path>) -> PathBuf {
    match root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().unwrap_or_default(),
    }
}

/// The carpenter app config directory (`~/.config/carpenter` on Linux), named
/// for the install ([`install_name`]) — a branch-tagged binary gets its own
/// dir, so two installs never share state (adr/027).
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(install_name()))
}

/// The install identity: a kebab-case tag every surface of an install derives
/// from — config dir, skill name/dir, opencode permission key. Resolution
/// order: `CARPENTER_INSTALL_NAME` (tests pin it; a user can too), else the
/// running binary's file stem (a branch install `carpenter_ivan_topic` tags
/// itself automatically). The stock `carpenter` stem maps to itself, so plain
/// installs resolve exactly the pre-adr/027 paths (adr/027).
pub fn install_name() -> String {
    let override_name = std::env::var("CARPENTER_INSTALL_NAME")
        .ok()
        .filter(|s| !s.is_empty());
    let stem = override_name.or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
    });
    install_name_from(stem.as_deref())
}

/// Pure core of [`install_name`]: sanitize a binary stem into the install tag.
/// The ` (deleted)` suffix (kernel artifact of upgrade's in-place replace —
/// see `skill::strip_deleted`) is stripped; [`slugify`] does the kebab
/// collapse (`_` → `-`, so the underscored command names map onto the kebab
/// identity that agent-app skill names require); a missing or unsanitizable
/// stem falls back to `carpenter`.
pub fn install_name_from(stem: Option<&str>) -> String {
    const DEFAULT: &str = "carpenter";
    let stem = stem.unwrap_or(DEFAULT);
    let stem = stem.strip_suffix(" (deleted)").unwrap_or(stem);
    slugify(stem).unwrap_or_else(|_| DEFAULT.to_string())
}

/// The binary file name for an install tag (adr/027): the stock `carpenter`
/// keeps its name; every tagged install swaps `-` for `_` so the command reads
/// `carpenter_nightly` / `carpenter_ivan_topic` (the skill/data identity stays
/// kebab — agent-app skill names forbid `_`).
pub fn binary_file_name(name: &str) -> String {
    if name == "carpenter" {
        String::from("carpenter")
    } else {
        name.replace('-', "_")
    }
}

/// Derive a kebab-case slug from a title.
///
/// Algorithm (`docs/data-model/02-conventions.md`): NFC normalize; lowercase;
/// collapse every run of non-`[a-z0-9]` to a single `-`; trim leading/trailing
/// `-`; truncate to 60 chars (re-trim). (NFC has no observable effect here —
/// every non-ASCII char collapses to `-` anyway — but it is applied for
/// spec fidelity.)
///
/// Returns [`CarpenterError::ValidationError`] if no alphanumerics survive.
/// Collision de-duplication (`-2`, `-3`, …) is the caller's job (it needs DB
/// access to test uniqueness within a scope).
pub fn slugify(title: &str) -> Result<String, CarpenterError> {
    use unicode_normalization::UnicodeNormalization;
    let lower: String = title.nfc().collect::<String>().to_lowercase();
    let mut collapsed = String::with_capacity(lower.len());
    let mut prev_dash = true;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            collapsed.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            collapsed.push('-');
            prev_dash = true;
        }
    }
    let slug: String = collapsed
        .chars()
        .take(60)
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if slug.is_empty() {
        return Err(CarpenterError::ValidationError(format!(
            "cannot derive slug from title {title:?}"
        )));
    }
    Ok(slug)
}

/// Validate a user-provided slug against the slug convention
/// (`docs/data-model/02-conventions.md`): non-empty, ≤ 60 chars,
/// `^[a-z0-9]+(-[a-z0-9]+)*$` (single-`-`-joined lowercase ASCII segments).
///
/// Derived slugs ([`slugify`]) satisfy this by construction; a *provided*
/// slug must be checked so the course/lesson directory name and the DB row
/// can never diverge on a Unicode-normalizing filesystem (adr/017).
pub fn validate_slug(slug: &str) -> Result<(), CarpenterError> {
    let shape_ok = |s: &str| {
        !s.is_empty()
            && !s.starts_with('-')
            && !s.ends_with('-')
            && !s.contains("--")
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if slug.len() > 60 || !shape_ok(slug) {
        return Err(CarpenterError::ValidationError(format!(
            "invalid slug {slug:?}: must be non-empty kebab-case (lowercase ascii \
             [a-z0-9] segments joined by single '-', max 60 chars)"
        )));
    }
    Ok(())
}

/// Map an [`std::io::Error`] to [`CarpenterError::StoreError`].
pub fn io_to_store(e: std::io::Error) -> CarpenterError {
    CarpenterError::StoreError(e.to_string())
}

/// Parse a `--spec` string into a typed spec. **YAML-only**
/// ([adr/014](../../docs/adr/014-yaml-spec-input.md)): `serde_yml::from_str` is
/// the single parser (block scalars for multi-line `content`, no JSON `\n`/`\"`
/// escaping). A parse failure ⇒ `ValidationError`. Note YAML is a superset of
/// JSON, so a flow-style JSON mapping still parses — there is just no separate
/// JSON code path.
pub fn parse_spec<T: serde::de::DeserializeOwned + 'static>(
    text: &str,
) -> Result<T, CarpenterError> {
    serde_yml::from_str(text).map_err(|e| CarpenterError::ValidationError(format!("bad spec: {e}")))
}

/// Atomically write bytes to `path` (temp file + rename on the same filesystem).
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CarpenterError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_to_store)?;
    }
    let tmp = path.with_extension("carpenter-tmp");
    std::fs::write(&tmp, bytes).map_err(io_to_store)?;
    std::fs::rename(&tmp, path).map_err(io_to_store)?;
    Ok(())
}

/// Scaffold a course directory: write `course.json`, open `course.db` with the
/// schema applied, and insert the `course_meta` row. Shared by `course create`
/// and `build` so the on-disk course shape cannot drift between them.
pub fn init_course_dir(
    dir: &Path,
    slug: &str,
    title: &str,
    goal: &str,
    description: &str,
) -> Result<(), CarpenterError> {
    let now = crate::core::time::now_iso();
    let course_json = serde_json::json!({
        "slug": slug,
        "title": title,
        "goal": goal,
        "description": description,
        "created_at": now,
    });
    atomic_write(&dir.join("course.json"), course_json.to_string().as_bytes())?;
    let conn = crate::core::db::open(&dir.join("course.db"))?;
    crate::core::db::insert_course_meta(
        &conn,
        &crate::models::CourseRow {
            slug: slug.into(),
            title: title.into(),
            goal: goal.into(),
            description: description.into(),
            created_at: now,
        },
    )?;
    Ok(())
}

/// Is `dir` on `$PATH`? (Used by `install` to report `on_path`.)
pub fn is_on_path(dir: &Path) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    let target = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    path_var
        .split(':')
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .any(|p| p.canonicalize().ok().as_deref() == Some(&target))
}

/// Read a `--spec` argument: `-` for stdin, otherwise a file path.
pub fn read_spec(arg: &str) -> Result<String, CarpenterError> {
    if arg == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(io_to_store)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(arg).map_err(io_to_store)
    }
}

#[cfg(test)]
fn assert_slug(input: &str, expected: &str) {
    assert_eq!(slugify(input).expect("slug ok"), expected);
}

#[cfg(test)]
#[test]
fn slugify_basic() {
    assert_slug("Data Structures", "data-structures");
}

#[cfg(test)]
#[test]
fn slugify_collapses_runs() {
    assert_slug("Arrays  101!!!", "arrays-101");
}

#[cfg(test)]
#[test]
fn slugify_trims_edges() {
    assert_slug("-- Hello --", "hello");
}

#[cfg(test)]
#[test]
fn slugify_truncates_at_60() {
    let long = "a".repeat(80);
    let s = slugify(&long).expect("ok");
    assert_eq!(s.len(), 60);
}

#[cfg(test)]
#[test]
fn slugify_non_ascii_collapses_to_dash() {
    assert_slug("Café ☕", "caf");
}

#[cfg(test)]
#[test]
fn validate_slug_accepts_kebab_case() {
    for ok in ["a", "data-structures", "linalg-for-ml", "b2b-funnel-42"] {
        validate_slug(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
    }
}

#[cfg(test)]
#[test]
fn validate_slug_rejects_non_kebab_shapes() {
    for bad in [
        "",
        "-leading",
        "trailing-",
        "double--dash",
        "Upper",
        "under_score",
        "space in",
        "unicode-ñ",
        "日本",
        "dot.slug",
        &"x".repeat(61),
    ] {
        assert!(
            matches!(validate_slug(bad), Err(CarpenterError::ValidationError(_))),
            "{bad:?} should be rejected"
        );
    }
}

#[cfg(test)]
#[test]
fn slugify_no_alnums_is_validation_error() {
    let err = slugify("!!!");
    assert!(matches!(err, Err(CarpenterError::ValidationError(_))));
}

#[cfg(test)]
#[test]
fn config_dir_is_named_for_the_install() {
    if let Some(d) = config_dir() {
        assert_eq!(d.file_name().unwrap().to_string_lossy(), install_name());
    }
}

#[cfg(test)]
#[test]
fn install_name_stock_binary_maps_to_itself() {
    assert_eq!(install_name_from(Some("carpenter")), "carpenter");
    assert_eq!(install_name_from(None), "carpenter");
}

#[cfg(test)]
#[test]
fn install_name_branch_stems_are_kebab_tagged() {
    assert_eq!(
        install_name_from(Some("carpenter-ivan-topic")),
        "carpenter-ivan-topic"
    );
    assert_eq!(install_name_from(Some("ivan_feature")), "ivan-feature");
    assert_eq!(install_name_from(Some("Feature.Branch")), "feature-branch");
}

#[cfg(test)]
#[test]
fn install_name_strips_deleted_and_falls_back() {
    // upgrade's in-place replace makes the kernel report ` (deleted)`
    assert_eq!(install_name_from(Some("carpenter (deleted)")), "carpenter");
    assert_eq!(
        install_name_from(Some("carpenter-ivan-x (deleted)")),
        "carpenter-ivan-x"
    );
    // a stem with no alphanumerics cannot be a tag — stock fallback
    assert_eq!(install_name_from(Some("!!!")), "carpenter");
}

#[cfg(test)]
#[test]
fn underscored_command_names_map_onto_kebab_identity() {
    assert_eq!(
        install_name_from(Some("carpenter_nightly")),
        "carpenter-nightly"
    );
    assert_eq!(
        install_name_from(Some("carpenter_ivan_topic")),
        "carpenter-ivan-topic"
    );
}

#[cfg(test)]
#[test]
fn binary_file_name_swaps_dashes_for_underscores() {
    assert_eq!(binary_file_name("carpenter"), "carpenter");
    assert_eq!(binary_file_name("carpenter-nightly"), "carpenter_nightly");
    assert_eq!(
        binary_file_name("carpenter-ivan-topic"),
        "carpenter_ivan_topic"
    );
}

#[cfg(test)]
#[test]
fn atomic_write_roundtrips() {
    let path = std::env::temp_dir().join(format!(
        "carpenter-aw-{}-{}.txt",
        std::process::id(),
        std::sync::atomic::AtomicUsize::new(0).fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    atomic_write(&path, b"hello").expect("write");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    let _ = std::fs::remove_file(&path);
}

#[cfg(test)]
#[derive(serde::Deserialize, PartialEq, Debug)]
struct SpecProbe {
    name: String,
    n: i64,
    content: String,
}

#[cfg(test)]
#[test]
fn parse_spec_accepts_yaml() {
    let yaml = "name: x\nn: 2\ncontent: a\n";
    let v: SpecProbe = parse_spec(yaml).expect("valid YAML spec");
    assert_eq!(v.name, "x");
    assert_eq!(v.n, 2);
}

#[cfg(test)]
#[test]
fn parse_spec_yaml_block_scalar_preserves_newlines() {
    // The motivating win for YAML: multi-line `content` without `\n`/`\"` escaping.
    let yaml = "name: x\nn: 2\ncontent: |\n  line one\n  line two\n";
    let v: SpecProbe = parse_spec(yaml).expect("YAML block scalar");
    assert_eq!(v.content, "line one\nline two\n");
}

#[cfg(test)]
#[test]
fn parse_spec_rejects_missing_required_field() {
    // Valid YAML, wrong shape (missing `n`) ⇒ ValidationError.
    let err = parse_spec::<SpecProbe>("name: x\ncontent: a\n").unwrap_err();
    assert!(matches!(err, CarpenterError::ValidationError(_)));
}

#[cfg(test)]
#[test]
fn parse_spec_rejects_garbage() {
    // Structurally broken in both JSON and YAML ⇒ ValidationError.
    let err = parse_spec::<SpecProbe>("{[}").unwrap_err();
    assert!(matches!(err, CarpenterError::ValidationError(_)));
}
