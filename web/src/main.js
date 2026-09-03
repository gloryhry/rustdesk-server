import './style.css';

const API_BASE = (import.meta.env.VITE_API_BASE || '').replace(/\/$/, '');
const TOKEN_KEY = 'rustdesk_api_token';
const LOCALE_KEY = 'rustdesk_api_locale';

const messages = {
  'zh-CN': {
    brand: 'RustDesk API',
    login: '登录',
    register: '注册',
    username: '用户名',
    email: '邮箱',
    password: '密码',
    signingIn: '正在登录...',
    registering: '正在注册...',
    profile: '当前用户',
    addressBook: '地址簿',
    users: '用户管理',
    devices: '设备',
    groups: '用户组',
    logout: '退出登录',
    refresh: '刷新',
    save: '保存',
    saving: '正在保存...',
    account: '账户',
    role: '角色',
    status: '状态',
    administrator: '管理员',
    member: '普通用户',
    enabled: '启用',
    disabled: '禁用',
    createdAt: '创建时间',
    noUsers: '暂无用户',
    noDevices: '暂无设备',
    noGroups: '暂无用户组',
    uuid: '设备 UUID',
    platform: '平台',
    invalidJson: '地址簿必须是有效的 JSON 对象',
    saved: '地址簿已保存',
    registered: '注册成功，请登录',
    requestFailed: '请求失败',
    sessionExpired: '登录已失效，请重新登录',
    emptyAddressBook: '地址簿为空',
    language: 'English',
    loading: '加载中...',
    signedInAs: '已登录账户',
    apiUnavailable: '无法连接 API 服务'
  },
  'en-US': {
    brand: 'RustDesk API',
    login: 'Sign in',
    register: 'Register',
    username: 'Username',
    email: 'Email',
    password: 'Password',
    signingIn: 'Signing in...',
    registering: 'Registering...',
    profile: 'Current user',
    addressBook: 'Address book',
    users: 'User management',
    logout: 'Sign out',
    refresh: 'Refresh',
    save: 'Save',
    saving: 'Saving...',
    account: 'Account',
    role: 'Role',
    status: 'Status',
    administrator: 'Administrator',
    member: 'Member',
    enabled: 'Enabled',
    disabled: 'Disabled',
    createdAt: 'Created',
    noUsers: 'No users found',
    noDevices: 'No devices found',
    noGroups: 'No groups found',
    uuid: 'Device UUID',
    platform: 'Platform',
    invalidJson: 'Address book must be a valid JSON object',
    saved: 'Address book saved',
    registered: 'Registration complete. Sign in to continue.',
    requestFailed: 'Request failed',
    sessionExpired: 'Your session expired. Sign in again.',
    emptyAddressBook: 'Address book is empty',
    language: '简体中文',
    loading: 'Loading...',
    signedInAs: 'Signed in as',
    apiUnavailable: 'Unable to reach the API server'
  }
};

const state = {
  locale: localStorage.getItem(LOCALE_KEY) || 'zh-CN',
  token: sessionStorage.getItem(TOKEN_KEY) || '',
  user: null,
  activeView: 'profile',
  authMode: 'login',
  addressBook: '{\n  "peers": [],\n  "tags": [],\n  "tag_colors": "{}"\n}',
  users: [],
  devices: [],
  groups: [],
  busy: false,
  notice: null
};

const app = document.querySelector('#app');
const t = key => messages[state.locale][key] || key;

function setNotice(type, text) {
  state.notice = text ? { type, text } : null;
}

function safeUser(payload) {
  return payload?.data?.user || payload?.data || payload?.user || payload;
}

function render() {
  document.documentElement.lang = state.locale;
  app.replaceChildren();

  const shell = element('div', 'app-shell');
  shell.append(header());
  shell.append(state.token && state.user ? workspace() : authView());
  if (state.notice) shell.append(notice());
  app.append(shell);
  bindEvents();
}

function header() {
  const node = element('header', 'topbar');
  const brand = element('div', 'brand');
  brand.append(element('span', 'brand-mark', 'R'));
  brand.append(element('strong', '', t('brand')));
  node.append(brand);

  const actions = element('div', 'topbar-actions');
  const locale = button('locale-toggle', t('language'), 'secondary');
  locale.type = 'button';
  actions.append(locale);
  if (state.token && state.user) actions.append(button('logout', t('logout'), 'secondary'));
  node.append(actions);
  return node;
}

function authView() {
  const main = element('main', 'auth-layout');
  const panel = element('section', 'auth-panel');
  const tabs = element('div', 'segmented');
  tabs.append(tabButton('login', t('login'), state.authMode === 'login'));
  tabs.append(tabButton('register', t('register'), state.authMode === 'register'));
  panel.append(tabs);

  const form = element('form', 'auth-form');
  form.id = 'auth-form';
  form.append(field('username', t('username'), 'text', 'username', true));
  if (state.authMode === 'register') {
    form.append(field('email', t('email'), 'email', 'email', false));
  }
  form.append(field('password', t('password'), 'password', 'current-password', true));
  const submit = button('auth-submit', state.busy
    ? t(state.authMode === 'login' ? 'signingIn' : 'registering')
    : t(state.authMode), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  panel.append(form);
  main.append(panel);
  return main;
}

function workspace() {
  const main = element('main', 'workspace');
  const sidebar = element('aside', 'sidebar');
  const identity = element('div', 'identity');
  identity.append(element('span', 'avatar', state.user.name?.slice(0, 1).toUpperCase() || 'U'));
  const copy = element('div');
  copy.append(element('small', '', t('signedInAs')));
  copy.append(element('strong', '', state.user.name || state.user.username || 'user'));
  identity.append(copy);
  sidebar.append(identity);

  const nav = element('nav', 'nav-list');
  nav.append(navButton('profile', t('profile')));
  nav.append(navButton('addressBook', t('addressBook')));
  nav.append(navButton('devices', t('devices')));
  nav.append(navButton('groups', t('groups')));
  if (state.user.is_admin) nav.append(navButton('users', t('users')));
  sidebar.append(nav);
  main.append(sidebar);

  const content = element('section', 'content');
  if (state.activeView === 'addressBook') content.append(addressBookView());
  else if (state.activeView === 'devices') content.append(devicesView());
  else if (state.activeView === 'groups') content.append(groupsView());
  else if (state.activeView === 'users' && state.user.is_admin) content.append(usersView());
  else content.append(profileView());
  main.append(content);
  return main;
}

function profileView() {
  const view = viewHeader(t('profile'));
  const grid = element('dl', 'detail-grid');
  detail(grid, t('account'), state.user.name || state.user.username || '');
  detail(grid, t('email'), state.user.email || '—');
  detail(grid, t('role'), state.user.is_admin ? t('administrator') : t('member'));
  detail(grid, t('status'), Number(state.user.status) === 1 ? t('enabled') : t('disabled'));
  view.append(grid);
  return view;
}

function addressBookView() {
  const view = viewHeader(t('addressBook'), [
    button('refresh-address-book', t('refresh'), 'secondary'),
    button('save-address-book', state.busy ? t('saving') : t('save'), 'primary')
  ]);
  const editor = document.createElement('textarea');
  editor.id = 'address-book-editor';
  editor.className = 'json-editor';
  editor.spellcheck = false;
  editor.value = state.addressBook;
  editor.disabled = state.busy;
  view.append(editor);
  return view;
}

function usersView() {
  const view = viewHeader(t('users'), [button('refresh-users', t('refresh'), 'secondary')]);
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  [t('account'), t('email'), t('role'), t('status'), t('createdAt')].forEach(label => {
    headRow.append(element('th', '', label));
  });
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (!state.users.length) {
    const row = document.createElement('tr');
    const cell = element('td', 'empty-cell', t('noUsers'));
    cell.colSpan = 5;
    row.append(cell);
    body.append(row);
  } else {
    state.users.forEach(user => {
      const row = document.createElement('tr');
      row.append(element('td', 'strong-cell', user.name || user.username || ''));
      row.append(element('td', '', user.email || '—'));
      row.append(element('td', '', user.is_admin ? t('administrator') : t('member')));
      row.append(element('td', '', Number(user.status) === 1 ? t('enabled') : t('disabled')));
      row.append(element('td', '', user.created_at || user.createdAt || '—'));
      body.append(row);
    });
  }
  table.append(body);
  tableWrap.append(table);
  view.append(tableWrap);
  return view;
}

function devicesView() {
  const view = viewHeader(t('devices'), [button('refresh-devices', t('refresh'), 'secondary')]);
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  [t('account'), t('uuid'), t('platform'), t('status')].forEach(label => {
    headRow.append(element('th', '', label));
  });
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (!state.devices.length) {
    const row = document.createElement('tr');
    const cell = element('td', 'empty-cell', t('noDevices'));
    cell.colSpan = 4;
    row.append(cell);
    body.append(row);
  } else {
    state.devices.forEach(device => {
      const row = document.createElement('tr');
      row.append(element('td', 'strong-cell', device.name || device.id || '—'));
      row.append(element('td', '', device.uuid || '—'));
      row.append(element('td', '', [device.os, device.device_type].filter(Boolean).join(' / ') || '—'));
      row.append(element('td', '', Number(device.status) === 1 ? t('enabled') : t('disabled')));
      body.append(row);
    });
  }
  table.append(body);
  tableWrap.append(table);
  view.append(tableWrap);
  return view;
}

function groupsView() {
  const view = viewHeader(t('groups'), [button('refresh-groups', t('refresh'), 'secondary')]);
  const list = element('div', 'group-list');
  if (!state.groups.length) list.append(element('p', 'empty-state', t('noGroups')));
  state.groups.forEach(group => {
    const item = element('div', 'group-row');
    item.append(element('strong', '', group.name || '—'));
    item.append(element('small', '', group.created_at || ''));
    list.append(item);
  });
  view.append(list);
  return view;
}

function viewHeader(title, actions = []) {
  const view = element('div', 'view');
  const heading = element('div', 'view-heading');
  heading.append(element('h1', '', title));
  const group = element('div', 'button-group');
  actions.forEach(action => group.append(action));
  heading.append(group);
  view.append(heading);
  return view;
}

function notice() {
  const node = element('div', `notice ${state.notice.type}`, state.notice.text);
  node.setAttribute('role', 'status');
  return node;
}

function field(id, label, type, autocomplete, required) {
  const wrapper = element('label', 'field');
  wrapper.htmlFor = id;
  wrapper.append(element('span', '', label));
  const input = document.createElement('input');
  input.id = id;
  input.name = id;
  input.type = type;
  input.autocomplete = autocomplete;
  input.required = required;
  input.maxLength = id === 'password' ? 72 : 254;
  wrapper.append(input);
  return wrapper;
}

function detail(list, label, value) {
  const item = element('div', 'detail-item');
  item.append(element('dt', '', label));
  item.append(element('dd', '', value));
  list.append(item);
}

function element(tag, className = '', text = '') {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== '') node.textContent = text;
  return node;
}

function button(id, label, variant) {
  const node = element('button', `button ${variant}`, label);
  node.id = id;
  node.type = 'button';
  return node;
}

function tabButton(mode, label, active) {
  const node = button(`auth-${mode}`, label, active ? 'active' : '');
  node.dataset.authMode = mode;
  return node;
}

function navButton(view, label) {
  const node = button(`nav-${view}`, label, state.activeView === view ? 'nav-active' : 'nav');
  node.dataset.view = view;
  return node;
}

function bindEvents() {
  document.querySelector('#locale-toggle')?.addEventListener('click', () => {
    state.locale = state.locale === 'zh-CN' ? 'en-US' : 'zh-CN';
    localStorage.setItem(LOCALE_KEY, state.locale);
    render();
  });
  document.querySelectorAll('[data-auth-mode]').forEach(node => node.addEventListener('click', () => {
    state.authMode = node.dataset.authMode;
    setNotice(null, null);
    render();
  }));
  document.querySelector('#auth-form')?.addEventListener('submit', submitAuth);
  document.querySelector('#logout')?.addEventListener('click', logout);
  document.querySelectorAll('[data-view]').forEach(node => node.addEventListener('click', async () => {
    state.activeView = node.dataset.view;
    setNotice(null, null);
    render();
    if (state.activeView === 'addressBook') await loadAddressBook();
    if (state.activeView === 'devices') await loadDevices();
    if (state.activeView === 'groups') await loadGroups();
    if (state.activeView === 'users') await loadUsers();
  }));
  document.querySelector('#refresh-address-book')?.addEventListener('click', loadAddressBook);
  document.querySelector('#save-address-book')?.addEventListener('click', saveAddressBook);
  document.querySelector('#refresh-users')?.addEventListener('click', loadUsers);
  document.querySelector('#refresh-devices')?.addEventListener('click', loadDevices);
  document.querySelector('#refresh-groups')?.addEventListener('click', loadGroups);
}

async function submitAuth(event) {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const payload = {
    username: String(data.get('username') || ''),
    password: String(data.get('password') || '')
  };
  if (state.authMode === 'register') payload.email = String(data.get('email') || '');
  state.busy = true;
  setNotice(null, null);
  render();
  try {
    const result = await api(`/api/${state.authMode}`, {
      method: 'POST',
      body: JSON.stringify(payload),
      authenticated: false
    });
    if (state.authMode === 'register') {
      state.authMode = 'login';
      setNotice('success', t('registered'));
    } else {
      state.token = result.access_token || result.data?.access_token || '';
      if (!state.token) throw new Error(t('requestFailed'));
      sessionStorage.setItem(TOKEN_KEY, state.token);
      await loadCurrentUser(false);
    }
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadCurrentUser(renderAfter = true) {
  try {
    const result = await api('/api/currentUser');
    state.user = safeUser(result);
    if (!state.user || typeof state.user !== 'object') throw new Error(t('requestFailed'));
  } catch (error) {
    clearSession();
    setNotice('error', error.message);
  }
  if (renderAfter) render();
}

async function loadAddressBook() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/ab');
    const raw = result.data ?? result;
    const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
    state.addressBook = JSON.stringify(parsed, null, 2);
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function saveAddressBook() {
  const editor = document.querySelector('#address-book-editor');
  const raw = editor?.value || '';
  let parsed;
  try {
    parsed = JSON.parse(raw);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error();
  } catch {
    setNotice('error', t('invalidJson'));
    render();
    return;
  }
  state.busy = true;
  render();
  try {
    await api('/api/ab', { method: 'POST', body: JSON.stringify({ data: JSON.stringify(parsed) }) });
    state.addressBook = JSON.stringify(parsed, null, 2);
    setNotice('success', t('saved'));
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadDevices() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/devices');
    const value = result.data?.list || result.data || result.list || result;
    state.devices = Array.isArray(value) ? value : [];
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadGroups() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/groups');
    const value = result.data?.list || result.data || result.list || result;
    state.groups = Array.isArray(value) ? value : [];
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadUsers() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/admin/user/list');
    const value = result.data?.list || result.data || result.list || result;
    state.users = Array.isArray(value) ? value : [];
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function logout() {
  try {
    await api('/api/logout', { method: 'POST' });
  } catch {
    // Local session cleanup is still required when the server session expired.
  }
  clearSession();
  setNotice(null, null);
  render();
}

async function api(path, options = {}) {
  const headers = { Accept: 'application/json', ...options.headers };
  if (options.body) headers['Content-Type'] = 'application/json';
  if (options.authenticated !== false && state.token) headers.Authorization = `Bearer ${state.token}`;
  let response;
  try {
    response = await fetch(`${API_BASE}${path}`, { ...options, headers });
  } catch {
    throw new Error(t('apiUnavailable'));
  }
  const text = await response.text();
  let payload = null;
  if (text) {
    try { payload = JSON.parse(text); } catch { payload = text; }
  }
  if (!response.ok) {
    if (response.status === 401) {
      clearSession();
      throw new Error(t('sessionExpired'));
    }
    const message = payload?.message || payload?.error || `${t('requestFailed')} (${response.status})`;
    throw new Error(message);
  }
  return payload;
}

function clearSession() {
  state.token = '';
  state.user = null;
  state.users = [];
  state.devices = [];
  state.groups = [];
  state.activeView = 'profile';
  sessionStorage.removeItem(TOKEN_KEY);
}

render();
if (state.token) loadCurrentUser();
