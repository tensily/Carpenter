**example:**

```sh
carpenter -c ds lesson create --spec lesson.yaml
```

Input spec (`--spec <FILE|->`):
```yaml
title: Arrays 101
slug: arrays-101
order: 1                  # optional; omitted ⇒ appended (max(ord)+1, atomic)
sections:
  - title: What is an array
    snippets:
      - kind: markdown
        content: An array stores items contiguously…
      - kind: code
        content: |
          import numpy as np
          np.array([1, 2, 3])
    practice:
      - name: sum_array
        signature: "def sum_array(arr):"
        prompt: Return the sum of the array.
        cases:
          - compare: exact
            args:
              - [1, 2, 3]
            kwargs: {}
            expected: 6
quizzes:
  - name: max_value
    signature: "def max_value(arr):"
    prompt: Return the max.
    cases:
      - compare: exact
        args:
          - [3, 1, 2]
        kwargs: {}
        expected: 3
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"lesson created: arrays-101","data":{"id":"arrays-101","slug":"arrays-101","path":"<root>/courses/<slug>/lessons/01-arrays-101","counts":{"sections":1,"practice":1,"quizzes":1,"cases":2}}}
```

`sections[].snippets[0].kind` must be `markdown`. Array index ⇒ `ord`. `compare` defaults to `exact`. A provided `slug` must be kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`, max 60 — adr/017); a derived slug always is. A duplicate provided `slug` is an `AlreadyExists` error — provided slugs are never uniquified; only derived slugs (slug omitted) dedup on collision (`<slug>-2`, `<slug>-3`, …). Omit `order` to append atomically (`max(ord)+1` inside the insert statement, unique by index); an explicit `order` colliding with an existing lesson's is a `Conflict` (`"lesson ord N already taken"`). Concurrent creates are safe; for a fixed sequence prefer explicit `order`s or create sequentially.
