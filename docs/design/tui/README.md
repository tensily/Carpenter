# TUI wireframe

Standalone, opencode-independent wireframe of carpenter course authoring, at
`docs/design/tui/`. This README is the contract every implementation unit builds
against — the source of truth for tokens, classes, ids, and the mock store.
Doc-only: no Rust/CLI surface.

```
docs/design/tui/
  README.md             this contract
  index.html            DOM: six screens, bound ids, asset include order
  css/tokens.css        CSS custom properties (design tokens)
  css/layout.css        frame grid + header/main/keybar + screens
  css/components.css    component classes and their states
  js/mock.js            store: seed data + mutating methods (no document access)
  js/app.js             hash router + DOM bindings
```

## Purpose

A clickable wireframe for the course-authoring flow, usable without carpenter
installed and without opencode: open `index.html` via `file://` in any browser.
No build step, no dependencies, no network loads (no CDN fonts/scripts) —
vanilla HTML/CSS/JS only. Light-minimal theme, monospace-flavored.

Design stance — mirrors the CLI: carpenter has **no embedded LLM**.

- The TUI collects intake/spec via forms; it never generates content. Every
  action represents a deterministic CLI run:

  | TUI action | stands for |
  |---|---|
  | new course (intake submit) | `course create` |
  | review confirm | `plan create` → `plan confirm` |
  | lesson create / run | `lesson create` / `lesson execute` |
  | add note | `notes add` |
  | pivot confirm | `plan update` (+ re-confirm) |

- Viewing lessons stays in **VS Code** (Jupyter notebooks) — the lesson screen
  shows an open-in-VS-Code hint; the wireframe never renders notebook content.
- Feedback uses the note kinds `gap` / `mistake` / `strength` / `pattern` /
  `progress` (same taxonomy as `notes add`).
- Pivot = **human-driven** plan revision + re-confirm: goal edits, relinks, and
  add/removes are applied explicitly and shown as a diff before confirming;
  nothing is auto-suggested.

Status: wireframe — mock data only, no backend. Nothing calls the CLI;
`js/mock.js` stands in for it.

## Flow

1. intake — outcome / background / constraints (3-step wizard) → draft spec.
2. review — spec preview → confirm → scaffold: plan + lessons appear on the board.
3. board — plan status, goals + lesson links, lesson outline, progress rollup.
4. study in VS Code — open the lesson notebook; the TUI only hints and logs.
5. feedback — notes (`gap`/`mistake`/`strength`/`pattern`/`progress`) on the lesson screen.
6. pivot loop — revise goals → diff → confirm revised plan → back to board.

## Screens

Six SPA screens, one `<section class="c-screen">` each, hash-routed (default
`#/home`):

| screen | route | shows | binds (ids) |
|---|---|---|---|
| home | `#/home` | course list, new course | — |
| intake | `#/intake` | 3-step wizard: outcome / background / constraints | `#f-outcome` `#f-background` `#f-time` `#f-pace` `#f-depth` |
| review | `#/review` | spec preview → confirm | `#spec-preview` (+ modal) |
| board | `#/board` | plan status, goals + lesson links, lesson outline, progress rollup | `#plan-status` `#goals-list` `#lesson-list` `#progress-rollup` |
| lesson | `#/lesson` | metadata, actions, open-in-VS-Code hint, feedback log | `#lesson-title` `#lesson-actions` `#notes-list` `#note-kind` `#note-text` |
| pivot | `#/pivot` | goal edits, diff of changes, confirm revised plan | `#pivot-goals` `#pivot-diff` |

Global chrome on every screen: header, keybar, `#toast`, `#modal`
(`#modal-title` `#modal-body` `#modal-confirm`).

## Component inventory

The single listing of classes and states lives in the Contract section below
(tokens, layout classes, component classes). Every component there must exist in
`css/components.css` with all listed states.

## Contract

Reproduced verbatim. The six implementation units (index, tokens, layout,
components, mock store, app) work against this contract alone and must not
drift from it.

```text
Purpose: standalone, opencode-independent TUI wireframe for carpenter course authoring. Opens via file://, no build step, no dependencies, no network loads (no CDN fonts/scripts), vanilla HTML/CSS/JS only. Light-minimal theme, monospace-flavored. The wireframe is fully interactive only when all six units land; each unit works against this contract alone.

CSS custom properties (tokens.css, :root) — exact names:
--bg --surface --ink --muted --line --accent --accent-ink --ok --warn --err
--radius --radius-sm
--space-1 --space-2 --space-3 --space-4 --space-5 --space-6
--font-sans --font-mono
--text-sm --text-md --text-lg

Layout classes (layout.css): .c-frame (body-level grid: header/main/footer rows), .c-header, .c-main, .c-keybar, .c-screen (one per screen; hidden via [hidden] { display:none }).

Component classes (components.css) — every one must exist with its states:
c-panel (+ __head, __title, __body); c-btn (+ --primary --ghost --danger; :hover :focus-visible [disabled]); c-kbd; c-field (label + control wrapper); c-input; c-textarea; c-chips > c-chip (+ --on); c-stepper > c-step (+ --active --done); c-list > c-row (+ --selected --clickable; :hover :focus-visible); c-badge (+ --planned --created --verified --skipped --draft --confirmed --gap --mistake --strength --pattern --progress); c-progress (+ __fill); c-empty; c-note; c-diff-row (+ --add --del --mod); c-modal (overlay shown via [open]; inner .c-modal__box); c-toast (+ --ok --err); c-actions (button row); c-kv (key/value spec-preview rows).

DOM contract (index.html):
- body.c-frame; header.c-header; main.c-main; footer.c-keybar with c-kbd hints.
- Six sections: <section class="c-screen" data-screen="home|intake|review|board|lesson|pivot">.
- Hash routes #/home #/intake #/review #/board #/lesson #/pivot (default #/home). Nav triggers carry data-nav="<screen>", actions data-action="<name>", wizard movement data-step="prev|next"; selectable rows carry data-select="course|lesson".
- Element ids app.js binds (exact): #toast #modal #modal-title #modal-body #modal-confirm #f-outcome #f-background #f-time #f-pace #f-depth #spec-preview #plan-status #goals-list #lesson-list #progress-rollup #lesson-title #lesson-actions #notes-list #note-kind #note-text #pivot-diff #pivot-goals
- index.html includes, in order: css/tokens.css, css/layout.css, css/components.css, then <script src="js/mock.js" defer></script>, <script src="js/app.js" defer></script>.
- Each screen ships realistic pre-rendered static markup so the wireframe reads before JS enhances it.

Store contract (js/mock.js): assigns const store and window.store; no top-level document access (so `node -e "window={};require('./js/mock.js')"` works).
Shape: { courses: [{slug,title,path,lessonCount,progressPct}], activeCourse, plan: {status:'draft'|'confirmed', goals:[{id,text,links:[lessonId]}]}, lessons: [{id,slug,title,status:'planned'|'created'|'verified'|'skipped'}], notes: [{id,lessonId,kind,text,at}], draftSpec: {title,slug,goal,description}, log: [{cmd,ok}] }
Methods (mutate, append to log, return affected value): addNote({lessonId,kind,text}), setLessonStatus(id,status), confirmPlan(), applyPivot({goalEdits,goalsAdded,goalsRemoved,relinks,lessonsAdded,lessonsRemoved}); nextId(prefix, items) is pure — next free `prefix<N>` id.
Seed: courses linalg-for-ml (active, 9 lessons, 22%) + python-foundations (80%); plan confirmed with 4 goals — g1 vectors/matrices links [vectors-refresher, matrices-refresher]; g2 systems/LU links [systems-and-elimination, lu-decomposition]; g3 eigen/SVD links [determinants-and-eigenvectors, svd]; g4 ML applications links [ml-linear-regression, ml-pca, ml-neural-network]; lessons vectors-refresher (verified), matrices-refresher (created), systems-and-elimination (created), lu-decomposition / determinants-and-eigenvectors / svd / ml-linear-regression / ml-pca / ml-neural-network (planned); 2–3 notes across kinds; draftSpec {title "Computational Linear Algebra for ML", slug linalg-for-ml, goal "Build ML-ready linear-algebra intuition from vectors to SVD", description "Vectors, matrices, systems, decompositions, and ML applications."}.
```

## How to open

Open `docs/design/tui/index.html` in a browser — `file://` works, no server and
no build step needed. The wireframe is fully interactive only when all six
units are merged (index, tokens, layout, components, mock store, app); until
then, every screen still reads via its pre-rendered static markup.
