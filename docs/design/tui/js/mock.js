'use strict';

// Mock data store for the carpenter TUI wireframe (docs/design/tui).
// The single source app.js renders from: plain script, no imports, no DOM
// access at load time (loads under `node` with a stubbed `window`), no
// fetch/timers/randomness. Every method mutates state, appends a realistic
// CLI command to `log`, and returns the affected value.

const lessonCmds = {
  verified: (id) => `lesson execute ${id}`,
  created: (id) => `lesson new ${id}`,
  skipped: (id) => `skip ${id} --scope lesson`,
};

const store = {
  courses: [
    {
      slug: 'linalg-for-ml',
      title: 'Computational Linear Algebra for ML',
      path: '~/courses/linalg-for-ml',
      lessonCount: 9,
      progressPct: 22,
    },
    {
      slug: 'python-foundations',
      title: 'Python Foundations',
      path: '~/courses/python-foundations',
      lessonCount: 5,
      progressPct: 80,
    },
  ],

  activeCourse: 'linalg-for-ml',

  plan: {
    status: 'confirmed',
    goals: [
      {
        id: 'g1',
        text: 'Represent vectors and matrices in NumPy and compute products.',
        links: ['vectors-refresher', 'matrices-refresher'],
      },
      {
        id: 'g2',
        text: 'Solve linear systems via elimination and LU.',
        links: ['systems-and-elimination', 'lu-decomposition'],
      },
      {
        id: 'g3',
        text: 'Apply eigenvalues, eigenvectors, and the SVD.',
        links: ['determinants-and-eigenvectors', 'svd'],
      },
      {
        id: 'g4',
        text: 'Apply linear algebra to ML: regression, PCA, nets, classification.',
        links: ['ml-linear-regression', 'ml-pca', 'ml-neural-network'],
      },
    ],
  },

  lessons: [
    { id: 'vectors-refresher', slug: 'vectors-refresher', title: 'Vectors: A Computational Refresher', status: 'verified' },
    { id: 'matrices-refresher', slug: 'matrices-refresher', title: 'Matrices in NumPy', status: 'created' },
    { id: 'systems-and-elimination', slug: 'systems-and-elimination', title: 'Systems and Elimination', status: 'created' },
    { id: 'lu-decomposition', slug: 'lu-decomposition', title: 'LU Decomposition', status: 'planned' },
    { id: 'determinants-and-eigenvectors', slug: 'determinants-and-eigenvectors', title: 'Determinants and Eigenvectors', status: 'planned' },
    { id: 'svd', slug: 'svd', title: 'Singular Value Decomposition', status: 'planned' },
    { id: 'ml-linear-regression', slug: 'ml-linear-regression', title: 'Linear Regression', status: 'planned' },
    { id: 'ml-pca', slug: 'ml-pca', title: 'PCA', status: 'planned' },
    { id: 'ml-neural-network', slug: 'ml-neural-network', title: 'Neural Networks', status: 'planned' },
  ],

  notes: [
    {
      id: 'n1',
      lessonId: 'vectors-refresher',
      kind: 'gap',
      text: 'Confused about row vs column vectors in the dot-product section.',
      at: '2026-10-14T09:30:00Z',
    },
    {
      id: 'n2',
      lessonId: 'matrices-refresher',
      kind: 'strength',
      text: 'Broadcasting rules finally clicked during the practice sets.',
      at: '2026-10-14T16:05:00Z',
    },
  ],

  draftSpec: {
    title: 'Computational Linear Algebra for ML',
    slug: 'linalg-for-ml',
    goal: 'Build ML-ready linear-algebra intuition from vectors to SVD',
    description: 'Vectors, matrices, systems, decompositions, and ML applications.',
  },

  log: [],

  addNote({ lessonId, kind, text }) {
    const note = { id: store.nextId('n', store.notes), lessonId, kind, text, at: new Date().toISOString() };
    store.notes.push(note);
    store.log.push({ cmd: `notes add --kind ${kind} ${lessonId}`, ok: true });
    return note;
  },

  nextId(prefix, items) {
    let maxSeq = 0;
    for (const item of items) {
      const match = new RegExp(`^${prefix}(\\d+)$`).exec(item.id);
      if (match) maxSeq = Math.max(maxSeq, Number(match[1]));
    }
    return prefix + (maxSeq + 1);
  },

  setLessonStatus(id, status) {
    const lesson = store.lessons.find((entry) => entry.id === id);
    const cmd = lessonCmds[status] ? lessonCmds[status](id) : `lesson update ${id}`;
    store.log.push({ cmd, ok: !!lesson });
    if (!lesson) return null;
    lesson.status = status;
    return lesson;
  },

  confirmPlan() {
    store.plan.status = 'confirmed';
    store.log.push({ cmd: 'plan confirm pl1', ok: true });
    return store.plan;
  },

  applyPivot({ goalEdits, goalsAdded, goalsRemoved, relinks, lessonsAdded, lessonsRemoved } = {}) {
    const goals = store.plan.goals;
    (goalEdits || []).forEach(({ id, text }) => {
      const goal = goals.find((entry) => entry.id === id);
      if (goal) goal.text = text;
    });
    (goalsAdded || []).forEach((goal) => {
      goals.push({ ...goal, links: [...(goal.links || [])] });
    });
    (goalsRemoved || []).forEach((id) => {
      const index = goals.findIndex((entry) => entry.id === id);
      if (index !== -1) goals.splice(index, 1);
    });
    (relinks || []).forEach(({ goalId, links }) => {
      const goal = goals.find((entry) => entry.id === goalId);
      if (goal) goal.links = [...links];
    });
    (lessonsAdded || []).forEach(({ id, slug, title }) => {
      store.lessons.push({ id, slug, title, status: 'planned' });
    });
    (lessonsRemoved || []).forEach((id) => {
      const lesson = store.lessons.find((entry) => entry.id === id);
      if (lesson) lesson.status = 'skipped';
    });
    store.plan.status = 'confirmed';
    store.log.push({ cmd: 'plan update + confirm pl1', ok: true });
    return store.plan;
  },
};

window.store = store;
