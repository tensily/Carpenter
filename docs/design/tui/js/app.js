'use strict';

(function () {
  const store = window.store;
  if (!store) {
    console.warn('mock store missing');
    return;
  }

  const SCREENS = ['home', 'intake', 'review', 'board', 'lesson', 'pivot'];
  const RENDERERS = {
    home: renderHome,
    intake: renderIntake,
    review: renderReview,
    board: renderBoard,
    lesson: renderLesson,
    pivot: renderPivot,
  };
  const SINGLE_SELECT_CHIP_GROUPS = new Set(['note-kind', 'f-pace', 'f-depth']);
  const SPEC_FIELDS = [
    ['title', 'Title'],
    ['slug', 'Slug'],
    ['goal', 'Goal'],
    ['description', 'Description'],
  ];
  const DEFAULT_COURSE_SLUG = 'linalg-for-ml';
  const TOAST_MS = 2400;

  let currentLessonId = null;
  let wizardStep = 0;
  let modalOnConfirm = null;
  let toastTimer = null;

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = text;
    return node;
  }

  function setHash(screen) {
    location.hash = '#/' + screen;
  }

  function screenFromHash() {
    const name = location.hash.replace(/^#\/?/, '');
    return SCREENS.includes(name) ? name : 'home';
  }

  function route() {
    const screen = screenFromHash();
    document.querySelectorAll('.c-screen').forEach((section) => {
      section.classList.toggle('hidden', section.dataset.screen !== screen);
    });
    const render = RENDERERS[screen];
    if (render) render();
    document.querySelectorAll('[data-nav]').forEach((nav) => {
      if (nav.dataset.nav === screen) nav.setAttribute('aria-current', 'page');
      else nav.removeAttribute('aria-current');
    });
  }

  function onDocumentClick(event) {
    if (event.target === modalEl()) {
      closeModal();
      return;
    }
    if (event.target.closest('#modal-confirm')) {
      const onConfirm = modalOnConfirm;
      if (onConfirm) onConfirm();
      closeModal();
      return;
    }
    const action = event.target.closest('[data-action]');
    if (action) {
      runAction(action.dataset.action, action);
      return;
    }
    const nav = event.target.closest('[data-nav]');
    if (nav) {
      adjustWizardStep(nav);
      setHash(nav.dataset.nav);
      return;
    }
    const row = event.target.closest('.c-row--clickable');
    if (row) {
      selectRow(row);
      return;
    }
    const chip = event.target.closest('.c-chip');
    if (chip) toggleChip(chip);
  }

  function runAction(name, source) {
    switch (name) {
      case 'confirm-scaffold': return confirmScaffold();
      case 'lesson-execute': return updateLessonStatus('verified', 'executed: 7 ran · 0 errored');
      case 'lesson-verify': return lessonStatusUpdate('verified');
      case 'lesson-skip': return lessonStatusUpdate('skipped');
      case 'add-note': return addNote();
      case 'confirm-pivot': return confirmPivot();
      case 'goal-add': return addGoal();
      case 'goal-remove': return removeGoal(source);
      case 'modal-cancel': return closeModal();
    }
  }

  function confirmScaffold() {
    const summary = el('div');
    appendSpecRows(summary);
    openModal('Scaffold course?', summary, () => {
      const slug = (store.draftSpec && store.draftSpec.slug) || DEFAULT_COURSE_SLUG;
      toast('course created: ' + slug, 'ok');
      setHash('board');
    });
  }

  function lessonStatusUpdate(status) {
    const lesson = currentLesson();
    if (lesson) updateLessonStatus(status, status + ': ' + lesson.slug);
  }

  function updateLessonStatus(status, message) {
    const lesson = currentLesson();
    if (!lesson) return;
    store.setLessonStatus(lesson.id, status);
    toast(message, 'ok');
    renderLesson();
    renderBoard();
  }

  function addNote() {
    const lesson = currentLesson();
    const field = document.getElementById('note-text');
    const text = field ? field.value.trim() : '';
    if (!lesson) return;
    if (!text) {
      toast('note text required', 'err');
      return;
    }
    const kind = selectedChipValue('note-kind') || 'gap';
    store.addNote({ lessonId: lesson.id, kind, text });
    if (field) field.value = '';
    renderNotes(lesson);
    toast('note added', 'ok');
  }

  function confirmPivot() {
    store.applyPivot({});
    toast('plan updated · confirmed', 'ok');
    setHash('board');
  }

  function addGoal() {
    const goals = (store.plan && store.plan.goals) || [];
    goals.push({ id: nextGoalId(goals), text: '', links: [] });
    renderPivot();
  }

  function removeGoal(button) {
    const row = button.closest('[data-goal-id]');
    if (!row || !store.plan) return;
    store.plan.goals = (store.plan.goals || []).filter((goal) => goal.id !== row.dataset.goalId);
    renderPivot();
  }

  function nextGoalId(goals) {
    let n = goals.length + 1;
    while (goals.some((goal) => goal.id === 'g' + n)) n += 1;
    return 'g' + n;
  }

  function modalEl() {
    return document.getElementById('modal');
  }

  function openModal(title, bodyNode, onConfirm) {
    const modal = modalEl();
    if (!modal) return;
    const titleEl = document.getElementById('modal-title');
    const bodyEl = document.getElementById('modal-body');
    if (titleEl) titleEl.textContent = title;
    if (bodyEl) bodyEl.replaceChildren(bodyNode);
    modalOnConfirm = onConfirm || null;
    modal.classList.remove('hidden');
  }

  function closeModal() {
    const modal = modalEl();
    if (modal) modal.classList.add('hidden');
    modalOnConfirm = null;
  }

  function modalIsOpen() {
    const modal = modalEl();
    return Boolean(modal) && !modal.classList.contains('hidden');
  }

  function toast(text, kind) {
    const node = document.getElementById('toast');
    if (!node) return;
    node.textContent = text;
    node.classList.remove('c-toast--ok', 'c-toast--err');
    node.classList.add(kind === 'err' ? 'c-toast--err' : 'c-toast--ok');
    node.classList.remove('hidden');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => node.classList.add('hidden'), TOAST_MS);
  }

  function selectedChipValue(groupId) {
    const group = document.getElementById(groupId);
    if (!group) return null;
    const chip = group.querySelector('.c-chip--on');
    if (!chip) return null;
    return (chip.dataset.kind || chip.textContent || '').trim().toLowerCase();
  }

  function toggleChip(chip) {
    const group = chip.closest('.c-chips');
    if (group && SINGLE_SELECT_CHIP_GROUPS.has(group.id)) {
      group.querySelectorAll('.c-chip--on').forEach((other) => {
        if (other !== chip) other.classList.remove('c-chip--on');
      });
    }
    chip.classList.toggle('c-chip--on');
  }

  function wizardSteps() {
    return Array.from(document.querySelectorAll('[data-screen="intake"] .c-step'));
  }

  function renderIntake() {
    renderWizard();
  }

  function renderWizard() {
    const steps = wizardSteps();
    if (!steps.length) return;
    wizardStep = Math.max(0, Math.min(wizardStep, steps.length - 1));
    steps.forEach((step, index) => {
      step.classList.toggle('c-step--active', index === wizardStep);
      step.classList.toggle('c-step--done', index < wizardStep);
    });
  }

  function adjustWizardStep(nav) {
    const screen = nav.closest('.c-screen');
    if (!screen || screen.dataset.screen !== 'intake') return;
    const label = (nav.textContent || '').trim().toLowerCase();
    const last = Math.max(0, wizardSteps().length - 1);
    if (label.includes('continue')) wizardStep = Math.min(wizardStep + 1, last);
    else if (label.includes('back')) wizardStep = Math.max(wizardStep - 1, 0);
    renderWizard();
  }

  function selectRow(row) {
    const list = row.parentElement;
    if (list) {
      list.querySelectorAll('.c-row--selected').forEach((selected) => {
        selected.classList.remove('c-row--selected');
      });
    }
    row.classList.add('c-row--selected');
    if (row.closest('#lesson-list')) {
      currentLessonId = row.dataset.lessonId || null;
      renderLesson();
    } else if (row.closest('[data-screen="home"]')) {
      if (row.dataset.courseSlug) store.activeCourse = row.dataset.courseSlug;
      setHash('board');
    }
  }

  function renderHome() {
    const list = document.querySelector('[data-screen="home"] .c-list');
    if (!list) return;
    list.replaceChildren();
    (store.courses || []).forEach((course) => list.append(courseRow(course)));
  }

  function courseRow(course) {
    const row = el('div', 'c-row c-row--clickable');
    row.dataset.courseSlug = course.slug;
    row.append(
      el('span', null, course.title),
      el('span', 'c-kbd', course.slug),
      progressNode(course.progressPct),
      el('span', null, (course.lessonCount || 0) + ' lessons')
    );
    return row;
  }

  function progressNode(pct) {
    const bar = el('div', 'c-progress');
    const fill = el('div', 'c-progress__fill');
    fill.style.width = (pct || 0) + '%';
    bar.append(fill);
    return bar;
  }

  function appendSpecRows(container) {
    const spec = store.draftSpec || {};
    SPEC_FIELDS.forEach(([key, label]) => {
      if (spec[key]) container.append(kvRow(label, spec[key]));
    });
  }

  function kvRow(label, value) {
    const row = el('div', 'c-kv');
    row.append(el('span', null, label), el('span', null, value));
    return row;
  }

  function renderReview() {
    const preview = document.getElementById('spec-preview');
    if (!preview) return;
    preview.replaceChildren();
    appendSpecRows(preview);
  }

  function renderBoard() {
    renderPlanStatus();
    renderGoals();
    renderLessons();
    renderProgress();
  }

  function renderPlanStatus() {
    const holder = document.getElementById('plan-status');
    if (!holder) return;
    const badge = holder.querySelector('.c-badge') || holder;
    const status = (store.plan && store.plan.status) || 'draft';
    badge.className = badge === holder ? 'c-badge c-badge--' + status : 'c-badge--' + status;
    badge.textContent = status;
  }

  function renderGoals() {
    const list = document.getElementById('goals-list');
    if (!list) return;
    list.replaceChildren();
    ((store.plan && store.plan.goals) || []).forEach((goal) => list.append(goalRow(goal)));
  }

  function goalRow(goal) {
    const row = el('div', 'c-row');
    row.append(el('span', null, goal.text));
    (goal.links || []).forEach((lessonId) => {
      const lesson = (store.lessons || []).find((entry) => entry.id === lessonId);
      row.append(
        el('span', lesson ? 'c-badge c-badge--' + lesson.status : 'c-badge', lesson ? lesson.slug : lessonId)
      );
    });
    return row;
  }

  function renderLessons() {
    const list = document.getElementById('lesson-list');
    if (!list) return;
    list.replaceChildren();
    (store.lessons || []).forEach((lesson) => list.append(lessonRow(lesson)));
  }

  function lessonRow(lesson) {
    const row = el('div', 'c-row c-row--clickable');
    row.dataset.lessonId = lesson.id;
    row.classList.toggle('c-row--selected', lesson.id === currentLessonId);
    row.append(
      el('span', 'c-badge c-badge--' + lesson.status, lesson.status),
      el('span', null, lesson.title),
      el('span', 'c-kbd', lesson.slug)
    );
    return row;
  }

  function renderProgress() {
    const rollup = document.getElementById('progress-rollup');
    if (!rollup) return;
    const lessons = store.lessons || [];
    const verified = lessons.filter((lesson) => lesson.status === 'verified').length;
    const pct = lessons.length ? Math.round((100 * verified) / lessons.length) : 0;
    rollup.replaceChildren(
      progressNode(pct),
      el('div', null, pct + '% · ' + verified + '/' + lessons.length + ' lessons verified')
    );
  }

  function currentLesson() {
    const lessons = store.lessons || [];
    if (currentLessonId) {
      const pinned = lessons.find((lesson) => lesson.id === currentLessonId);
      if (pinned) return pinned;
    }
    const seeded = lessons.find(
      (lesson) => lesson.id === 'vectors-refresher' || lesson.slug === 'vectors-refresher'
    );
    return seeded || lessons.find((lesson) => lesson.status === 'created') || lessons[0] || null;
  }

  function renderLesson() {
    const lesson = currentLesson();
    if (!lesson) return;
    currentLessonId = lesson.id;
    const title = document.getElementById('lesson-title');
    if (title) title.textContent = lesson.title;
    renderLessonActions(lesson);
    renderNotes(lesson);
  }

  function renderLessonActions(lesson) {
    const holder = document.getElementById('lesson-actions');
    if (!holder) return;
    const enabled = {
      'lesson-execute': lesson.status === 'created',
      'lesson-verify': lesson.status === 'planned' || lesson.status === 'created',
      'lesson-skip': lesson.status !== 'skipped',
    };
    holder.querySelectorAll('[data-action]').forEach((button) => {
      const allowed = enabled[button.dataset.action];
      if (allowed !== undefined) button.disabled = !allowed;
    });
  }

  function renderNotes(lesson) {
    const list = document.getElementById('notes-list');
    if (!list) return;
    list.replaceChildren();
    const notes = (store.notes || []).filter((note) => note.lessonId === lesson.id);
    if (!notes.length) {
      list.append(el('div', 'c-empty', 'no notes yet'));
      return;
    }
    notes.forEach((note) => {
      const node = el('div', 'c-note');
      node.append(el('span', 'c-badge c-badge--' + note.kind, note.kind), el('span', null, note.text));
      list.append(node);
    });
  }

  function renderPivot() {
    const holder = document.getElementById('pivot-goals');
    if (!holder) return;
    const add = holder.querySelector('[data-action="goal-add"]');
    holder.replaceChildren();
    ((store.plan && store.plan.goals) || []).forEach((goal) => holder.append(goalInputRow(goal)));
    if (add) holder.append(add);
  }

  function goalInputRow(goal) {
    const row = el('div', 'c-row');
    row.dataset.goalId = goal.id;
    const input = el('input', 'c-input');
    input.type = 'text';
    input.value = goal.text || '';
    const remove = el('button', 'c-btn c-btn--ghost', 'Remove');
    remove.type = 'button';
    remove.dataset.action = 'goal-remove';
    row.append(input, remove);
    return row;
  }

  function onDocumentInput(event) {
    const row = event.target.closest('#pivot-goals [data-goal-id]');
    if (!row || !store.plan) return;
    const goal = (store.plan.goals || []).find((entry) => entry.id === row.dataset.goalId);
    if (goal) goal.text = event.target.value;
  }

  function onDocumentKeydown(event) {
    if (modalIsOpen()) {
      if (event.key === 'Escape') closeModal();
      return;
    }
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) return;
    if (event.key === 'ArrowRight') cycleScreen(1);
    else if (event.key === 'ArrowLeft') cycleScreen(-1);
    else if (event.key === 'n') setHash('intake');
    else if (event.key === 'p') setHash('pivot');
  }

  function cycleScreen(delta) {
    const index = SCREENS.indexOf(screenFromHash());
    const next = (index + delta + SCREENS.length) % SCREENS.length;
    setHash(SCREENS[next]);
  }

  function start() {
    document.addEventListener('click', onDocumentClick);
    document.addEventListener('input', onDocumentInput);
    document.addEventListener('keydown', onDocumentKeydown);
    window.addEventListener('hashchange', route);
    if (!location.hash) location.hash = '#/home';
    route();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
