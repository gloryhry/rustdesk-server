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
    sessions: '会话管理',
    sessionId: '会话 ID',
    noSessions: '暂无会话',
    devices: '设备',
    groups: '用户组',
    deviceGroups: '设备组',
    groupName: '组名称',
    create: '创建',
    delete: '删除',
    noDeviceGroups: '暂无设备组',
    addDevice: '添加设备',
    noAssignedDevices: '尚未分配设备',
    ldap: 'LDAP 配置',
    ldapUrl: 'LDAP URL',
    bindDn: '绑定 DN',
    bindPassword: '绑定密码（留空则保持不变）',
    userBaseDn: '用户 Base DN',
    userFilter: '用户过滤器',
    usernameAttribute: '用户名属性',
    emailAttribute: '邮箱属性',
    useTls: '使用 StartTLS',
    timeoutSeconds: '超时（秒）',
    runtimeOnly: '配置仅应用于当前 API 进程，重启后从环境变量重新加载。',
    ldapSaved: 'LDAP 配置已应用',
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
    apiServer: 'API Server',
    idServer: 'ID Server',
    relayServer: 'Relay Server',
    publicKey: '公开 KEY',
    tags: '标签',
    tagName: '标签名称',
    tagColor: '颜色（可选）',
    noTags: '暂无标签',
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
    sessions: 'Session management',
    sessionId: 'Session ID',
    noSessions: 'No sessions found',
    devices: 'Devices',
    groups: 'User groups',
    deviceGroups: 'Device groups',
    groupName: 'Group name',
    create: 'Create',
    delete: 'Delete',
    noDeviceGroups: 'No device groups found',
    addDevice: 'Add device',
    noAssignedDevices: 'No assigned devices',
    ldap: 'LDAP configuration',
    ldapUrl: 'LDAP URL',
    bindDn: 'Bind DN',
    bindPassword: 'Bind password (leave blank to keep)',
    userBaseDn: 'User base DN',
    userFilter: 'User filter',
    usernameAttribute: 'Username attribute',
    emailAttribute: 'Email attribute',
    useTls: 'Use StartTLS',
    timeoutSeconds: 'Timeout (seconds)',
    runtimeOnly: 'Changes apply to the current API process only; restart reloads environment settings.',
    ldapSaved: 'LDAP configuration applied',
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
    apiServer: 'API Server',
    idServer: 'ID Server',
    relayServer: 'Relay Server',
    publicKey: 'Public key',
    tags: 'Tags',
    tagName: 'Tag name',
    tagColor: 'Color (optional)',
    noTags: 'No tags found',
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
  tags: [],
  tagDraft: { name: '', color: '' },
  serverConfig: null,
  users: [],
  sessions: [],
  devices: [],
  groups: [],
  deviceGroups: [],
  deviceGroupMemberships: [],
  deviceGroupDevices: [],
  deviceGroupDraft: '',
  deviceGroupsRequest: 0,
  devicesRequest: 0,
  ldap: null,
  ldapDraft: null,
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
  nav.append(navButton('deviceGroups', t('deviceGroups')));
  if (state.user.is_admin) {
    nav.append(navButton('users', t('users')));
    nav.append(navButton('sessions', t('sessions')));
    nav.append(navButton('ldap', t('ldap')));
  }
  sidebar.append(nav);
  main.append(sidebar);

  const content = element('section', 'content');
  if (state.activeView === 'addressBook') content.append(addressBookView());
  else if (state.activeView === 'devices') content.append(devicesView());
  else if (state.activeView === 'groups') content.append(groupsView());
  else if (state.activeView === 'deviceGroups') content.append(deviceGroupsView());
  else if (state.activeView === 'users' && state.user.is_admin) content.append(usersView());
  else if (state.activeView === 'sessions' && state.user.is_admin) content.append(sessionsView());
  else if (state.activeView === 'ldap' && state.user.is_admin) content.append(ldapView());
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
  if (state.serverConfig) {
    detail(grid, t('apiServer'), state.serverConfig.api_server || '—');
    detail(grid, t('idServer'), state.serverConfig.id_server || '—');
    detail(grid, t('relayServer'), state.serverConfig.relay_server || '—');
    detail(grid, t('publicKey'), state.serverConfig.key || '—');
  }
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
  view.append(tagsPanel());
  return view;
}

function tagsPanel() {
  const panel = element('section', 'tags-panel');
  panel.append(element('h2', '', t('tags')));
  const form = element('form', 'inline-form');
  form.id = 'tag-form';
  const name = document.createElement('input');
  name.name = 'name';
  name.placeholder = t('tagName');
  name.maxLength = 64;
  name.required = true;
  name.value = state.tagDraft.name;
  name.disabled = state.busy;
  const color = document.createElement('input');
  color.name = 'color';
  color.placeholder = t('tagColor');
  color.maxLength = 7;
  color.value = state.tagDraft.color;
  color.disabled = state.busy;
  form.append(name, color);
  const submit = button('save-tag', t('save'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  panel.append(form);
  const list = element('div', 'tag-list');
  if (!state.tags.length) list.append(element('p', 'empty-state', t('noTags')));
  state.tags.forEach(tag => {
    const row = element('div', 'tag-row');
    row.append(element('span', '', tag.name || '—'));
    if (tag.color) {
      const swatch = element('span', 'tag-swatch');
      swatch.style.backgroundColor = tag.color;
      swatch.title = tag.color;
      row.append(swatch);
    }
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteTag = tag.name || '';
    remove.disabled = state.busy;
    row.append(remove);
    list.append(row);
  });
  panel.append(list);
  return panel;
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

function sessionsView() {
  const view = viewHeader(t('sessions'), [button('refresh-sessions', t('refresh'), 'secondary')]);
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const row = document.createElement('tr');
  [t('sessionId'), t('account'), t('platform'), t('status')].forEach(label => row.append(element('th', '', label)));
  head.append(row);
  table.append(head);
  const body = document.createElement('tbody');
  state.sessions.forEach(session => {
    const item = document.createElement('tr');
    item.append(element('td', 'strong-cell', session.id || '—'));
    item.append(element('td', '', session.username || session.user_id || '—'));
    item.append(element('td', '', [session.device_os, session.device_type].filter(Boolean).join(' / ') || '—'));
    const revoke = button('', t('logout'), 'secondary');
    revoke.dataset.revokeSession = session.id;
    revoke.disabled = Boolean(session.revoked_at) || state.busy;
    const actionCell = document.createElement('td');
    actionCell.append(revoke);
    item.append(actionCell);
    body.append(item);
  });
  if (!state.sessions.length) {
    const emptyRow = document.createElement('tr');
    const emptyCell = element('td', 'empty-cell', t('noSessions'));
    emptyCell.colSpan = 4;
    emptyRow.append(emptyCell);
    body.append(emptyRow);
  }
  table.append(body);
  tableWrap.append(table);
  view.append(tableWrap);
  return view;
}

function devicesView() {
  const refresh = button('refresh-devices', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  const view = viewHeader(t('devices'), [refresh]);
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

function deviceGroupsView() {
  const refresh = button('refresh-device-groups', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  const view = viewHeader(t('deviceGroups'), [refresh]);
  const form = element('form', 'inline-form');
  form.id = 'device-group-form';
  const input = document.createElement('input');
  input.id = 'device-group-name';
  input.name = 'name';
  input.placeholder = t('groupName');
  input.value = state.deviceGroupDraft;
  input.maxLength = 128;
  input.required = true;
  input.disabled = state.busy;
  form.append(input);
  const submit = button('create-device-group', t('create'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  view.append(form);
  const list = element('div', 'group-list');
  if (!state.deviceGroups.length) list.append(element('p', 'empty-state', t('noDeviceGroups')));
  state.deviceGroups.forEach(group => {
    const item = element('div', 'device-group-card');
    const heading = element('div', 'group-row');
    heading.append(element('strong', '', group.name || '—'));
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteDeviceGroup = group.id;
    remove.disabled = state.busy;
    heading.append(remove);
    item.append(heading);
    const members = state.deviceGroupMemberships.filter(member => member.group_id === group.id);
    const assigned = element('div', 'membership-list');
    if (!members.length) assigned.append(element('small', 'empty-state', t('noAssignedDevices')));
    members.forEach(member => {
      const device = state.deviceGroupDevices.find(value => value.id === member.device_id);
      const row = element('div', 'membership-row');
      row.append(element('span', '', device?.name || device?.uuid || member.device_id));
      const detach = button('', t('delete'), 'secondary');
      detach.dataset.removeDeviceGroup = group.id;
      detach.dataset.deviceId = member.device_id;
      detach.disabled = state.busy;
      row.append(detach);
      assigned.append(row);
    });
    item.append(assigned);
    const available = state.deviceGroupDevices.filter(device => !members.some(member => member.device_id === device.id));
    if (available.length) {
      const assign = element('div', 'inline-form');
      const select = document.createElement('select');
      select.id = `device-group-select-${group.id}`;
      available.forEach(device => {
        const option = document.createElement('option');
        option.value = device.id;
        option.textContent = device.name || device.uuid || device.id;
        select.append(option);
      });
      select.disabled = state.busy;
      assign.append(select);
      const add = button('', t('addDevice'), 'primary');
      add.dataset.addDeviceGroup = group.id;
      add.disabled = state.busy;
      assign.append(add);
      item.append(assign);
    }
    list.append(item);
  });
  view.append(list);
  return view;
}

function ldapView() {
  const config = state.ldapDraft || state.ldap || {
    enabled: false,
    url: '',
    bind_dn: '',
    user_base_dn: '',
    user_filter: '(&(objectClass=person)(uid={username}))',
    username_attribute: 'uid',
    email_attribute: 'mail',
    use_tls: false,
    timeout_seconds: 5
  };
  const view = viewHeader(t('ldap'), [button('refresh-ldap', t('refresh'), 'secondary')]);
  const form = element('form', 'panel-form');
  form.id = 'ldap-form';
  form.append(checkboxField('ldap-enabled', t('enabled'), config.enabled));
  form.append(configField('ldap-url', t('ldapUrl'), config.url));
  form.append(configField('ldap-bind-dn', t('bindDn'), config.bind_dn));
  form.append(configField('ldap-bind-password', t('bindPassword'), config.bind_password || '', 'password'));
  form.append(configField('ldap-user-base-dn', t('userBaseDn'), config.user_base_dn));
  form.append(configField('ldap-user-filter', t('userFilter'), config.user_filter));
  form.append(configField('ldap-username-attribute', t('usernameAttribute'), config.username_attribute));
  form.append(configField('ldap-email-attribute', t('emailAttribute'), config.email_attribute));
  form.append(checkboxField('ldap-use-tls', t('useTls'), config.use_tls));
  form.append(configField('ldap-timeout', t('timeoutSeconds'), String(config.timeout_seconds), 'number'));
  form.append(element('p', 'empty-state', t('runtimeOnly')));
  const submit = button('save-ldap', state.busy ? t('saving') : t('save'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  view.append(form);
  return view;
}

function configField(id, label, value, type = 'text') {
  const wrapper = element('label', 'field');
  wrapper.htmlFor = id;
  wrapper.append(element('span', '', label));
  const input = document.createElement('input');
  input.id = id;
  input.name = id;
  input.type = type;
  input.value = value || '';
  input.disabled = state.busy;
  if (type === 'number') {
    input.min = '1';
    input.max = '60';
  }
  wrapper.append(input);
  return wrapper;
}

function checkboxField(id, label, checked) {
  const wrapper = element('label', 'field checkbox-field');
  const input = document.createElement('input');
  input.id = id;
  input.name = id;
  input.type = 'checkbox';
  input.checked = Boolean(checked);
  input.disabled = state.busy;
  wrapper.append(input);
  wrapper.append(element('span', '', label));
  return wrapper;
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
    const nextView = node.dataset.view;
    if (state.activeView === 'deviceGroups' && nextView !== 'deviceGroups') {
      state.deviceGroupsRequest += 1;
      state.busy = false;
    }
    if (state.activeView === 'devices' && nextView !== 'devices') {
      state.devicesRequest += 1;
      state.busy = false;
    }
    state.activeView = nextView;
    setNotice(null, null);
    render();
    if (state.activeView === 'addressBook') {
      await loadAddressBook();
      await loadTags();
    }
    if (state.activeView === 'devices') await loadDevices();
    if (state.activeView === 'groups') await loadGroups();
    if (state.activeView === 'deviceGroups') await loadDeviceGroups();
    if (state.activeView === 'users') await loadUsers();
    if (state.activeView === 'sessions') await loadSessions();
    if (state.activeView === 'ldap' && !state.ldap) await loadLdap();
  }));
  document.querySelector('#refresh-address-book')?.addEventListener('click', loadAddressBook);
  document.querySelector('#save-address-book')?.addEventListener('click', saveAddressBook);
  const tagForm = document.querySelector('#tag-form');
  tagForm?.addEventListener('submit', saveTag);
  tagForm?.addEventListener('input', event => {
    const data = new FormData(event.currentTarget);
    state.tagDraft = {
      name: String(data.get('name') || ''),
      color: String(data.get('color') || '')
    };
  });
  document.querySelectorAll('[data-delete-tag]').forEach(node => {
    node.addEventListener('click', () => deleteTag(node.dataset.deleteTag));
  });
  document.querySelector('#refresh-users')?.addEventListener('click', loadUsers);
  document.querySelector('#refresh-sessions')?.addEventListener('click', loadSessions);
  document.querySelectorAll('[data-revoke-session]').forEach(node => {
    node.addEventListener('click', () => revokeSession(node.dataset.revokeSession));
  });
  document.querySelector('#refresh-devices')?.addEventListener('click', loadDevices);
  document.querySelector('#refresh-groups')?.addEventListener('click', loadGroups);
  document.querySelector('#refresh-device-groups')?.addEventListener('click', loadDeviceGroups);
  const deviceGroupForm = document.querySelector('#device-group-form');
  deviceGroupForm?.addEventListener('submit', createDeviceGroup);
  deviceGroupForm?.addEventListener('input', event => {
    state.deviceGroupDraft = String(new FormData(event.currentTarget).get('name') || '');
  });
  document.querySelectorAll('[data-delete-device-group]').forEach(node => {
    node.addEventListener('click', () => deleteDeviceGroup(node.dataset.deleteDeviceGroup));
  });
  document.querySelectorAll('[data-add-device-group]').forEach(node => {
    node.addEventListener('click', () => addDeviceGroupMember(node.dataset.addDeviceGroup));
  });
  document.querySelectorAll('[data-remove-device-group]').forEach(node => {
    node.addEventListener('click', () => removeDeviceGroupMember(
      node.dataset.removeDeviceGroup,
      node.dataset.deviceId
    ));
  });
  document.querySelector('#refresh-ldap')?.addEventListener('click', () => loadLdap(true));
  const ldapForm = document.querySelector('#ldap-form');
  ldapForm?.addEventListener('submit', saveLdap);
  ldapForm?.addEventListener('input', preserveLdapDraft);
  ldapForm?.addEventListener('change', preserveLdapDraft);
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
    await loadBootstrapData();
  } catch (error) {
    clearSession();
    setNotice('error', error.message);
  }
  if (renderAfter) render();
}

async function loadBootstrapData() {
  const [serverResult, addressResult] = await Promise.allSettled([
    api('/api/server-config', { method: 'POST', body: '{}' }),
    api('/api/ab')
  ]);
  if (serverResult.status === 'fulfilled') {
    state.serverConfig = serverResult.value.data || serverResult.value;
  }
  if (addressResult.status === 'fulfilled') {
    const raw = addressResult.value.data ?? addressResult.value;
    try {
      const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
      state.addressBook = JSON.stringify(parsed, null, 2);
    } catch {
      setNotice('error', t('requestFailed'));
    }
  }
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

async function loadTags() {
  try {
    const result = await api('/api/ab/tags');
    const value = result.data || result;
    state.tags = Array.isArray(value) ? value.filter(tag => tag && typeof tag === 'object') : [];
  } catch (error) {
    setNotice('error', error.message);
  }
  render();
}

async function saveTag(event) {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const payload = {
    name: String(data.get('name') || '').trim(),
    color: String(data.get('color') || '').trim()
  };
  if (!payload.name) return;
  state.tagDraft = payload;
  state.busy = true;
  render();
  try {
    await api('/api/ab/tags', { method: 'POST', body: JSON.stringify(payload) });
    state.tagDraft = { name: '', color: '' };
    await loadTags();
    setNotice('success', t('saved'));
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function deleteTag(name) {
  state.busy = true;
  render();
  try {
    await api('/api/ab/tags/delete', {
      method: 'POST',
      body: JSON.stringify({ name })
    });
    await loadTags();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadDevices() {
  const request = ++state.devicesRequest;
  state.busy = true;
  render();
  try {
    const result = await api('/api/devices');
    if (request !== state.devicesRequest || state.activeView !== 'devices') return;
    const value = result.data?.list || result.data || result.list || result;
    state.devices = Array.isArray(value) ? value : [];
    setNotice(null, null);
  } catch (error) {
    if (request === state.devicesRequest && state.user) setNotice('error', error.message);
  } finally {
    if (request === state.devicesRequest && state.activeView === 'devices') {
      state.busy = false;
      render();
    } else if (!state.user) {
      state.busy = false;
      render();
    }
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

async function loadDeviceGroups() {
  const request = ++state.deviceGroupsRequest;
  state.busy = true;
  render();
  try {
    const [result, deviceResult] = await Promise.all([
      api('/api/device-groups'),
      api('/api/devices')
    ]);
    if (request !== state.deviceGroupsRequest) return;
    const value = result.data?.list || result.data || result.list || result;
    const devices = deviceResult.data?.list || deviceResult.data || deviceResult.list || deviceResult;
    state.deviceGroups = Array.isArray(value) ? value : [];
    state.deviceGroupMemberships = Array.isArray(result.memberships) ? result.memberships : [];
    state.deviceGroupDevices = Array.isArray(devices) ? devices : [];
    setNotice(null, null);
  } catch (error) {
    if (request === state.deviceGroupsRequest) setNotice('error', error.message);
  } finally {
    if (request === state.deviceGroupsRequest || !state.user) {
      state.busy = false;
      render();
    }
  }
}

async function createDeviceGroup(event) {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const name = String(data.get('name') || '').trim();
  if (!name) return;
  state.deviceGroupDraft = name;
  state.busy = true;
  render();
  try {
    await api('/api/device-groups', {
      method: 'POST',
      body: JSON.stringify({ name })
    });
    state.deviceGroupDraft = '';
    if (state.activeView === 'deviceGroups') await loadDeviceGroups();
  } catch (error) {
    if (state.activeView === 'deviceGroups') setNotice('error', error.message);
  } finally {
    if (state.activeView === 'deviceGroups' || !state.user) {
      state.busy = false;
      render();
    }
  }
}

async function deleteDeviceGroup(id) {
  state.busy = true;
  render();
  try {
    await api('/api/device-groups/delete', {
      method: 'POST',
      body: JSON.stringify({ id })
    });
    if (state.activeView === 'deviceGroups') await loadDeviceGroups();
  } catch (error) {
    if (state.activeView === 'deviceGroups') setNotice('error', error.message);
  } finally {
    if (state.activeView === 'deviceGroups' || !state.user) {
      state.busy = false;
      render();
    }
  }
}

async function addDeviceGroupMember(groupId) {
  const select = document.querySelector(`#device-group-select-${groupId}`);
  const deviceId = select?.value || '';
  if (!deviceId) return;
  state.busy = true;
  render();
  try {
    await api('/api/device-groups/members', {
      method: 'POST',
      body: JSON.stringify({ group_id: groupId, device_id: deviceId })
    });
    if (state.activeView === 'deviceGroups') await loadDeviceGroups();
  } catch (error) {
    if (state.activeView === 'deviceGroups') setNotice('error', error.message);
  } finally {
    if (state.activeView === 'deviceGroups' || !state.user) {
      state.busy = false;
      render();
    }
  }
}

async function removeDeviceGroupMember(groupId, deviceId) {
  state.busy = true;
  render();
  try {
    await api('/api/device-groups/members/delete', {
      method: 'POST',
      body: JSON.stringify({ group_id: groupId, device_id: deviceId })
    });
    if (state.activeView === 'deviceGroups') await loadDeviceGroups();
  } catch (error) {
    if (state.activeView === 'deviceGroups') setNotice('error', error.message);
  } finally {
    if (state.activeView === 'deviceGroups' || !state.user) {
      state.busy = false;
      render();
    }
  }
}

async function loadSessions() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/admin/session/list');
    const value = result.data?.list || result.data || result.list || result;
    state.sessions = Array.isArray(value) ? value : [];
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function revokeSession(id) {
  state.busy = true;
  render();
  try {
    await api('/api/admin/session/revoke', {
      method: 'POST',
      body: JSON.stringify({ id })
    });
    await loadSessions();
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

function ldapPayload(form) {
  const data = new FormData(form);
  return {
    enabled: data.has('ldap-enabled'),
    url: String(data.get('ldap-url') || ''),
    bind_dn: String(data.get('ldap-bind-dn') || ''),
    bind_password: String(data.get('ldap-bind-password') || ''),
    user_base_dn: String(data.get('ldap-user-base-dn') || ''),
    user_filter: String(data.get('ldap-user-filter') || ''),
    username_attribute: String(data.get('ldap-username-attribute') || ''),
    email_attribute: String(data.get('ldap-email-attribute') || ''),
    use_tls: data.has('ldap-use-tls'),
    timeout_seconds: Number(data.get('ldap-timeout') || 5)
  };
}

function preserveLdapDraft(event) {
  state.ldapDraft = ldapPayload(event.currentTarget);
}

async function loadLdap(discardDraft = false) {
  state.busy = true;
  render();
  try {
    state.ldap = await api('/api/admin/ldap/config');
    if (discardDraft || !state.ldapDraft) {
      state.ldapDraft = { ...state.ldap, bind_password: '' };
    }
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function saveLdap(event) {
  event.preventDefault();
  const payload = ldapPayload(event.currentTarget);
  state.ldapDraft = payload;
  state.busy = true;
  render();
  try {
    state.ldap = await api('/api/admin/ldap/config', {
      method: 'POST',
      body: JSON.stringify(payload)
    });
    state.ldapDraft = { ...state.ldap, bind_password: '' };
    setNotice('success', t('ldapSaved'));
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
  state.sessions = [];
  state.devices = [];
  state.groups = [];
  state.tags = [];
  state.tagDraft = { name: '', color: '' };
  state.deviceGroups = [];
  state.deviceGroupMemberships = [];
  state.deviceGroupDevices = [];
  state.deviceGroupDraft = '';
  state.deviceGroupsRequest += 1;
  state.devicesRequest += 1;
  state.ldap = null;
  state.ldapDraft = null;
  state.serverConfig = null;
  state.activeView = 'profile';
  sessionStorage.removeItem(TOKEN_KEY);
}

render();
if (state.token) loadCurrentUser();
