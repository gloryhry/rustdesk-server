export function element(tag, className = '', text = '') {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== '') node.textContent = text;
  return node;
}

export function button(id, label, variant = '') {
  const node = element('button', `button ${variant}`.trim(), label);
  node.id = id;
  node.type = 'button';
  return node;
}

export function badge(text, tone = 'neutral') {
  return element('span', `badge ${tone}`.trim(), text);
}

export function pageHeader(title, subtitle, actions = []) {
  const header = element('div', 'page-header');
  const heading = element('div', 'page-heading');
  heading.append(element('h1', '', title));
  if (subtitle) heading.append(element('p', 'page-subtitle', subtitle));
  header.append(heading);
  if (actions.length) {
    const group = element('div', 'button-group');
    actions.filter(Boolean).forEach(action => group.append(action));
    header.append(group);
  }
  return header;
}

export function card({ id = '', className = '', title = '', subtitle = '', actions = [], headerExtra = null, body = null, footer = null } = {}) {
  const node = element('section', `card ${className}`.trim());
  if (id) node.id = id;
  if (title || subtitle || actions.length || headerExtra) {
    const header = element('div', 'card-header');
    const heading = element('div', 'card-heading');
    if (title) heading.append(element('h2', '', title));
    if (subtitle) heading.append(element('p', 'card-subtitle', subtitle));
    header.append(heading);
    if (headerExtra) header.append(headerExtra);
    if (actions.length) {
      const group = element('div', 'button-group');
      actions.filter(Boolean).forEach(action => group.append(action));
      header.append(group);
    }
    node.append(header);
  }
  if (body) {
    const bodyNode = element('div', 'card-body');
    appendContent(bodyNode, body);
    node.append(bodyNode);
  }
  if (footer) {
    const footerNode = element('div', 'card-footer');
    appendContent(footerNode, footer);
    node.append(footerNode);
  }
  return node;
}

function appendContent(parent, content) {
  if (Array.isArray(content)) {
    content.filter(Boolean).forEach(item => appendContent(parent, item));
  } else if (content instanceof Node) {
    parent.append(content);
  } else if (content !== null && content !== undefined) {
    parent.append(document.createTextNode(String(content)));
  }
}

export function formGrid(children, className = '') {
  const grid = element('div', `form-grid ${className}`.trim());
  appendContent(grid, children);
  return grid;
}

export function formSection(title, description, children, options = {}) {
  const fieldset = element('fieldset', `form-section ${options.className || ''}`.trim());
  if (title) fieldset.append(element('legend', '', title));
  if (description) fieldset.append(element('p', 'form-section-description', description));
  fieldset.append(formGrid(children, options.gridClassName || ''));
  return fieldset;
}

export function configField(id, label, value, type = 'text', options = {}) {
  const wrapper = element('label', `field ${options.className || ''}`.trim());
  wrapper.htmlFor = id;
  const labelNode = element('span', 'field-label', label);
  if (options.required) labelNode.append(element('span', 'required-mark', ' *'));
  wrapper.append(labelNode);

  let input;
  if (type === 'textarea') {
    input = document.createElement('textarea');
    input.rows = options.rows || 2;
  } else if (type === 'select') {
    input = document.createElement('select');
    for (const option of options.options || []) {
      const node = document.createElement('option');
      node.value = option.value;
      node.textContent = option.label;
      input.append(node);
    }
  } else {
    input = document.createElement('input');
    input.type = type;
  }

  input.id = id;
  input.name = options.name || id;
  input.disabled = Boolean(options.disabled);
  input.readOnly = Boolean(options.readOnly);
  input.required = Boolean(options.required);
  input.autocomplete = options.autocomplete || 'off';
  if (options.placeholder) input.placeholder = options.placeholder;
  if (options.title) input.title = options.title;
  if (options.maxLength) input.maxLength = options.maxLength;
  if (options.min !== undefined) input.min = String(options.min);
  if (options.max !== undefined) input.max = String(options.max);
  if (options.inputMode) input.inputMode = options.inputMode;
  if (options.spellcheck === false) input.spellcheck = false;
  if (type === 'textarea' || type === 'select') input.value = value ?? '';
  else input.value = value ?? '';
  wrapper.append(input);
  if (options.help) wrapper.append(element('small', 'field-help', options.help));
  return wrapper;
}

export function checkboxField(id, label, checked, options = {}) {
  const wrapper = element('label', `field checkbox-field ${options.className || ''}`.trim());
  wrapper.htmlFor = id;
  const input = document.createElement('input');
  input.id = id;
  input.name = options.name || id;
  input.type = 'checkbox';
  input.checked = Boolean(checked);
  input.disabled = Boolean(options.disabled);
  wrapper.append(input, element('span', '', label));
  if (options.help) wrapper.append(element('small', 'field-help', options.help));
  return wrapper;
}

export function selectField(id, label, options, value, config = {}) {
  return configField(id, label, value, 'select', { ...config, options });
}

export function field(id, label, type, autocomplete = 'off', required = false) {
  return configField(id, label, '', type, { autocomplete, required, maxLength: id === 'password' ? 72 : 254 });
}

export function detail(list, label, value) {
  const item = element('div', 'detail-item');
  item.append(element('dt', '', label));
  item.append(element('dd', '', value));
  list.append(item);
}

export function emptyState(text, action = null) {
  const node = element('div', 'empty-state-card');
  node.append(element('p', '', text));
  if (action) node.append(action);
  return node;
}

export function tableShell(headers, rows, options = {}) {
  const wrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  if (options.responsive !== false) table.className = 'responsive-table';
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  headers.forEach(header => {
    const th = element('th', '', typeof header === 'string' ? header : header.label);
    th.scope = 'col';
    if (header && typeof header === 'object' && header.className) th.className = header.className;
    headRow.append(th);
  });
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (options.bodyId) body.id = options.bodyId;
  rows.forEach(row => body.append(row));
  table.append(body);
  wrap.append(table);
  return { wrap, table, tbody: body };
}

export function cell(label, content, className = '') {
  const td = document.createElement('td');
  if (className) td.className = className;
  if (label) td.dataset.label = label;
  if (content instanceof Node) td.append(content);
  else td.textContent = content ?? '—';
  return td;
}

export function emptyRow(colSpan, text) {
  const row = document.createElement('tr');
  const td = element('td', 'empty-cell', text);
  td.colSpan = colSpan;
  row.append(td);
  return row;
}

export function tabs(id, items, active) {
  const nav = element('div', 'tabs');
  nav.id = id;
  nav.setAttribute('role', 'tablist');
  for (const item of items) {
    const node = element('button', `tab ${item.id === active ? 'active' : ''}`.trim(), item.label);
    node.type = 'button';
    node.id = `${id}-${item.id}`;
    node.dataset.tab = item.id;
    node.setAttribute('role', 'tab');
    node.setAttribute('aria-selected', String(item.id === active));
    node.setAttribute('aria-controls', `${id}-${item.id}-panel`);
    node.tabIndex = item.id === active ? 0 : -1;
    nav.append(node);
  }
  nav.addEventListener('keydown', event => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const buttons = [...nav.querySelectorAll('[role="tab"]')];
    const index = buttons.indexOf(document.activeElement);
    if (index < 0) return;
    event.preventDefault();
    let next = index;
    if (event.key === 'ArrowLeft') next = (index - 1 + buttons.length) % buttons.length;
    if (event.key === 'ArrowRight') next = (index + 1) % buttons.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = buttons.length - 1;
    buttons[next].focus();
    buttons[next].click();
  });
  return nav;
}

export function tagChip(name, color = '') {
  const chip = element('span', 'tag-chip');
  if (color) {
    const swatch = element('span', 'tag-swatch');
    swatch.style.backgroundColor = color;
    swatch.title = color;
    chip.append(swatch);
  }
  chip.append(element('span', '', name));
  return chip;
}
