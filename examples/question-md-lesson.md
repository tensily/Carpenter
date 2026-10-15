# Prompt cells and stub scaffolds, end to end

How authored prompts reach the learner: every practice/quiz with a non-empty
prompt renders a managed markdown cell (`managed=question-md` + the owning
`practice_id`/`quiz_id`) directly above its stub, carrying the full prompt
verbatim — markdown allowed, so `$$` math and backticks live there. The stub
itself comments every prompt line (blank lines become a bare `#`) so the
scaffold stays valid Python; an empty prompt renders no question cell and no
comment at all. On sync the question cells regenerate wholesale while
learner-filled stubs survive. Running example: a `stats` course (assumed to
exist with its venv from here on; `build-a-course.md` scaffolds both), lesson
`descriptive-stats`.

The fenced ` ```sh ` blocks below are the real flow; the ` ```yaml ` blocks are
the specs and the ` ```json ` blocks are the result envelopes. (Only the `sh`
blocks are counted by the compile-time scenario gate — see
`docs/adr/013-compile-enforced-scenarios.md`.)

## 1. Author multi-line prompts (renders question cells + commented scaffolds)

```sh
carpenter -c stats lesson create --spec <lesson-spec>.yaml
```

`<lesson-spec>.yaml` — both checkable prompts are multi-line; the quiz's
carries a `$$` math line:
```yaml
title: "Center and spread"
slug: descriptive-stats
sections:
  - title: Mean and standard deviation
    snippets:
      - kind: markdown
        content: |
          ## Mean and standard deviation

          Two numbers summarize any list of measurements: where it is
          centered (the mean) and how wide it scatters (the standard
          deviation). Every benchmark, poll, and model residual reduces
          to this pair.
      - kind: code
        content: |
          xs = [2, 4, 4, 4, 5, 5, 7, 9]
          print(sum(xs) / len(xs))
    practice:
      - name: mean_and_spread
        signature: "def mean_and_spread(xs):"
        prompt: |
          Given a non-empty list of numbers `xs`, return `[mean, spread]`.

          The **mean** is the arithmetic average; the **spread** is the
          population standard deviation — compute both by hand:

          $$\bar{x} = \frac{1}{n}\sum_i x_i \qquad \sigma = \sqrt{\frac{1}{n}\sum_i (x_i - \bar{x})^2}$$
        cases:
          - compare: exact
            args:
              - [2, 4, 4, 4, 5, 5, 7, 9]
            kwargs: {}
            expected: [5.0, 2.0]
      - name: range_only
        signature: "def range_only(xs):"
        prompt: ""
        cases:
          - compare: exact
            args:
              - [3, 1, 2]
            kwargs: {}
            expected: 2
quizzes:
  - name: z_score
    signature: "def z_score(x, mu, sigma):"
    prompt: |
      Return the z-score of `x` under $N(\mu, \sigma^2)$ — how many
      standard deviations `x` sits from the mean:

      $$z = \frac{x - \mu}{\sigma}$$
    cases:
      - compare: exact
        args:
          - 70
          - 60
          - 10
        kwargs: {}
        expected: 1.0
```
```json
{"status":"ok","message":"lesson created: descriptive-stats","data":{"id":"descriptive-stats","slug":"descriptive-stats","path":"<root>/courses/stats/lessons/01-descriptive-stats","counts":{"sections":1,"practice":2,"quizzes":1,"cases":3}}}
```

Each checkable with a prompt renders as a `question-md` → stub → check triple;
the empty-prompt one as stub → check only:

| rendered cells | spec source | contract pinned |
|---|---|---|
| `question-md` → `practice-stub` → `check` | `practice[0]` | full prompt verbatim above the stub; every prompt line commented inside it |
| `practice-stub` → `check` | `practice[1]` (`prompt: ""`) | empty prompt ⇒ no question cell, no comment line |
| `question-md` → `quiz-stub` → `check` | `quizzes[0]` | quizzes get the same treatment |

The `mean_and_spread` pair, rendered. The question cell is the prompt
verbatim:

```markdown
Given a non-empty list of numbers `xs`, return `[mean, spread]`.

The **mean** is the arithmetic average; the **spread** is the
population standard deviation — compute both by hand:

$$\bar{x} = \frac{1}{n}\sum_i x_i \qquad \sigma = \sqrt{\frac{1}{n}\sum_i (x_i - \bar{x})^2}$$
```

…directly above a stub that comments the same lines (blank → bare `#`), so
the cell compiles and the full text stays in the markdown above:

```python
def mean_and_spread(xs):
    # Given a non-empty list of numbers `xs`, return `[mean, spread]`.
    #
    # The **mean** is the arithmetic average; the **spread** is the
    # population standard deviation — compute both by hand:
    #
    # $$\bar{x} = \frac{1}{n}\sum_i x_i \qquad \sigma = \sqrt{\frac{1}{n}\sum_i (x_i - \bar{x})^2}$$
    raise NotImplementedError
```

The empty-prompt `range_only` gets neither a question cell nor a comment:

```python
def range_only(xs):
    raise NotImplementedError
```

## 2. Verify the fresh notebook runs clean

Multi-line scaffolds are valid Python — nothing lands at column 0:

```sh
carpenter -c stats lesson execute descriptive-stats --allow-errors
```
```json
{"status":"ok","message":"lesson executed: descriptive-stats","data":{"id":"descriptive-stats","executed":true,"cells":{"total":8,"ran":8,"errored":0},"errors":[]}}
```

`errored:0` on a multi-line-prompt lesson is the point: before
`canonical_scaffold` commented line-by-line, a prompt's second line landed at
column 0 inside the stub and fresh runs failed there with `IndentationError`
(strict mode surfaces it as `ExecuteError`).

## 3. Score the fresh stubs

Stubs only `raise` at call time — the helper catches it and scores a fail
(never printing an expected value):

```sh
carpenter -c stats quiz run descriptive-stats
```
```json
{"status":"ok","message":"quizzes run: descriptive-stats","data":{"lesson_id":"descriptive-stats","quizzes":[{"quiz_id":"q1","skipped":false,"pass_or_fail":false,"passed":0,"total":1,"cases":[{"case_id":"c3","passed":false,"error":"NotImplementedError: "}]}],"saved":true}}
```

## 4. Hand-edit, then sync (question cells regenerate; learner work survives)

The learner solves the practice — and mangles the question cell right above
it. Two edits, one cell pair; ownership decides which survives:

```python
# practice-stub (p1) — learner work, must survive:
def mean_and_spread(xs):
    n = len(xs)
    mean = sum(xs) / n
    var = sum((x - mean) ** 2 for x in xs) / n
    return [mean, var ** 0.5]

# question-md (p1), same pair — defaced:
DEFACED — this question cell was hand-edited out of shape.
```

```sh
carpenter -c stats lesson sync descriptive-stats
```
```json
{"status":"ok","message":"lesson synced: descriptive-stats","data":{"id":"descriptive-stats","synced":true,"conflicts":[]}}
```

`conflicts:[]` and both outcomes land: the stub is learner-edited while the
DB is unchanged, so the 3-way merge keeps it verbatim (a conflict needs
*both* a learner edit and a DB change); the question cell is a managed
non-stub — regenerated wholesale from the DB prompt, so the defaced cell is
the full verbatim prompt again, math included. `range_only` still renders no
question cell. The DB stays source of truth for every managed cell; the
learner owns filled stubs and their own untagged cells
([design/05](../docs/design/05-notebook-sync.md)).
