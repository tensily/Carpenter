# Author a pedagogically complete lesson

A lesson that teaches itself: every notebook carpenter renders should be
self-contained — it opens with a story that motivates the concept, defines the
concept from scratch, teaches the mechanics, and closes with the moral — before
any practice or quiz appears. This scenario authors one such lesson end-to-end:
lock the answer key, create the lesson, and verify the notebook runs clean.
Running example: a convolution lesson in the `signals` course (assumed to exist
with its venv from here on; `build-a-course.md` scaffolds both).

The fenced ` ```sh ` blocks below are the real flow; the ` ```yaml ` blocks are
the specs and the ` ```json ` blocks are the result envelopes. (Only the `sh`
blocks are counted by the compile-time scenario gate — see
`docs/adr/013-compile-enforced-scenarios.md`.)

## The lesson anatomy (markdown → spec)

Every rendered markdown cell is a `sections[].snippets[]` entry with
`kind: markdown`; runnable cells are `kind: code`. Author the snippets in this
canonical order:

| notebook cell | spec location | what it carries |
|---|---|---|
| opening story | first section, markdown snippet | a concrete vignette ending in the question the lesson answers |
| the concept | second section, markdown snippet | the lesson's building block defined from scratch: **what it is** (minimal formula, every symbol named), **how it works** (a worked example in real numbers), **where it's used** (2–4 domains, one outside the course's home field), **why it's a building block** (what later lessons stand on it) |
| teaching sections | following sections | mechanics, formulas, pitfalls — each closes with a `**Think:**` prompt before its practice (guess before you compute) |
| the moral | last section, markdown snippet | 2–3 sentences tying story → concept; never reveals a solution |

Discipline:

- **Self-containment** — the lesson may not assume an external lecture: every
  building block and every invoked tool (`np.convolve`, `signal.lfilter`, …)
  gets a plain-language line at point of use.
- **Guide-don't-give** — teaching prose never states a practice/quiz answer.
  Worked examples use *analogous* numbers, not the graded ones; the answer key
  lives only in the `solution:` fields, which `lesson verify` locks and the
  rendered notebook never shows.
- **One concept per lesson** — story, concept, practice, and quiz all point at
  the same building block.

## 1. Create the venv (once per course)

Required before `lesson verify` / `lesson execute` (uses `uv`).

```sh
carpenter -c signals venv create --python 3.12
```

## 2. Lock the answer key before creating

```sh
carpenter -c signals lesson verify --spec <lesson-spec>.yaml
```

`<lesson-spec>.yaml`:
```yaml
title: "Convolution: the system's fingerprint"
slug: convolution-basics
sections:
  - title: The canyon always answers the same way
    snippets:
      - kind: markdown
        content: |
          Yodel into a canyon — or clap in a stairwell — and the wall replies
          with the same echo pattern every time: shout louder and it scales,
          shout later and it shifts. The echo pattern is the canyon's
          fingerprint, and everything you ever hear back is your voice stamped
          with it.

          An LTI system is that canyon. This lesson is about the stamping
          operation — convolution — and the one recording that predicts every
          reply: the impulse response.
  - title: What convolution is
    snippets:
      - kind: markdown
        content: |
          **What it is —** Convolution answers: "a burst that happened *then*
          is still ringing *now* — by how much?" For each output time n, flip
          the impulse response h, slide it to n, multiply with the input x
          point by point, and sum the overlaps:

          $$y[n] = \sum_k x[k]\,h[n-k]$$

          **How it works —** x = [1, 2], h = [1, 1]: y[0] = 1·1 = 1;
          y[1] = 2·1 + 1·1 = 3; y[2] = 2·1 = 2. So [1, 2] ∗ [1, 1] = [1, 3, 2]
          — the input, smeared over three samples instead of two.

          **Where it's used —** room reverb (each wall reflection is a tap of
          h); image blur and sharpening (the same sum in two dimensions); the
          probability of a sum of two independent quantities; moving averages
          in economics; the convolution layers of machine vision.

          **Why it's a building block —** h *is* the entire system: measure it
          once and convolution predicts the output to every possible input,
          forever. And in the frequency domain convolution becomes plain
          multiplication — the fact that makes filters designable at all.
  - title: Flip, slide, sum
    snippets:
      - kind: markdown
        content: |
          The sum above is mechanical: flip h, slide it to position n,
          multiply the overlaps, add them. An input of length N convolved with
          a response of length M smears into a longer output — **Think:**
          before the practice below, at how many positions can at least one
          product land? Sketch the overlaps for N = 4, M = 3 in the margin
          first, then let the cell confirm you.
      - kind: code
        content: |
          import numpy as np
          np.convolve([1, 2, 3, 4], [1, 1, 1])
  - title: The moral of the story
    snippets:
      - kind: markdown
        content: |
          Every room you have ever clapped in was handing you its impulse
          response. Once you can read h, a filter, a phone line, and a
          cathedral are the same homework problem: convolve and find out.
    practice:
      - name: conv_length
        signature: "def conv_length(n, m):"
        prompt: Return the length of the full convolution of a length-n signal with a length-m response, as an int.
        solution: |
          def conv_length(n, m):
              return n + m - 1
        cases:
          - compare: exact
            args:
              - 4
              - 3
            kwargs: {}
            expected: 6
          - compare: exact
            args:
              - 1
              - 1
            kwargs: {}
            expected: 1
quizzes:
  - name: echo_taps
    signature: "def echo_taps(fs, delay_s):"
    prompt: An echo delays the input by delay_s seconds. Return how many samples that is at sample rate fs, as an int (round half to even is fine).
    solution: |
      def echo_taps(fs, delay_s):
          return round(fs * delay_s)
    cases:
      - compare: exact
        args:
          - 44100
          - 0.1
        kwargs: {}
        expected: 4410
```
```json
{"status":"ok","message":"lesson verified: (spec)","data":{"lesson_id":null,"checked":2,"passing":2,"failing":0,"checkables":[{"owner_type":"practice","owner_id":"conv_length","name":"conv_length","has_solution":true,"passed":2,"total":2,"cases":[{"case_id":"conv_length-0","passed":true},{"case_id":"conv_length-1","passed":true}]},{"owner_type":"quiz","owner_id":"echo_taps","name":"echo_taps","has_solution":true,"passed":1,"total":1,"cases":[{"case_id":"echo_taps-0","passed":true}]}]}}
```

Each `solution` ran against its own cases in the course venv — the key is
proven correct *before* any learner sees the lesson
([adr/015](../docs/adr/015-reference-solution-verify.md)). Note the
guide-don't-give discipline in the spec itself: the prose works an analogous
example ([1, 2] ∗ [1, 1]) and asks the learner to sketch the N = 4, M = 3 case,
but never states the length formula the practice grades.

## 3. Create the lesson (renders notebook + verification-only helper)

```sh
carpenter -c signals lesson create --spec <lesson-spec>.yaml
```
```json
{"status":"ok","message":"lesson created: convolution-basics","data":{"id":"convolution-basics","slug":"convolution-basics","path":"<root>/courses/signals/lessons/01-convolution-basics","counts":{"sections":4,"practice":1,"quizzes":1,"cases":3}}}
```

## 4. Verify the notebook runs clean

```sh
carpenter -c signals lesson execute convolution-basics --allow-errors
```
```json
{"status":"ok","message":"lesson executed: convolution-basics","data":{"id":"convolution-basics","executed":true,"cells":{"total":8,"ran":8,"errored":0},"errors":[]}}
```

Fresh stubs only `raise` at call time, so `errored:0` is the expected state —
practice/quiz cells define functions; they don't raise at definition time. The
check cells printed the real contract (`PASS`/`FAIL <id> <case>` + a `k/n`
summary, never an expected value) when the learner fills the stubs.
