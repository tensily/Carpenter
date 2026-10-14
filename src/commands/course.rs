//! `course` commands — create/list/show/update/delete/switch.
//!
//! All fns are pure over their arguments (a [`Paths`] + typed inputs); app.rs
//! does the clap wiring and `--spec` reading, then calls these.

use std::fs;

use crate::core::config;
use crate::core::db;
use crate::core::error::CarpenterError;
use crate::core::store::{self, Paths};
use crate::models::course::{CourseCounts, CourseRow, CourseSpec};
use crate::models::{common::RowError, Data};

/// Create a course from a spec YAML (`--spec -`/file).
pub fn create(paths: &Paths, spec_text: &str) -> Result<Data, CarpenterError> {
    let spec: CourseSpec = store::parse_spec(spec_text)?;
    validate_spec(&spec)?;
    let slug = match &spec.slug {
        Some(s) => {
            store::validate_slug(s)?;
            s.clone()
        }
        None => store::slugify(&spec.title)?,
    };
    let dir = paths.course(&slug);
    if dir.exists() {
        return Err(CarpenterError::AlreadyExists(format!("course {slug}")));
    }
    store::init_course_dir(&dir, &slug, &spec.title, &spec.goal, &spec.description)?;
    Ok(Data::CourseCreate {
        slug,
        title: spec.title,
        path: dir.display().to_string(),
    })
}

/// List all courses under `<root>/courses/`; corrupt ones surface in `errors[]`.
pub fn list(paths: &Paths) -> Result<Data, CarpenterError> {
    let mut items = Vec::new();
    let mut errors = Vec::new();
    let courses_dir = paths.courses();
    if courses_dir.exists() {
        for entry in fs::read_dir(&courses_dir).map_err(store::io_to_store)? {
            let Ok(entry) = entry else {
                continue;
            };
            let slug = entry.file_name().to_string_lossy().to_string();
            let db_path = entry.path().join("course.db");
            if !db_path.exists() {
                errors.push(RowError {
                    id: Some(slug),
                    reason: String::from("corrupt_course"),
                });
                continue;
            }
            match db::open(&db_path).and_then(|conn| {
                db::get_course_meta(&conn, &slug)
                    .and_then(|row| db::lessons_count(&conn).map(|c| (row, c)))
            }) {
                Ok((row, count)) => items.push(crate::models::CourseListItem {
                    slug: row.slug,
                    title: row.title,
                    goal: row.goal,
                    lessons_count: count,
                }),
                Err(_) => errors.push(RowError {
                    id: Some(slug),
                    reason: String::from("corrupt_course"),
                }),
            }
        }
    }
    items.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(Data::CourseList {
        courses: items,
        errors,
    })
}

/// Show one course with per-table counts.
pub fn show(paths: &Paths, slug: &str) -> Result<Data, CarpenterError> {
    let conn = open_course(paths, slug)?;
    let row = db::get_course_meta(&conn, slug)?;
    let db_counts = db::course_counts(&conn)?;
    Ok(Data::CourseShow {
        slug: row.slug,
        title: row.title,
        goal: row.goal,
        description: row.description,
        counts: CourseCounts {
            lessons: db_counts.lessons,
            sections: db_counts.sections,
            practice: db_counts.practice,
            quizzes: db_counts.quizzes,
        },
    })
}

/// Update a course's mutable fields from a spec (requires `--force`).
pub fn update(
    paths: &Paths,
    slug: &str,
    spec_text: &str,
    force: bool,
) -> Result<Data, CarpenterError> {
    if !force {
        return Err(CarpenterError::Conflict(format!(
            "update requires --force: course {slug}"
        )));
    }
    let spec: CourseSpec = store::parse_spec(spec_text)?;
    validate_spec(&spec)?;
    let dir = paths.course(slug);
    let conn = open_course(paths, slug)?;
    let existing = db::get_course_meta(&conn, slug)?;
    db::update_course_meta(&conn, slug, &spec.title, &spec.goal, &spec.description)?;
    let course_json = serde_json::json!({
        "slug": slug,
        "title": spec.title,
        "goal": spec.goal,
        "description": spec.description,
        "created_at": existing.created_at,
    });
    store::atomic_write(&dir.join("course.json"), course_json.to_string().as_bytes())?;
    Ok(Data::CourseUpdate {
        slug: slug.to_string(),
        updated: CourseRow {
            slug: slug.to_string(),
            title: spec.title,
            goal: spec.goal,
            description: spec.description,
            created_at: existing.created_at,
        },
    })
}

/// Delete a course (requires `--force`).
pub fn delete(paths: &Paths, slug: &str, force: bool) -> Result<Data, CarpenterError> {
    if !force {
        return Err(CarpenterError::Conflict(format!(
            "delete requires --force: course {slug}"
        )));
    }
    let dir = paths.course(slug);
    if !dir.exists() {
        return Err(CarpenterError::NotFound(format!("course {slug}")));
    }
    fs::remove_dir_all(&dir).map_err(store::io_to_store)?;
    Ok(Data::CourseDelete {
        slug: slug.to_string(),
        deleted: true,
    })
}

/// Switch the active course (writes `active_course` to config).
pub fn switch(paths: &Paths, slug: &str) -> Result<Data, CarpenterError> {
    let path = paths
        .config_file()
        .ok_or_else(|| CarpenterError::StoreError(String::from("no config directory resolved")))?;
    let mut cfg = config::load_from(&path);
    cfg.active_course = Some(slug.to_string());
    config::save_to(&path, &cfg)?;
    Ok(Data::CourseSwitch {
        active_course: slug.to_string(),
    })
}

fn open_course(paths: &Paths, slug: &str) -> Result<rusqlite::Connection, CarpenterError> {
    let db_path = paths.course(slug).join("course.db");
    if !db_path.exists() {
        return Err(CarpenterError::NotFound(format!("course {slug}")));
    }
    db::open(&db_path)
}

fn validate_spec(spec: &CourseSpec) -> Result<(), CarpenterError> {
    if spec.title.trim().is_empty() {
        return Err(CarpenterError::ValidationError(
            "title must be non-empty".into(),
        ));
    }
    if spec.goal.trim().is_empty() {
        return Err(CarpenterError::ValidationError(
            "goal must be non-empty".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn test_paths() -> Paths {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("carpenter-root-{}-{n}", std::process::id()));
        let config_dir =
            std::env::temp_dir().join(format!("carpenter-cfgdir-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&config_dir);
        Paths {
            root,
            config_dir: Some(config_dir),
        }
    }

    fn cleanup(paths: &Paths) {
        let _ = std::fs::remove_dir_all(&paths.root);
        if let Some(c) = &paths.config_dir {
            let _ = std::fs::remove_dir_all(c);
        }
    }

    const SPEC: &str = "title: Data Structures\ngoal: learn DS\n";

    #[test]
    fn create_ok() {
        let p = test_paths();
        let data = create(&p, SPEC).expect("create");
        let Data::CourseCreate { slug, path, .. } = data else {
            panic!("CourseCreate");
        };
        assert_eq!(slug, "data-structures");
        assert!(Path::new(&path).join("course.db").exists());
        assert!(Path::new(&path).join("course.json").exists());
        cleanup(&p);
    }

    #[test]
    fn create_rejects_duplicate() {
        let p = test_paths();
        create(&p, SPEC).expect("first");
        let err = create(&p, SPEC).unwrap_err();
        assert!(matches!(err, CarpenterError::AlreadyExists(_)));
        cleanup(&p);
    }

    #[test]
    fn create_rejects_bad_spec() {
        let p = test_paths();
        let err = create(&p, "title: ''\ngoal: g\n").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let err = create(&p, "{[}").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        cleanup(&p);
    }

    #[test]
    fn create_rejects_non_kebab_slug() {
        let p = test_paths();
        let err = create(&p, "title: T\ngoal: g\nslug: unicode-ñ\n").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)), "{err}");
        let err = create(&p, "title: T\ngoal: g\nslug: Bad_Slug\n").unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)), "{err}");
        assert!(!p.course("unicode-ñ").exists());
        cleanup(&p);
    }

    #[test]
    fn list_ok() {
        let p = test_paths();
        create(&p, SPEC).expect("c1");
        create(&p, "title: Algorithms\ngoal: learn algos\n").expect("c2");
        let Data::CourseList { courses, errors } = list(&p).expect("list") else {
            panic!("CourseList");
        };
        assert!(errors.is_empty());
        assert_eq!(courses.len(), 2);
        assert!(courses.iter().any(|c| c.slug == "algorithms"));
        cleanup(&p);
    }

    #[test]
    fn show_ok() {
        let p = test_paths();
        create(&p, SPEC).expect("create");
        let Data::CourseShow { slug, counts, .. } = show(&p, "data-structures").expect("show")
        else {
            panic!("CourseShow");
        };
        assert_eq!(slug, "data-structures");
        assert_eq!(counts.lessons, 0);
        cleanup(&p);
    }

    #[test]
    fn show_not_found() {
        let p = test_paths();
        let err = show(&p, "nope").unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)));
        cleanup(&p);
    }

    #[test]
    fn update_ok() {
        let p = test_paths();
        create(&p, SPEC).expect("create");
        let data = update(
            &p,
            "data-structures",
            "title: DS2\ngoal: g2\ndescription: d2\n",
            true,
        )
        .expect("update");
        let Data::CourseUpdate { updated, .. } = data else {
            panic!("CourseUpdate");
        };
        assert_eq!(updated.title, "DS2");
        assert_eq!(updated.description, "d2");
        cleanup(&p);
    }

    #[test]
    fn update_without_force_conflicts() {
        let p = test_paths();
        create(&p, SPEC).expect("create");
        let err = update(&p, "data-structures", SPEC, false).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
        cleanup(&p);
    }

    #[test]
    fn delete_ok() {
        let p = test_paths();
        create(&p, SPEC).expect("create");
        let _ = delete(&p, "data-structures", true).expect("delete");
        assert!(!p.course("data-structures").exists());
        cleanup(&p);
    }

    #[test]
    fn delete_without_force_conflicts() {
        let p = test_paths();
        create(&p, SPEC).expect("create");
        let err = delete(&p, "data-structures", false).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
        cleanup(&p);
    }

    #[test]
    fn switch_ok() {
        let p = test_paths();
        let Data::CourseSwitch { active_course } = switch(&p, "ds").expect("switch") else {
            panic!("CourseSwitch");
        };
        assert_eq!(active_course, "ds");
        let cfg_path = p.config_file().unwrap();
        let cfg = config::load_from(&cfg_path);
        assert_eq!(cfg.active_course.as_deref(), Some("ds"));
        cleanup(&p);
    }
}
