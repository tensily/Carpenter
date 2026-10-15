//! Compare semantics — the Rust reference implementation.
//!
//! [`compare`] mirrors the helper's Python `_compare` (parity-tested in P6).
//! Edge cases: `sorted` on non-mutually-sortable elements ⇒ [`CompareError::Unsortable`];
//! `set` on unhashable elements ⇒ [`CompareError::Unhashable`]. (`expected` is JSON,
//! so NaN cannot appear; the helper's `NaN != NaN` rule is exercised on the Python side.)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashSet;

/// Compare mode for a test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    /// `actual == expected` (default).
    #[default]
    Exact,
    /// `sorted(actual) == sorted(expected)`.
    Sorted,
    /// `set(actual) == set(expected)`.
    Set,
}

impl CompareMode {
    /// The CLI/DB spelling (`exact` | `sorted` | `set`).
    pub fn as_str(self) -> &'static str {
        match self {
            CompareMode::Exact => "exact",
            CompareMode::Sorted => "sorted",
            CompareMode::Set => "set",
        }
    }
}

/// A compare error — the case errors (it is not a crash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// `sorted` on elements that are not mutually sortable (e.g. mixed number/dict).
    Unsortable,
    /// `set` on an unhashable element (array/object).
    Unhashable,
}

/// Outcome of one compare: pass, fail, or case-error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareOutcome {
    /// actual matched expected.
    Pass,
    /// actual did not match.
    Fail,
    /// the case errored (unsortable/unhashable).
    Error(CompareError),
}

/// Compare `actual` against `expected` under `mode`.
pub fn compare(actual: &Value, expected: &Value, mode: CompareMode) -> CompareOutcome {
    match mode {
        CompareMode::Exact => {
            if actual == expected {
                CompareOutcome::Pass
            } else {
                CompareOutcome::Fail
            }
        }
        CompareMode::Sorted => compare_sorted(actual, expected),
        CompareMode::Set => compare_set(actual, expected),
    }
}

fn compare_sorted(actual: &Value, expected: &Value) -> CompareOutcome {
    let (Some(a), Some(b)) = (actual.as_array(), expected.as_array()) else {
        return CompareOutcome::Fail;
    };
    let mut a = a.clone();
    let mut b = b.clone();
    if sort_values(&mut a).is_err() {
        return CompareOutcome::Error(CompareError::Unsortable);
    }
    if sort_values(&mut b).is_err() {
        return CompareOutcome::Error(CompareError::Unsortable);
    }
    if a == b {
        CompareOutcome::Pass
    } else {
        CompareOutcome::Fail
    }
}

fn compare_set(actual: &Value, expected: &Value) -> CompareOutcome {
    let (Some(a), Some(b)) = (actual.as_array(), expected.as_array()) else {
        return CompareOutcome::Fail;
    };
    match (to_set(a), to_set(b)) {
        (Err(e), _) | (_, Err(e)) => CompareOutcome::Error(e),
        (Ok(sa), Ok(sb)) => {
            if sa == sb {
                CompareOutcome::Pass
            } else {
                CompareOutcome::Fail
            }
        }
    }
}

/// A total order over same-category values; `None` if not mutually comparable
/// (mirrors Python's `TypeError` on mixed category / dict / null comparison).
fn cmp_values(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => Some((*x as u8).cmp(&(*y as u8))),
        (Value::Number(x), Value::Number(y)) => num_cmp(x, y),
        (Value::Bool(x), Value::Number(y)) => x_f64_cmp(*x as u8 as f64, y),
        (Value::Number(x), Value::Bool(y)) => {
            x_f64_cmp(number_f64(x)?, &serde_json::Number::from(*y as u8 as i64))
        }
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Array(x), Value::Array(y)) => cmp_arrays(x, y),
        _ => None,
    }
}

fn number_f64(n: &serde_json::Number) -> Option<f64> {
    if let Some(i) = n.as_i64() {
        return Some(i as f64);
    }
    if let Some(u) = n.as_u64() {
        return Some(u as f64);
    }
    n.as_f64()
}

fn num_cmp(x: &serde_json::Number, y: &serde_json::Number) -> Option<Ordering> {
    let xf = number_f64(x)?;
    let yf = number_f64(y)?;
    xf.partial_cmp(&yf)
}

fn x_f64_cmp(x: f64, y: &serde_json::Number) -> Option<Ordering> {
    x.partial_cmp(&number_f64(y)?)
}

fn cmp_arrays(x: &[Value], y: &[Value]) -> Option<Ordering> {
    for (a, b) in x.iter().zip(y.iter()) {
        match cmp_values(a, b)? {
            Ordering::Equal => continue,
            ord => return Some(ord),
        }
    }
    Some(x.len().cmp(&y.len()))
}

/// Insertion sort via [`cmp_values`]; `Err` on the first incomparable pair.
fn sort_values(arr: &mut [Value]) -> Result<(), ()> {
    for i in 1..arr.len() {
        let mut j = i;
        while j > 0 {
            match cmp_values(&arr[j - 1], &arr[j]) {
                Some(Ordering::Greater) => {
                    arr.swap(j - 1, j);
                    j -= 1;
                }
                Some(_) => break,
                None => return Err(()),
            }
        }
    }
    Ok(())
}

fn to_set(arr: &[Value]) -> Result<HashSet<String>, CompareError> {
    let mut s = HashSet::new();
    for v in arr {
        if v.is_array() || v.is_object() {
            return Err(CompareError::Unhashable);
        }
        s.insert(canonical_scalar(v));
    }
    Ok(s)
}

/// Canonical string for a scalar (numbers normalized so `6` == `6.0`).
fn canonical_scalar(v: &Value) -> String {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            Some(f) => format!("{f}"),
            None => n.to_string(),
        },
        _ => v.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_pass_and_fail() {
        assert_eq!(
            compare(&json!(6), &json!(6), CompareMode::Exact),
            CompareOutcome::Pass
        );
        assert_eq!(
            compare(&json!(6), &json!(7), CompareMode::Exact),
            CompareOutcome::Fail
        );
        assert_eq!(
            compare(&json!("x"), &json!("y"), CompareMode::Exact),
            CompareOutcome::Fail
        );
    }

    #[test]
    fn sorted_pass() {
        assert_eq!(
            compare(&json!([3, 1, 2]), &json!([1, 2, 3]), CompareMode::Sorted),
            CompareOutcome::Pass
        );
    }

    #[test]
    fn sorted_fail_on_multiset_difference() {
        assert_eq!(
            compare(&json!([1, 1, 2]), &json!([1, 2, 2]), CompareMode::Sorted),
            CompareOutcome::Fail
        );
    }

    #[test]
    fn sorted_strings() {
        assert_eq!(
            compare(&json!(["b", "a"]), &json!(["a", "b"]), CompareMode::Sorted),
            CompareOutcome::Pass
        );
    }

    #[test]
    fn sorted_unsortable_mixed() {
        assert_eq!(
            compare(
                &json!([1, {"a": 1}]),
                &json!([{"a": 1}, 1]),
                CompareMode::Sorted
            ),
            CompareOutcome::Error(CompareError::Unsortable)
        );
    }

    #[test]
    fn set_pass_ignores_order_and_dups() {
        assert_eq!(
            compare(&json!([1, 2, 2]), &json!([2, 1]), CompareMode::Set),
            CompareOutcome::Pass
        );
    }

    #[test]
    fn set_fail_on_difference() {
        assert_eq!(
            compare(&json!([1, 2]), &json!([1, 3]), CompareMode::Set),
            CompareOutcome::Fail
        );
    }

    #[test]
    fn set_unhashable_element() {
        assert_eq!(
            compare(&json!([1, [2]]), &json!([1]), CompareMode::Set),
            CompareOutcome::Error(CompareError::Unhashable)
        );
    }

    #[test]
    fn set_normalizes_six_and_six_point_zero() {
        assert_eq!(
            compare(&json!([6]), &json!([6.0]), CompareMode::Set),
            CompareOutcome::Pass
        );
    }

    /// Parity: the helper's Python `_compare` must agree with the Rust `compare`
    /// across exact/sorted/set + the unsortable/unhashable edges. Runs the
    /// generated `helper.py`'s `_compare` via `python3` (stdlib-only).
    #[test]
    fn parity_with_helper_python() {
        use crate::core::helper::HELPER_PY;
        use std::process::Command;

        let Some(start) = HELPER_PY.find("class _CompareError") else {
            return;
        };
        let Some(rel) = HELPER_PY[start..].find("_TABLE") else {
            return;
        };
        let block = &HELPER_PY[start..start + rel]; // _CompareError + _compare
        let cases: Vec<(Value, Value, &str)> = vec![
            (json!(6), json!(6), "exact"),
            (json!(6), json!(7), "exact"),
            (json!("a"), json!("b"), "exact"),
            (json!([3, 1, 2]), json!([1, 2, 3]), "sorted"),
            (json!([1, 1, 2]), json!([1, 2, 2]), "sorted"),
            (json!(["b", "a"]), json!(["a", "b"]), "sorted"),
            (json!([1, {"a": 1}]), json!([{"a": 1}, 1]), "sorted"),
            (json!([1, 2, 2]), json!([2, 1]), "set"),
            (json!([1, 2]), json!([1, 3]), "set"),
            (json!([1, [2]]), json!([1]), "set"),
        ];
        let cases_json = serde_json::to_string(
            &cases
                .iter()
                .map(|(a, e, m)| json!({"a": a, "e": e, "m": m}))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let script = format!(
            "{block}\n\
             import json\n\
             cases = {cases_json}\n\
             out = []\n\
             for c in cases:\n\
             \x20\x20\x20\x20try:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20r = _compare(c['a'], c['e'], c['m'])\n\
             \x20\x20\x20\x20\x20\x20\x20\x20out.append('pass' if r else 'fail')\n\
             \x20\x20\x20\x20except _CompareError as ex:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20out.append(str(ex))\n\
             print(json.dumps(out))"
        );
        let Ok(out) = Command::new("python3").arg("-c").arg(&script).output() else {
            eprintln!("python3 unavailable — skipping parity test");
            return;
        };
        if !out.status.success() {
            panic!(
                "python parity script failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let py: Vec<String> =
            serde_json::from_slice(&out.stdout).expect("python returned a JSON list");
        let rust: Vec<String> = cases
            .iter()
            .map(|(a, e, m)| {
                let mode = match *m {
                    "exact" => CompareMode::Exact,
                    "sorted" => CompareMode::Sorted,
                    _ => CompareMode::Set,
                };
                match compare(a, e, mode) {
                    CompareOutcome::Pass => "pass",
                    CompareOutcome::Fail => "fail",
                    CompareOutcome::Error(CompareError::Unsortable) => "unsortable",
                    CompareOutcome::Error(CompareError::Unhashable) => "unhashable",
                }
                .to_string()
            })
            .collect();
        assert_eq!(py, rust, "Rust vs Python compare diverged");
    }
}
