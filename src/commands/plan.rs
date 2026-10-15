//! `plan` commands — create/show/list/confirm/update/delete (human-in-the-loop).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::core::error::CarpenterError;
use crate::core::store;
use crate::core::{db, store::Paths, time};
use crate::models::goal::GoalRow;
use crate::models::plan::{PlanListItem, PlanRow, PlanSpec};
use crate::models::Data;

/// The stored plan body (parsed back from `plans.content` on confirm).
#[derive(Debug, Deserialize, Default)]
struct PlanContent {
    #[serde(default)]
    goals: Vec<String>,
    #[serde(default)]
    links: BTreeMap<String, Vec<String>>,
}

fn encode_content(spec: &PlanSpec) -> String {
    serde_json::json!({"goals": spec.goals, "links": spec.links}).to_string()
}

fn link_index(key: &str) -> Option<usize> {
    key.strip_prefix("goal_index_")?.parse::<usize>().ok()
}

fn validate_plan_spec(spec: &PlanSpec) -> Result<(), CarpenterError> {
    if spec.title.trim().is_empty() {
        return Err(CarpenterError::ValidationError(
            "title must be non-empty".into(),
        ));
    }
    for key in spec.links.keys() {
        let Some(i) = link_index(key) else {
            return Err(CarpenterError::ValidationError(format!(
                "links key {key:?} must be `goal_index_<i>`"
            )));
        };
        if i >= spec.goals.len() {
            return Err(CarpenterError::ValidationError(format!(
                "links key {key} is out of range (goals has {})",
                spec.goals.len()
            )));
        }
    }
    Ok(())
}

/// Create a plan draft from a spec.
///
/// `--scope lesson` requires `--lesson <id>` (a lesson that exists); the plan's
/// `scope_id` is then that lesson id. `--scope course` ignores `--lesson`.
pub fn create(
    paths: &Paths,
    course_slug: &str,
    scope: &str,
    lesson: Option<&str>,
    spec_text: &str,
) -> Result<Data, CarpenterError> {
    let scope_id = match scope {
        "course" => {
            if lesson.is_some() {
                return Err(CarpenterError::ValidationError(
                    "--lesson requires --scope lesson".into(),
                ));
            }
            course_slug.to_string()
        }
        "lesson" => {
            let lid = lesson.ok_or_else(|| {
                CarpenterError::ValidationError("--scope lesson requires --lesson <id>".into())
            })?;
            let conn = db::open_course(paths, course_slug)?;
            if !db::lesson_exists(&conn, lid)? {
                return Err(CarpenterError::NotFound(format!("lesson {lid}")));
            }
            lid.to_string()
        }
        other => {
            return Err(CarpenterError::ValidationError(format!(
                "invalid --scope {other:?} (course|lesson)"
            )))
        }
    };
    let spec: PlanSpec = store::parse_spec(spec_text)?;
    validate_plan_spec(&spec)?;
    let conn = db::open_course(paths, course_slug)?;
    let id = db::next_id(&conn, "plans", "pl")?;
    let content = encode_content(&spec);
    let row = PlanRow {
        id: id.clone(),
        scope: scope.into(),
        scope_id: scope_id.clone(),
        title: spec.title.clone(),
        content: content.clone(),
        created_at: time::now_iso(),
        confirmed_at: None,
    };
    db::insert_plan(&conn, &row)?;
    Ok(Data::PlanCreate {
        id,
        scope: scope.into(),
        scope_id,
        title: spec.title,
        content,
        confirmed: false,
    })
}

/// Show one plan.
pub fn show(paths: &Paths, course_slug: &str, id: &str) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let row = db::get_plan(&conn, id)?;
    Ok(Data::PlanShow {
        id: row.id,
        scope: row.scope,
        scope_id: row.scope_id,
        title: row.title,
        content: row.content,
        confirmed_at: row.confirmed_at,
    })
}

/// List plans, optionally filtered by scope.
pub fn list(paths: &Paths, course_slug: &str, scope: Option<&str>) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let rows = db::list_plans(&conn, scope)?;
    let plans = rows
        .into_iter()
        .map(|r| PlanListItem {
            confirmed: r.confirmed_at.is_some(),
            id: r.id,
            scope: r.scope,
            scope_id: r.scope_id,
            title: r.title,
        })
        .collect();
    Ok(Data::PlanList { plans })
}

/// Confirm a plan (course scope materializes goals from `goals[]` + `links`).
///
/// Materialization runs in one transaction: a failed confirm (e.g. an
/// unresolvable lesson id in `links`) rolls back the goal inserts *and* the
/// `id_seq` bump, so ids are only consumed by confirms that commit.
pub fn confirm(paths: &Paths, course_slug: &str, id: &str) -> Result<Data, CarpenterError> {
    let mut conn = db::open_course(paths, course_slug)?;
    let row = db::get_plan(&conn, id)?;
    if row.confirmed_at.is_some() {
        return Err(CarpenterError::Conflict(format!(
            "plan {id} already confirmed"
        )));
    }
    let now = time::now_iso();
    let mut goals_created = Vec::new();
    let tx = db::transaction(&mut conn)?;
    if row.scope == "course" {
        let content: PlanContent = serde_json::from_str(&row.content)
            .map_err(|e| CarpenterError::StoreError(format!("plan content unreadable: {e}")))?;
        for (i, text) in content.goals.iter().enumerate() {
            let gid = db::next_id(&tx, "goals", "g")?;
            let covered = content
                .links
                .get(&format!("goal_index_{i}"))
                .cloned()
                .unwrap_or_default();
            for lid in &covered {
                if !db::lesson_exists(&tx, lid)? {
                    return Err(CarpenterError::ValidationError(format!(
                        "unresolvable lesson id in links: {lid}"
                    )));
                }
            }
            let goal = GoalRow {
                id: gid.clone(),
                scope: String::from("course"),
                scope_id: row.scope_id.clone(),
                text: text.clone(),
                status: String::from("pending"),
                covered_by: covered,
                override_flag: false,
                created_at: now.clone(),
            };
            db::insert_goal(&tx, &goal)?;
            goals_created.push(gid);
        }
    }
    db::set_plan_confirmed(&tx, id, &now)?;
    db::commit(tx)?;
    Ok(Data::PlanConfirm {
        id: id.into(),
        confirmed: true,
        confirmed_at: now,
        goals_created,
    })
}

/// Update a plan's title + body (only if not confirmed).
pub fn update(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    spec_text: &str,
) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let row = db::get_plan(&conn, id)?;
    if row.confirmed_at.is_some() {
        return Err(CarpenterError::Conflict(format!(
            "cannot update confirmed plan {id}"
        )));
    }
    let spec: PlanSpec = store::parse_spec(spec_text)?;
    validate_plan_spec(&spec)?;
    let content = encode_content(&spec);
    db::replace_plan(&conn, id, &spec.title, &content)?;
    Ok(Data::PlanUpdate {
        id: id.into(),
        updated: PlanRow {
            id: id.into(),
            scope: row.scope,
            scope_id: row.scope_id,
            title: spec.title,
            content,
            created_at: row.created_at,
            confirmed_at: None,
        },
    })
}

/// Delete a plan (`--force` required if it is confirmed).
pub fn delete(
    paths: &Paths,
    course_slug: &str,
    id: &str,
    force: bool,
) -> Result<Data, CarpenterError> {
    let conn = db::open_course(paths, course_slug)?;
    let row = db::get_plan(&conn, id)?;
    if row.confirmed_at.is_some() && !force {
        return Err(CarpenterError::Conflict(format!(
            "cannot delete confirmed plan {id} without --force"
        )));
    }
    db::delete_plan(&conn, id)?;
    Ok(Data::PlanDelete {
        id: id.into(),
        deleted: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testutil;
    use crate::models::Data;

    const SPEC: &str = "title: DS plan\ngoals:\n  - goal a\n  - goal b\nlinks: {}\n";

    fn plan_id(data: &Data) -> String {
        match data {
            Data::PlanCreate { id, .. } => id.clone(),
            _ => panic!("not a PlanCreate"),
        }
    }

    #[test]
    fn create_ok() {
        let (paths, slug) = testutil::setup();
        let data = create(&paths, &slug, "course", None, SPEC).expect("create");
        assert!(matches!(
            data,
            Data::PlanCreate {
                confirmed: false,
                ..
            }
        ));
    }

    #[test]
    fn create_lesson_scope_ok() {
        let (paths, slug) = testutil::setup();
        crate::commands::lesson::create(
            &paths,
            &slug,
            "title: Arrays\nslug: arrays\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: hi\nquizzes: []\n",
        )
        .expect("create lesson");
        let data = create(&paths, &slug, "lesson", Some("arrays"), SPEC).expect("create");
        match data {
            Data::PlanCreate {
                scope, scope_id, ..
            } => {
                assert_eq!(scope, "lesson");
                assert_eq!(scope_id, "arrays");
            }
            _ => panic!("PlanCreate"),
        }
    }

    #[test]
    fn create_lesson_scope_requires_flag() {
        let (paths, slug) = testutil::setup();
        let err = create(&paths, &slug, "lesson", None, SPEC).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
    }

    #[test]
    fn create_lesson_scope_unknown_lesson_is_not_found() {
        let (paths, slug) = testutil::setup();
        let err = create(&paths, &slug, "lesson", Some("arrays"), SPEC).unwrap_err();
        assert!(matches!(err, CarpenterError::NotFound(_)));
    }

    #[test]
    fn create_course_scope_rejects_lesson_flag() {
        let (paths, slug) = testutil::setup();
        let err = create(&paths, &slug, "course", Some("arrays"), SPEC).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
    }

    #[test]
    fn create_rejects_bad_link_index() {
        let (paths, slug) = testutil::setup();
        let bad = "title: t\ngoals: [only]\nlinks:\n  goal_index_3: [x]\n";
        let err = create(&paths, &slug, "course", None, bad).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
    }

    #[test]
    fn show_ok() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        assert!(matches!(
            show(&paths, &slug, &id).expect("show"),
            Data::PlanShow { .. }
        ));
    }

    #[test]
    fn list_ok() {
        let (paths, slug) = testutil::setup();
        create(&paths, &slug, "course", None, SPEC).unwrap();
        assert!(matches!(
            list(&paths, &slug, None).expect("list"),
            Data::PlanList { .. }
        ));
    }

    #[test]
    fn confirm_ok_creates_goals() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        let data = confirm(&paths, &slug, &id).expect("confirm");
        match data {
            Data::PlanConfirm { goals_created, .. } => assert_eq!(goals_created.len(), 2),
            _ => panic!("PlanConfirm"),
        }
    }

    #[test]
    fn confirm_twice_conflicts() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        confirm(&paths, &slug, &id).unwrap();
        let err = confirm(&paths, &slug, &id).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
    }

    #[test]
    fn confirm_failed_confirm_does_not_consume_goal_ids() {
        let (paths, slug) = testutil::setup();
        let ghost = "title: t\ngoals: [a, b, c]\nlinks:\n  goal_index_0: [ghost-lesson]\n";
        let bad = plan_id(&create(&paths, &slug, "course", None, ghost).unwrap());
        let err = confirm(&paths, &slug, &bad).unwrap_err();
        assert!(matches!(err, CarpenterError::ValidationError(_)));
        let good = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        let data = confirm(&paths, &slug, &good).expect("confirm after failed confirm");
        match data {
            Data::PlanConfirm { goals_created, .. } => {
                assert_eq!(goals_created, ["g1", "g2"]);
            }
            _ => panic!("PlanConfirm"),
        }
    }

    #[test]
    fn confirm_lesson_scope_creates_no_goals() {
        let (paths, slug) = testutil::setup();
        crate::commands::lesson::create(
            &paths,
            &slug,
            "title: Arrays\nslug: arrays\nsections:\n  - title: s\n    snippets:\n      - kind: markdown\n        content: hi\nquizzes: []\n",
        )
        .unwrap();
        let id = plan_id(&create(&paths, &slug, "lesson", Some("arrays"), SPEC).unwrap());
        let data = confirm(&paths, &slug, &id).expect("confirm");
        match data {
            Data::PlanConfirm { goals_created, .. } => {
                assert!(
                    goals_created.is_empty(),
                    "lesson scope never materializes goals"
                );
            }
            _ => panic!("PlanConfirm"),
        }
    }

    #[test]
    fn update_ok() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        assert!(matches!(
            update(&paths, &slug, &id, "title: t2\ngoals: []\nlinks: {}\n").expect("update"),
            Data::PlanUpdate { .. }
        ));
    }

    #[test]
    fn update_confirmed_conflicts() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        confirm(&paths, &slug, &id).unwrap();
        let err = update(&paths, &slug, &id, SPEC).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
    }

    #[test]
    fn delete_ok_unconfirmed() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        assert!(matches!(
            delete(&paths, &slug, &id, false).expect("delete"),
            Data::PlanDelete { deleted: true, .. }
        ));
    }

    #[test]
    fn delete_confirmed_needs_force() {
        let (paths, slug) = testutil::setup();
        let id = plan_id(&create(&paths, &slug, "course", None, SPEC).unwrap());
        confirm(&paths, &slug, &id).unwrap();
        let err = delete(&paths, &slug, &id, false).unwrap_err();
        assert!(matches!(err, CarpenterError::Conflict(_)));
        assert!(matches!(
            delete(&paths, &slug, &id, true).expect("delete forced"),
            Data::PlanDelete { deleted: true, .. }
        ));
    }
}
