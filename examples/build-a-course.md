# Build a course end-to-end

A canonical agent workflow: scaffold a course, author one lesson, set goals and
link them to covering lessons, verify the lesson runs, score the fresh
notebook, and roll up progress. Running example: a computational-linear-algebra
course (`linalg-for-ml`). Repeat the `lesson create` step for each lesson in the outline.

The fenced ` ```sh ` blocks below are the real flow; the ` ```yaml ` blocks are
the specs and the ` ```json ` blocks are the result envelopes. (Only the `sh`
blocks are counted by the compile-time scenario gate — see
`docs/adr/013-compile-enforced-scenarios.md`.)

## 0. Interview the learner (intake)

No carpenter command yet. Ask the user — the course spec in step 1 and the plan
goals in step 4 are the *output* of this interview, not a guess (the skill's
`## Learner intake` section gates `course create` on the user's agreement):

- **What do you want to learn, and why?** — target outcome (project, role,
  course, curiosity) and what "done" looks like.
- **Background** — languages known, math/tools experience, prior exposure to
  the topic; what they know well enough to skip.
- **Constraints** — time budget, pace, appetite for lesson count and depth.

The running example assumes the answers: knows Python basics, needs ML-ready
linear algebra for a data-role move, has ~6 weeks — which is exactly what the
`goal:` line below encodes.

## 1. Scaffold the course

```sh
carpenter course create --spec <course-spec>.yaml
```

`<course-spec>.yaml`:
```yaml
title: Computational Linear Algebra for ML
slug: linalg-for-ml
goal: Build ML-ready linear-algebra intuition from vectors to SVD
description: Vectors, matrices, systems, decompositions, and ML applications.
```
```json
{"status":"ok","message":"course created: linalg-for-ml","data":{"slug":"linalg-for-ml","title":"Computational Linear Algebra for ML","path":"<root>/courses/linalg-for-ml"}}
```

## 2. Create the venv and add deps

Required before `lesson execute` / `quiz run` (uses `uv`).

```sh
carpenter -c linalg-for-ml venv create --python 3.12
carpenter -c linalg-for-ml venv add numpy
```

## 3. Author a lesson (renders notebook + verification-only helper)

```sh
carpenter -c linalg-for-ml lesson create --spec <lesson-spec>.yaml
```

`<lesson-spec>.yaml` — a minimal one-section lesson (the full spec shape lives at
`docs/examples/lesson/create.md`):
```yaml
title: "Vectors: A Computational Refresher"
slug: vectors-refresher
sections:
  - title: The dot product
    snippets:
      - kind: markdown
        content: |
          ## The dot product

          $$\vec{x}\cdot\vec{y} = \sum_i x_i y_i$$
      - kind: code
        content: |
          import numpy as np
          print(np.dot([1,2,3],[4,5,6]))
    practice:
      - name: dot_product
        signature: "def dot_product(a, b):"
        prompt: Return the dot product of a and b as a Python float (use np.dot, then float()).
        cases:
          - compare: exact
            args:
              - [1, 2, 3]
              - [4, 5, 6]
            kwargs: {}
            expected: 32
quizzes:
  - name: dot_sign
    signature: "def dot_sign(a, b):"
    prompt: Return 1 if dot(a,b)>0, -1 if <0, 0 if ==0.
    cases:
      - compare: exact
        args:
          - [1, 2]
          - [3, 4]
        kwargs: {}
        expected: 1
```
```json
{"status":"ok","message":"lesson created: vectors-refresher","data":{"id":"vectors-refresher","slug":"vectors-refresher","path":"<root>/courses/linalg-for-ml/lessons/01-vectors-refresher","counts":{"sections":1,"practice":1,"quizzes":1,"cases":2}}}
```

## 4. Set goals and link covering lessons

`links` must reference lesson ids that **already exist** — confirming a plan
whose links name unknown lessons fails with
`{"status":"error","code":"ValidationError","message":"validation error: unresolvable lesson id in links: <id>"}`,
so create the lessons first (step 3).

```sh
carpenter -c linalg-for-ml plan create --scope course --spec <plan-spec>.yaml
carpenter -c linalg-for-ml plan confirm pl1
```

`<plan-spec>.yaml` (`links` keys MUST be `goal_index_<i>` — the 0-based index into `goals[]`):
```yaml
title: "Computational Linear Algebra for ML — Learning Goals"
goals:
  - Represent vectors and matrices in NumPy and compute products.
  - Solve linear systems via elimination and LU.
  - Apply eigenvalues, eigenvectors, and the SVD.
  - Apply linear algebra to ML: regression, PCA, nets, classification.
links:
  goal_index_0: [vectors-refresher, matrices-refresher]
  goal_index_1: [systems-and-elimination, lu-decomposition]
  goal_index_2: [determinants-and-eigenvectors, svd]
  goal_index_3: [ml-linear-regression, ml-pca, ml-neural-network]
```
```json
{"status":"ok","message":"plan confirmed: pl1","data":{"id":"pl1","confirmed":true,"confirmed_at":"2026-08-09T12:00:00Z","goals_created":["g1","g2","g3","g4"]}}
```

## 5. Verify the teaching cells run clean

Stubs only `raise` at call time, so `--allow-errors` reports `errored:0` on a
freshly rendered notebook (practice/quiz cells define functions; they don't
raise at definition time).

```sh
carpenter -c linalg-for-ml lesson execute vectors-refresher --allow-errors
```
```json
{"status":"ok","message":"lesson executed: vectors-refresher","data":{"id":"vectors-refresher","executed":true,"cells":{"total":7,"ran":7,"errored":0},"errors":[]}}
```

## 6. Score the fresh notebook

Empty stubs → every quiz fails with `NotImplementedError` — the expected fresh
state. The helper catches the exception and records `pass_or_fail:false`; no cell
error is raised.

```sh
carpenter -c linalg-for-ml quiz run vectors-refresher
```
```json
{"status":"ok","message":"quizzes run: vectors-refresher","data":{"lesson_id":"vectors-refresher","quizzes":[{"quiz_id":"q1","skipped":false,"pass_or_fail":false,"passed":0,"total":1,"cases":[{"case_id":"c1","passed":false,"error":"NotImplementedError: "}]}],"saved":true}}
```

## 7. Roll up progress

```sh
carpenter -c linalg-for-ml progress summary
```
```json
{"status":"ok","message":"progress summarized","data":{"lessons":{"total":1,"complete":0,"in_progress":0,"skipped":0},"quizzes":{"passing":0,"total":1},"goals":{"total":4,"achieved":0},"notes":{"total":0,"open":0,"recurring":0,"by_kind":{"gap":0,"mistake":0,"strength":0,"pattern":0,"progress":0}}}}
```

## Conventions (agent discipline)

These are not `carpenter` commands, but they are the verification loop the agent
should run while authoring each lesson:

- **Lock the answer key before trusting `quiz run`.** Put a `solution` (Python
  defining the fn `name`) on each practice/quiz and run
  `carpenter -c linalg-for-ml lesson verify --spec <lesson-spec>.yaml` — it runs
  each solution against its own cases with the same compare logic that grades the
  learner ([adr/015](../docs/adr/015-reference-solution-verify.md)).
  `np.linalg.solve` with integer solutions is bit-exact; `np.linalg.inv` /
  `eig` / `svd` are not.
- **Round floats to 8 decimals.** Outputs of `inv` / `eig` / `svd` / `lstsq`
  carry ~1e-16 noise. Have the learner return `np.round(result, 8).tolist()` and
  store expected values pre-rounded. Integer-valued cases need no rounding.
- **Design sign-/ambiguity-free cases.** Grade on ranks, singular values,
  variances, traces, determinants, and integer-valued solutions — not on
  eigenvectors (sign ambiguity) or raw `inv` output.
- **Never hand-edit a rendered `lesson.ipynb`.** Regenerate only via
  `lesson create` / `lesson update` / `lesson sync`.
