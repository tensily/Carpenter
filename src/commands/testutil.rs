//! Test helpers shared across command tests (test-only).

use crate::commands::course;
use crate::core::store::Paths;
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

const COURSE_SPEC: &str = "title: Data Structures\ngoal: learn DS\n";

/// Create a temp workspace with one course; return `(paths, course_slug)`.
pub fn setup() -> (Paths, String) {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("carpenter-p3-{}-{n}", std::process::id()));
    let config_dir =
        std::env::temp_dir().join(format!("carpenter-p3cfg-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&config_dir);
    let paths = Paths {
        root,
        config_dir: Some(config_dir),
    };
    let slug = match course::create(&paths, COURSE_SPEC).expect("setup course") {
        crate::models::Data::CourseCreate { slug, .. } => slug,
        _ => unreachable!(),
    };
    (paths, slug)
}

/// Create a temp workspace (root + config_dir) with no course; for meta-command
/// tests (bug/feature, config, register, …). `config_dir` is a `carpenter` leaf
/// under a unique parent so [`Paths::xdg_root`](crate::core::store::Paths::xdg_root)
/// (the sibling `opencode/` anchor) is per-test-isolated.
pub fn meta_setup() -> Paths {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("carpenter-meta-{}-{n}", std::process::id()));
    let config_dir = root.join("xdg").join("carpenter");
    let _ = std::fs::remove_dir_all(&root);
    Paths {
        root,
        config_dir: Some(config_dir),
    }
}
