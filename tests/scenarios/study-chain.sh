#!/bin/sh
# Scenario: the full study workflow, driven the way a learner studies — one
# lesson at a time (course -> venv -> lesson verify/create/execute -> quiz ->
# plan confirm -> progress). Quiz failure pre-solve is by design (learner
# stubs raise NotImplementedError); the learner is then simulated by editing
# the stub cells, and the quizzes must flip to pass. Simulates the PR #55 QA
# study-chain pass on the validated intro-stats corpus.
#
# Needs uv (venv) + network (deps) + python3 (learner edit); skips gracefully
# when uv is absent — the CI gates job installs uv, so CI runs it for real.

command -v uv >/dev/null 2>&1 || { echo "study-chain: uv not found, skipping"; exit 0; }

set -eu
. "$(dirname "$0")/lib.sh"

root="$SHOME/course"
c() { "$CARPENTER" --root "$root" -c intro-stats "$@"; }
specs="$SHOME/specs"
mkdir -p "$specs"

cat > "$specs/course.yaml" <<'YAML'
title: Intro Statistics with NumPy
slug: intro-stats
goal: Learn applied introductory statistics using NumPy
description: Descriptive statistics, distributions and sampling, correlation and simple linear regression.
YAML

cat > "$specs/lesson-1.yaml" <<'YAML'
title: Descriptive Statistics Fundamentals
slug: descriptive-stats
order: 1
sections:
  - title: Central tendency and spread
    snippets:
      - kind: markdown
        content: |
          ## Mean and standard deviation

          The mean summarizes center; the standard deviation summarizes spread.
          NumPy computes both in one pass over the array.
      - kind: code
        content: |
          import numpy as np
          data = np.array([2, 4, 4, 4, 5, 5, 7, 9])
          print(float(data.mean()), float(data.std()))
    practice:
      - name: mean_value
        signature: "def mean_value(arr):"
        prompt: Return the mean of the array as a Python float.
        solution: |
          def mean_value(arr):
              import numpy as np
              return float(np.mean(arr))
        cases:
          - compare: exact
            args:
              - [1, 2, 3, 4]
            kwargs: {}
            expected: 2.5
      - name: std_value
        signature: "def std_value(arr):"
        prompt: Return the population standard deviation as a float (round to 8 decimals).
        solution: |
          def std_value(arr):
              import numpy as np
              return float(np.round(np.std(arr), 8))
        cases:
          - compare: exact
            args:
              - [2, 4, 4, 4, 5, 5, 7, 9]
            kwargs: {}
            expected: 2.0
quizzes:
  - name: median_value
    signature: "def median_value(arr):"
    prompt: Return the median of the array as a Python float.
    solution: |
      def median_value(arr):
          import numpy as np
          return float(np.median(arr))
    cases:
      - compare: exact
        args:
          - [3, 1, 2]
        kwargs: {}
        expected: 2.0
YAML

cat > "$specs/lesson-2.yaml" <<'YAML'
title: Distributions and Standardization
slug: distributions-sampling
order: 2
sections:
  - title: Z-scores and percentiles
    snippets:
      - kind: markdown
        content: |
          ## Standardizing values

          A z-score states how many standard deviations a value sits above
          the mean. Percentiles locate a value within a sorted sample.
      - kind: code
        content: |
          import numpy as np
          scores = np.arange(1, 11)
          print(float(np.percentile(scores, 25)))
    practice:
      - name: z_score
        signature: "def z_score(value, mean, std):"
        prompt: Return the z-score rounded to 8 decimals as a float.
        solution: |
          def z_score(value, mean, std):
              return round((value - mean) / std, 8)
        cases:
          - compare: exact
            args:
              - 7
              - 5
              - 2
            kwargs: {}
            expected: 1.0
      - name: quartile
        signature: "def quartile(arr):"
        prompt: Return the 25th percentile of the array as a float.
        solution: |
          def quartile(arr):
              import numpy as np
              return float(np.percentile(arr, 25))
        cases:
          - compare: exact
            args:
              - [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            kwargs: {}
            expected: 3.25
quizzes:
  - name: standardize
    signature: "def standardize(arr):"
    prompt: Return the array standardized to z-scores, rounded to 8 decimals, as a nested list.
    solution: |
      def standardize(arr):
          import numpy as np
          return np.round((np.array(arr) - np.mean(arr)) / np.std(arr), 8).tolist()
    cases:
      - compare: exact
        args:
          - [1, 2, 3]
        kwargs: {}
        expected: [-1.22474487, 0.0, 1.22474487]
YAML

cat > "$specs/lesson-3.yaml" <<'YAML'
title: Correlation and Simple Regression
slug: correlation-regression
order: 3
sections:
  - title: Linear relationships
    snippets:
      - kind: markdown
        content: |
          ## Correlation and least squares

          The correlation coefficient measures linear association; a
          degree-1 polynomial fit recovers slope and intercept.
      - kind: code
        content: |
          import numpy as np
          x = np.array([0, 1, 2])
          y = np.array([0, 2, 4])
          print(float(np.corrcoef(x, y)[0, 1]))
          print(np.polyfit(x, y, 1))
    practice:
      - name: correlation
        signature: "def correlation(x, y):"
        prompt: Return the Pearson correlation coefficient as a float (round to 8 decimals).
        solution: |
          def correlation(x, y):
              import numpy as np
              return float(np.round(np.corrcoef(x, y)[0, 1], 8))
        cases:
          - compare: exact
            args:
              - [1, 2, 3]
              - [2, 4, 6]
            kwargs: {}
            expected: 1.0
      - name: slope
        signature: "def slope(x, y):"
        prompt: Return the least-squares slope as a float.
        solution: |
          def slope(x, y):
              import numpy as np
              return float(np.round(np.polyfit(x, y, 1)[0], 8))
        cases:
          - compare: exact
            args:
              - [0, 1, 2]
              - [0, 2, 4]
            kwargs: {}
            expected: 2.0
quizzes:
  - name: intercept
    signature: "def intercept(x, y):"
    prompt: Return the least-squares intercept as a float.
    solution: |
      def intercept(x, y):
          import numpy as np
          return float(np.round(np.polyfit(x, y, 1)[1], 8))
    cases:
      - compare: exact
        args:
          - [0, 1, 2]
          - [1, 3, 5]
        kwargs: {}
        expected: 1.0
YAML

cat > "$specs/plan.yaml" <<'YAML'
title: Intro Stats course plan
goals:
  - Compute descriptive statistics with NumPy
  - Standardize values and read percentiles
  - Fit simple linear relationships
links:
  goal_index_0:
    - descriptive-stats
  goal_index_1:
    - distributions-sampling
  goal_index_2:
    - correlation-regression
YAML

must_ok course c course create --spec "$specs/course.yaml"
must_ok venv c venv create --python 3.12
must_ok deps c venv add numpy

learner_edit() { # $1 = lesson dir, $2 = JSON {fn_name: solution body}
    python3 - "$root/courses/intro-stats/lessons/$1/lesson.ipynb" "$2" <<'PY'
import json
import re
import sys

nb = json.load(open(sys.argv[1]))
sols = json.loads(sys.argv[2])
for cell in nb["cells"]:
    managed = cell.get("metadata", {}).get("managed")
    if managed in ("quiz-stub", "practice-stub"):
        fn = re.match(r"def (\w+)\(", "".join(cell["source"])).group(1)
        cell["source"] = cell["source"][:-1] + sols[fn].splitlines(True)
json.dump(nb, open(sys.argv[1], "w"))
PY
}

study() { # $1 = lesson slug, $2 = spec
    env=$(must_ok "verify-$1" c lesson verify --spec "$specs/$2")
    assert_has "verify-$1" '"passing":3' "$env"
    must_ok "create-$1" c lesson create --spec "$specs/$2"
    env=$(must_ok "execute-$1" c lesson execute "$1" --allow-errors)
    assert_has "execute-$1" '"errored":0' "$env"
    env=$(must_ok "quiz-presolve-$1" c quiz run "$1")
    assert_has "quiz-presolve-$1" '"pass_or_fail":false' "$env"
    assert_has "quiz-presolve-$1" 'NotImplementedError' "$env"
}

solve() { # $1 = lesson slug, $2 = lesson dir, $3 = JSON stub solutions
    learner_edit "$2" "$3"
    env=$(must_ok "quiz-$1" c quiz run "$1")
    assert_has "quiz-$1" '"pass_or_fail":true' "$env"
    assert_has "quiz-$1" '"saved":true' "$env"
}

# One lesson at a time: fully finish a lesson before touching the next. The
# learner fills every stub (practice + quiz) with the spec-verified bodies.
study descriptive-stats lesson-1.yaml
solve descriptive-stats 01-descriptive-stats '{
"mean_value":   "    import numpy as np\n    return float(np.mean(arr))\n",
"std_value":    "    import numpy as np\n    return float(np.round(np.std(arr), 8))\n",
"median_value": "    import numpy as np\n    return float(np.median(arr))\n"
}'

study distributions-sampling lesson-2.yaml
solve distributions-sampling 02-distributions-sampling '{
"z_score":      "    return round((value - mean) / std, 8)\n",
"quartile":     "    import numpy as np\n    return float(np.percentile(arr, 25))\n",
"standardize":  "    import numpy as np\n    return np.round((np.array(arr) - np.mean(arr)) / np.std(arr), 8).tolist()\n"
}'

study correlation-regression lesson-3.yaml
solve correlation-regression 03-correlation-regression '{
"correlation":  "    import numpy as np\n    return float(np.round(np.corrcoef(x, y)[0, 1], 8))\n",
"slope":        "    import numpy as np\n    return float(np.round(np.polyfit(x, y, 1)[0], 8))\n",
"intercept":    "    import numpy as np\n    return float(np.round(np.polyfit(x, y, 1)[1], 8))\n"
}'

must_ok plan c plan create --scope course --spec "$specs/plan.yaml"
env=$(must_ok confirm c plan confirm pl1)
assert_has goals '"goals_created":["g1","g2","g3"]' "$env"

env=$(must_ok progress c progress summary)
assert_has lessons-complete '"complete":3' "$env"
assert_has quizzes-passing '"passing":3' "$env"
assert_has goals-achieved '"achieved":3' "$env"

echo "study-chain: ok"
