import './style.css';

const API_BASE = (import.meta.env.VITE_API_BASE || '').replace(/\/$/, '');
const TOKEN_KEY = 'rustdesk_api_token';
sessionStorage.removeItem(TOKEN_KEY);
localStorage.removeItem(TOKEN_KEY);
const LOCALE_KEY = 'rustdesk_api_locale';

const messages = {
  'zh-CN': {
    brand: 'RustDesk API',
    login: '登录',
    register: '注册',
    providerReadOnly: '环境变量（只读）',
    username: '用户名',
    email: '邮箱',
    password: '密码',
    signingIn: '正在登录...',
    registering: '正在注册...',
    profile: '当前用户',
    addressBook: '地址簿',
    peerId: '设备 ID',
    hostname: '主机名',
    alias: '别名',
    forceRelay: '始终中继',
    addPeer: '添加设备',
    updatePeer: '更新设备',
    noPeers: '暂无地址簿设备',
    users: '用户管理',
    sessions: '会话管理',
    oauth: 'OAuth 配置',
    oauthSignIn: '使用 OAuth 登录',
    provider: '提供商',
    authorizationEndpoint: '授权端点',
    userInfoEndpoint: '用户信息端点',
    redirectConfigured: '回调地址已配置',
    addProvider: '添加提供商',
     editProvider: '编辑提供商',
     providerName: '名称',
     clientId: 'Client ID',
     clientSecret: 'Client Secret（留空则保持不变）',
     tokenEndpoint: '令牌端点',
     issuerEndpoint: 'Issuer（OIDC 必填）',
     jwksEndpoint: 'JWKS（OIDC 必填）',
     scopes: 'Scopes',
     providerEnabled: '已启用',
     providerDisabled: '已禁用',
     saveProvider: '保存提供商',
     providerSaved: 'OAuth 提供商已保存',
     providerDeleted: 'OAuth 提供商已删除',
     providerToggled: 'OAuth 提供商状态已更新',
    sessionId: '会话 ID',
    noSessions: '暂无会话',
    devices: '设备',
    deviceRegistry: '已登记设备与未认证报告',
    untrustedReport: '未认证报告，仅供参考',
    verifiedBinding: '已核验归属',
    pendingBinding: '待核验归属',
    fingerprint: '公钥指纹（从设备独立核对后填写）',
    bindDevice: '确认绑定',
    unbindDevice: '解除绑定',
    online: '在线',
    offline: '离线',
    groups: '用户组',
    deviceGroups: '设备组',
    groupName: '组名称',
    create: '创建',
    delete: '删除',
    cancel: '取消',
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
     createUser: '创建用户',
     creatingUser: '正在创建...',
     actions: '操作',
     disable: '禁用',
     enable: '启用',
     remove: '删除',
    noDevices: '暂无设备',
    deleteDevice: '删除设备',
    noGroups: '暂无用户组',
    noGroupMembers: '暂无成员',
    addMember: '添加成员',
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
    tagColor: '颜色（#RRGGBB 或 #RRGGBBAA）',
    editTag: '编辑标签',
    noTags: '暂无标签',
    language: 'English',
    loading: '加载中...',
    signedInAs: '已登录账户',
    cookieUnavailable: 'Cookie 会话不可用，请允许站点 Cookie 或使用同源反向代理',
    apiUnavailable: '无法连接 API 服务'
  },
  'en-US': {
    brand: 'RustDesk API',
    login: 'Sign in',
    register: 'Register',
    providerReadOnly: 'Environment (read only)',
    username: 'Username',
    email: 'Email',
    password: 'Password',
    signingIn: 'Signing in...',
    registering: 'Registering...',
    profile: 'Current user',
    addressBook: 'Address book',
    peerId: 'Device ID',
    hostname: 'Hostname',
    alias: 'Alias',
    forceRelay: 'Always relay',
    addPeer: 'Add device',
    updatePeer: 'Update device',
    noPeers: 'No address-book devices',
    users: 'User management',
    sessions: 'Session management',
    oauth: 'OAuth configuration',
    oauthSignIn: 'Sign in with OAuth',
    provider: 'Provider',
    authorizationEndpoint: 'Authorization endpoint',
    userInfoEndpoint: 'Userinfo endpoint',
    redirectConfigured: 'Callback configured',
    noProviders: 'No enabled providers',
     addProvider: 'Add provider',
     editProvider: 'Edit provider',
     providerName: 'Name',
     clientId: 'Client ID',
     clientSecret: 'Client secret (leave blank to keep)',
     tokenEndpoint: 'Token endpoint',
     issuerEndpoint: 'Issuer (required for OIDC)',
     jwksEndpoint: 'JWKS (required for OIDC)',
     scopes: 'Scopes',
     providerEnabled: 'Enabled',
     providerDisabled: 'Disabled',
     saveProvider: 'Save provider',
     providerSaved: 'OAuth provider saved',
     providerDeleted: 'OAuth provider deleted',
     providerToggled: 'OAuth provider status updated',
    sessionId: 'Session ID',
    noSessions: 'No sessions found',
    devices: 'Devices',
    deviceRegistry: 'Registered devices and unsigned reports',
    untrustedReport: 'Unsigned report, for reference only',
    verifiedBinding: 'Verified ownership',
    pendingBinding: 'Ownership pending verification',
    fingerprint: 'Public key fingerprint (verify independently on device)',
    bindDevice: 'Confirm binding',
    unbindDevice: 'Unbind',
    online: 'Online',
    offline: 'Offline',
    groups: 'User groups',
    deviceGroups: 'Device groups',
    groupName: 'Group name',
    create: 'Create',
    delete: 'Delete',
    cancel: 'Cancel',
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
     createUser: 'Create user',
     creatingUser: 'Creating...',
     actions: 'Actions',
     disable: 'Disable',
     enable: 'Enable',
     remove: 'Delete',
    noDevices: 'No devices found',
    deleteDevice: 'Delete device',
    noGroups: 'No groups found',
    noGroupMembers: 'No members found',
    addMember: 'Add member',
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
    tagColor: 'Color (#RRGGBB or #RRGGBBAA)',
    editTag: 'Edit tag',
    noTags: 'No tags found',
    language: '简体中文',
    loading: 'Loading...',
    signedInAs: 'Signed in as',
    cookieUnavailable: 'Cookie session unavailable. Allow site cookies or use a same-origin reverse proxy.',
    apiUnavailable: 'Unable to reach the API server'
  }
};

const state = {
  locale: localStorage.getItem(LOCALE_KEY) || 'zh-CN',
  csrfToken: '',
  cookieSession: false,
  user: null,
  activeView: 'profile',
  authMode: 'login',
  addressBook: '{\n  "peers": [],\n  "tags": [],\n  "tag_colors": "{}"\n}',
  addressBookEntries: [],
  addressBookRevision: null,
  addressBookDraft: {
    id: '',
    peer_id: '',
    username: '',
    hostname: '',
    alias: '',
    platform: '',
    tags: '',
    force_always_relay: false
  },
  tags: [],
  tagDraft: { name: '', color: '' },
  serverConfig: null,
  users: [],
  userDraft: { username: '', email: '', password: '' },
  sessions: [],
  oauthProviders: [],
  oauthLoginProviders: [],
  oauthRedirectConfigured: false,
  oauthDraft: null,
  devices: [],
  registeredDevices: [],
  registryUsers: [],
  registrySearch: '',
  groups: [],
  groupMemberships: [],
  groupUsers: [],
  groupDraft: '',
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
  shell.append(state.user && state.cookieSession ? workspace() : authView());
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
  if (state.user && state.cookieSession) actions.append(button('logout', t('logout'), 'secondary'));
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
  if (state.oauthLoginProviders.length) {
    const oauth = element('div', 'oauth-login');
    oauth.append(element('small', '', t('oauthSignIn')));
    state.oauthLoginProviders.forEach(provider => {
      const action = button(`oauth-login-${provider}`, provider, 'secondary');
      action.dataset.oauthProvider = provider;
      oauth.append(action);
    });
    panel.append(oauth);
  }
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
    nav.append(navButton('oauth', t('oauth')));
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
  else if (state.activeView === 'oauth' && state.user.is_admin) content.append(oauthView());
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
  view.append(peerEditor());
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  [t('peerId'), t('alias'), t('hostname'), t('platform'), t('tags'), t('actions')]
    .forEach(label => headRow.append(element('th', '', label)));
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (!state.addressBookEntries.length) {
    const row = document.createElement('tr');
    const cell = element('td', 'empty-cell', t('noPeers'));
    cell.colSpan = 6;
    row.append(cell);
    body.append(row);
  } else {
    state.addressBookEntries.forEach(peer => {
      const row = document.createElement('tr');
      row.append(element('td', 'strong-cell', peer.peerId || peer.id || '—'));
      row.append(element('td', '', peer.alias || '—'));
      row.append(element('td', '', peer.hostname || '—'));
      row.append(element('td', '', peer.platform || '—'));
      const tags = Array.isArray(peer.tags) ? peer.tags.join(', ') : '—';
      row.append(element('td', '', tags));
      const actions = element('div', 'button-group');
      const edit = button('', t('updatePeer'), 'secondary');
      edit.dataset.editPeer = peer.id || peer.peerId || '';
      edit.disabled = state.busy;
      actions.append(edit);
      const remove = button('', t('delete'), 'secondary');
      remove.dataset.deletePeer = peer.id || peer.peerId || '';
      remove.disabled = state.busy;
      actions.append(remove);
      const cell = document.createElement('td');
      cell.append(actions);
      row.append(cell);
      body.append(row);
    });
  }
  table.append(body);
  tableWrap.append(table);
  view.append(tableWrap);

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

function peerEditor() {
  const panel = element('section', 'panel-form');
  const form = document.createElement('form');
  form.id = 'peer-form';
  const draft = state.addressBookDraft;
  form.append(configField('peer-id', t('peerId'), draft.peer_id));
  form.append(configField('peer-username', t('username'), draft.username));
  form.append(configField('peer-hostname', t('hostname'), draft.hostname));
  form.append(configField('peer-alias', t('alias'), draft.alias));
  form.append(configField('peer-platform', t('platform'), draft.platform));
  form.append(configField('peer-tags', t('tags'), draft.tags));
  form.append(checkboxField('peer-relay', t('forceRelay'), draft.force_always_relay));
  const actions = element('div', 'button-group');
  const submit = button('save-peer', state.busy
    ? t('saving')
    : (draft.id ? t('updatePeer') : t('addPeer')), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  actions.append(submit);
  if (draft.id) {
    const clear = button('clear-peer', t('cancel'), 'secondary');
    clear.type = 'button';
    clear.disabled = state.busy;
    actions.append(clear);
  }
  form.append(actions);
  panel.append(form);
  return panel;
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
  color.maxLength = 9;
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
    const edit = button('', t('editTag'), 'secondary');
    edit.dataset.editTag = tag.name || '';
    edit.disabled = state.busy;
    row.append(edit);
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
  const form = element('form', 'inline-form');
  form.id = 'admin-user-form';
  form.append(configField('admin-user-name', t('username'), state.userDraft.username));
  form.append(configField('admin-user-email', t('email'), state.userDraft.email, 'email'));
  form.append(configField('admin-user-password', t('password'), state.userDraft.password, 'password'));
  const submit = button('create-admin-user', state.busy ? t('creatingUser') : t('createUser'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  view.append(form);
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  [t('account'), t('email'), t('role'), t('status'), t('createdAt'), t('actions')].forEach(label => {
    headRow.append(element('th', '', label));
  });
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (!state.users.length) {
    const row = document.createElement('tr');
    const cell = element('td', 'empty-cell', t('noUsers'));
    cell.colSpan = 6;
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
      const actions = element('div', 'button-group');
      const toggle = button('', Number(user.status) === 1 ? t('disable') : t('enable'), 'secondary');
      toggle.dataset.toggleUser = user.id;
      toggle.dataset.userStatus = Number(user.status) === 1 ? '0' : '1';
      toggle.disabled = state.busy;
      actions.append(toggle);
      const remove = button('', t('remove'), 'secondary');
      remove.dataset.deleteUser = user.id;
      remove.disabled = state.busy;
      actions.append(remove);
      const cell = document.createElement('td');
      cell.append(actions);
      row.append(cell);
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

function oauthView() {
  const editing = state.oauthDraft?.id;
  const view = viewHeader(t('oauth'), [button('refresh-oauth', t('refresh'), 'secondary')]);
  view.append(element('p', 'empty-state', `${t('redirectConfigured')}: ${state.oauthRedirectConfigured ? t('enabled') : t('disabled')}`));
  const form = element('form', 'panel-form oauth-provider-form');
  form.id = 'oauth-provider-form';
  const draft = state.oauthDraft || {};
  const kindLabel = element('label', 'field');
  kindLabel.append(element('span', '', 'OAuth / OIDC'));
  const kind = document.createElement('select'); kind.id = 'oauth-kind'; kind.name = 'kind';
  for (const value of ['oauth2', 'oidc']) { const option = document.createElement('option'); option.value = value; option.textContent = value === 'oidc' ? 'OIDC' : 'OAuth2'; kind.append(option); }
  kind.value = draft.kind || 'oidc';
  kind.disabled = Boolean(editing); kindLabel.append(kind); form.append(kindLabel);
  [['name', t('providerName'), 'text'], ['client_id', t('clientId'), 'text'], ['client_secret', t('clientSecret'), 'password'], ['authorization_url', t('authorizationEndpoint'), 'url'], ['token_url', t('tokenEndpoint'), 'url'], ['userinfo_url', t('userInfoEndpoint'), 'url'], ['issuer_url', t('issuerEndpoint'), 'url'], ['jwks_url', t('jwksEndpoint'), 'url'], ['scopes', t('scopes'), 'text']].forEach(([key, label, type]) => {
    const input = field(`oauth-${key}`, label, type, 'off', key === 'name' || key === 'client_id' || key === 'authorization_url' || key === 'token_url' || key === 'userinfo_url');
    input.querySelector('input').name = key;
    input.querySelector('input').value = draft[key] || '';
    input.querySelector('input').readOnly = Boolean(editing) && !['client_secret', 'scopes'].includes(key);
    if (key === 'client_secret') input.querySelector('input').required = !editing;
    form.append(input);
  });
  const enabled = element('label', 'checkbox-field');
  const checkbox = document.createElement('input'); checkbox.type = 'checkbox'; checkbox.name = 'enabled'; checkbox.checked = draft.enabled !== false;
  enabled.append(checkbox, element('span', '', t('enabled'))); form.append(enabled);
  const actions = element('div', 'button-group');
  const saveButton = button('save-oauth-provider', editing ? t('editProvider') : t('addProvider'), 'primary'); saveButton.type = 'submit'; actions.append(saveButton);
  if (editing) actions.append(button('cancel-oauth-provider', t('cancel'), 'secondary'));
  form.append(actions); view.append(form);
  const tableWrap = element('div', 'table-wrap'); const table = document.createElement('table');
  const head = document.createElement('thead'); const row = document.createElement('tr');
  [t('provider'), t('authorizationEndpoint'), t('userInfoEndpoint'), t('status'), t('actions')].forEach(label => row.append(element('th', '', label))); head.append(row); table.append(head);
  const body = document.createElement('tbody');
  state.oauthProviders.forEach(provider => {
    const item = document.createElement('tr'); item.append(element('td', 'strong-cell', provider.name || '—')); item.append(element('td', '', provider.authorization_url || '—')); item.append(element('td', '', provider.userinfo_url || '—')); item.append(element('td', '', provider.read_only ? t('providerReadOnly') : (provider.enabled === false ? t('providerDisabled') : t('providerEnabled'))));
    const cell = document.createElement('td'); const edit = button('', t('editProvider'), 'secondary'); edit.dataset.editOauth = provider.id || provider.name; edit.disabled = Boolean(provider.read_only); cell.append(edit); const toggle = button('', provider.enabled === false ? t('enable') : t('disable'), 'secondary'); toggle.dataset.toggleOauth = provider.id || provider.name; toggle.dataset.oauthEnabled = provider.enabled === false ? 'false' : 'true'; toggle.disabled = Boolean(provider.read_only); cell.append(toggle); const remove = button('', t('delete'), 'secondary'); remove.dataset.deleteOauth = provider.id || provider.name; remove.disabled = Boolean(provider.read_only); cell.append(remove); item.append(cell); body.append(item);
  });
  if (!state.oauthProviders.length) { const empty = document.createElement('tr'); const cell = element('td', 'empty-cell', t('noProviders')); cell.colSpan = 5; empty.append(cell); body.append(empty); }
  table.append(body); tableWrap.append(table); view.append(tableWrap); return view;
}

function devicesView() {
  const refresh = button('refresh-devices', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  const view = viewHeader(t('devices'), [refresh]);
  const tableWrap = element('div', 'table-wrap');
  const table = document.createElement('table');
  const head = document.createElement('thead');
  const headRow = document.createElement('tr');
  const columns = [t('account'), t('uuid'), t('platform'), t('status')];
  if (state.user.is_admin) columns.push(t('actions'));
  columns.forEach(label => headRow.append(element('th', '', label)));
  head.append(headRow);
  table.append(head);
  const body = document.createElement('tbody');
  if (!state.devices.length) {
    const row = document.createElement('tr');
    const cell = element('td', 'empty-cell', t('noDevices'));
    cell.colSpan = columns.length;
    row.append(cell);
    body.append(row);
  } else {
    state.devices.forEach(device => {
      const row = document.createElement('tr');
      row.append(element('td', 'strong-cell', device.name || device.peer_id || device.id || '—'));
      row.append(element('td', '', device.uuid || '—'));
      row.append(element('td', '', [device.os, device.device_type].filter(Boolean).join(' / ') || '—'));
      row.append(element('td', '', `${Number(device.status) === 1 ? t('enabled') : t('disabled')} / ${device.verified ? t('verifiedBinding') : t('pendingBinding')}`));
      if (state.user.is_admin) {
        const remove = button('', t('deleteDevice'), 'secondary');
        remove.dataset.deleteDevice = device.id;
        remove.disabled = state.busy;
        const cell = document.createElement('td');
        cell.append(remove);
        row.append(cell);
      }
      body.append(row);
    });
  }
  table.append(body);
  tableWrap.append(table);
  view.append(tableWrap);
  if (state.user.is_admin) view.append(deviceRegistryView());
  return view;
}

function deviceRegistryView() {
  const section = element('section', 'tags-panel');
  section.append(element('h2', '', t('deviceRegistry')));
  const search = element('form', 'inline-form'); search.id = 'registry-search';
  const query = document.createElement('input'); query.name = 'peer_id'; query.placeholder = t('peerId'); query.value = state.registrySearch;
  const searchButton = button('', t('refresh'), 'secondary'); searchButton.type = 'submit'; search.append(query, searchButton); section.append(search);
  for (const peer of state.registeredDevices) {
    const panel = element('section', 'panel-form'); panel.dataset.registryPeer = peer.peer_id;
    panel.append(element('strong', '', `${peer.peer_id} / ${peer.online ? t('online') : t('offline')}`));
    panel.append(element('p', '', `UUID: ${peer.uuid}`));
    panel.append(element('code', '', peer.pk_fingerprint));
    panel.append(element('p', '', `${t('untrustedReport')}: ${peer.untrusted_sysinfo?.hostname || peer.untrusted_sysinfo?.device_name || '—'}`));
    panel.append(element('p', '', peer.verified ? t('verifiedBinding') : t('pendingBinding')));
    const form = element('form', 'inline-form'); form.dataset.bindPeer = peer.peer_id;
    const owner = document.createElement('select'); owner.name = 'user_id'; owner.required = true;
    const placeholder = document.createElement('option'); placeholder.value = ''; placeholder.textContent = t('account'); owner.append(placeholder);
    for (const user of state.registryUsers) { const option = document.createElement('option'); option.value = user.id; option.textContent = user.name || user.id; owner.append(option); }
    owner.value = peer.owner_id || '';
    const fingerprint = document.createElement('input'); fingerprint.name = 'pk_fingerprint'; fingerprint.required = true; fingerprint.placeholder = t('fingerprint');
    const submit = button('', t('bindDevice'), 'primary'); submit.type = 'submit'; submit.disabled = state.busy;
    form.append(owner, fingerprint, submit); panel.append(form);
    if (peer.device_id && peer.verified) { const unbind = button('', t('unbindDevice'), 'secondary'); unbind.dataset.unbindDevice = peer.device_id; panel.append(unbind); }
    section.append(panel);
  }
  return section;
}

function groupsView() {
  const view = viewHeader(t('groups'), [button('refresh-groups', t('refresh'), 'secondary')]);
  const form = element('form', 'inline-form');
  form.id = 'group-form';
  const input = document.createElement('input');
  input.name = 'name';
  input.placeholder = t('groupName');
  input.maxLength = 128;
  input.required = true;
  input.value = state.groupDraft;
  input.disabled = state.busy;
  form.append(input);
  const submit = button('create-group', t('create'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(submit);
  view.append(form);

  const list = element('div', 'group-list');
  if (!state.groups.length) list.append(element('p', 'empty-state', t('noGroups')));
  state.groups.forEach(group => {
    const item = element('section', 'device-group-card');
    const heading = element('div', 'group-row');
    heading.append(element('strong', '', group.name || '—'));
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteGroup = group.id;
    remove.disabled = state.busy;
    heading.append(remove);
    item.append(heading);

    const members = state.groupMemberships.filter(member => member.group_id === group.id);
    const memberList = element('div', 'membership-list');
    if (!members.length) memberList.append(element('small', 'empty-state', t('noGroupMembers')));
    members.forEach(member => {
      const user = state.groupUsers.find(value => value.id === member.user_id);
      const row = element('div', 'membership-row');
      row.append(element('span', '', user?.name || user?.username || member.user_id));
      const detach = button('', t('delete'), 'secondary');
      detach.dataset.removeGroup = group.id;
      detach.dataset.userId = member.user_id;
      detach.disabled = state.busy;
      row.append(detach);
      memberList.append(row);
    });
    item.append(memberList);

    const available = state.groupUsers.filter(user => !members.some(member => member.user_id === user.id));
    if (available.length) {
      const assign = element('div', 'inline-form');
      const select = document.createElement('select');
      select.id = `group-select-${group.id}`;
      available.forEach(user => {
        const option = document.createElement('option');
        option.value = user.id;
        option.textContent = user.name || user.username || user.id;
        select.append(option);
      });
      select.disabled = state.busy;
      assign.append(select);
      const add = button('', t('addMember'), 'primary');
      add.dataset.addGroup = group.id;
      add.disabled = state.busy;
      assign.append(add);
      item.append(assign);
    }
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
  document.querySelectorAll('[data-oauth-provider]').forEach(node => {
    node.addEventListener('click', () => {
      const params = new URLSearchParams({
        provider: node.dataset.oauthProvider || '',
        id: 'web',
        uuid: 'web'
      });
      window.location.assign(`${API_BASE}/api/oidc/auth?${params.toString()}`);
    });
  });
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
    if (state.activeView === 'oauth') await loadOauthProviders();
    if (state.activeView === 'ldap' && !state.ldap) await loadLdap();
  }));
  document.querySelector('#refresh-address-book')?.addEventListener('click', loadAddressBook);
  document.querySelector('#save-address-book')?.addEventListener('click', saveAddressBook);
  document.querySelector('#peer-form')?.addEventListener('submit', savePeer);
  document.querySelector('#peer-form')?.addEventListener('input', preservePeerDraft);
  document.querySelector('#peer-form')?.addEventListener('change', preservePeerDraft);
  document.querySelector('#clear-peer')?.addEventListener('click', clearPeerDraft);
  document.querySelectorAll('[data-edit-peer]').forEach(node => {
    node.addEventListener('click', () => editPeer(node.dataset.editPeer));
  });
  document.querySelectorAll('[data-delete-peer]').forEach(node => {
    node.addEventListener('click', () => deletePeer(node.dataset.deletePeer));
  });
  const tagForm = document.querySelector('#tag-form');
  tagForm?.addEventListener('submit', saveTag);
  tagForm?.addEventListener('input', event => {
    const data = new FormData(event.currentTarget);
    state.tagDraft = {
      ...state.tagDraft,
      revision: state.tagDraft.revision ?? state.addressBookRevision,
      name: String(data.get('name') || ''),
      color: String(data.get('color') || '')
    };
  });
  document.querySelectorAll('[data-edit-tag]').forEach(node => {
    node.addEventListener('click', () => {
      const tag = state.tags.find(tag => tag.name === node.dataset.editTag);
      if (tag) { state.tagDraft = { old_name: tag.name, name: tag.name, color: tag.color || '', revision: state.addressBookRevision }; render(); }
    });
  });
  document.querySelectorAll('[data-delete-tag]').forEach(node => {
    node.addEventListener('click', () => deleteTag(node.dataset.deleteTag));
  });
  document.querySelector('#refresh-users')?.addEventListener('click', loadUsers);
  document.querySelector('#admin-user-form')?.addEventListener('submit', createAdminUser);
  document.querySelector('#admin-user-form')?.addEventListener('input', preserveUserDraft);
  document.querySelectorAll('[data-toggle-user]').forEach(node => {
    node.addEventListener('click', () => updateUserStatus(node.dataset.toggleUser, node.dataset.userStatus));
  });
  document.querySelectorAll('[data-delete-user]').forEach(node => {
    node.addEventListener('click', () => deleteAdminUser(node.dataset.deleteUser));
  });
  document.querySelector('#refresh-sessions')?.addEventListener('click', loadSessions);
  document.querySelector('#refresh-oauth')?.addEventListener('click', loadOauthProviders);
  document.querySelector('#oauth-provider-form')?.addEventListener('submit', saveOauthProvider);
  document.querySelector('#cancel-oauth-provider')?.addEventListener('click', () => { state.oauthDraft = null; render(); });
  document.querySelectorAll('[data-edit-oauth]').forEach(node => node.addEventListener('click', () => editOauthProvider(node.dataset.editOauth)));
  document.querySelectorAll('[data-toggle-oauth]').forEach(node => node.addEventListener('click', () => toggleOauthProvider(node.dataset.toggleOauth, node.dataset.oauthEnabled === 'true')));
  document.querySelectorAll('[data-delete-oauth]').forEach(node => node.addEventListener('click', () => deleteOauthProvider(node.dataset.deleteOauth)));
  document.querySelectorAll('[data-revoke-session]').forEach(node => {
    node.addEventListener('click', () => revokeSession(node.dataset.revokeSession));
  });
  document.querySelector('#refresh-devices')?.addEventListener('click', loadDevices);
  document.querySelector('#registry-search')?.addEventListener('submit', event => { event.preventDefault(); state.registrySearch = new FormData(event.currentTarget).get('peer_id') || ''; loadDevices(); });
  document.querySelectorAll('[data-bind-peer]').forEach(form => form.addEventListener('submit', bindRegisteredDevice));
  document.querySelectorAll('[data-unbind-device]').forEach(node => node.addEventListener('click', () => unbindRegisteredDevice(node.dataset.unbindDevice)));
  document.querySelectorAll('[data-delete-device]').forEach(node => {
    node.addEventListener('click', () => deleteDevice(node.dataset.deleteDevice));
  });
  document.querySelector('#refresh-groups')?.addEventListener('click', loadGroups);
  document.querySelector('#group-form')?.addEventListener('submit', createGroup);
  document.querySelector('#group-form')?.addEventListener('input', event => {
    state.groupDraft = String(new FormData(event.currentTarget).get('name') || '');
  });
  document.querySelectorAll('[data-delete-group]').forEach(node => {
    node.addEventListener('click', () => deleteGroup(node.dataset.deleteGroup));
  });
  document.querySelectorAll('[data-add-group]').forEach(node => {
    node.addEventListener('click', () => addGroupMember(node.dataset.addGroup));
  });
  document.querySelectorAll('[data-remove-group]').forEach(node => {
    node.addEventListener('click', () => removeGroupMember(
      node.dataset.removeGroup,
      node.dataset.userId
    ));
  });
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
      await loadCurrentUser(false, true);
    }
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function loadLoginOptions() {
  try {
    const result = await api('/api/login-options', { authenticated: false });
    const options = Array.isArray(result) ? result : result?.data;
    state.oauthLoginProviders = (Array.isArray(options) ? options : [])
      .filter(option => typeof option === 'string' && option.startsWith('oidc/'))
      .map(option => option.slice('oidc/'.length))
      .filter(Boolean);
  } catch {
    state.oauthLoginProviders = [];
  }
}

async function loadCurrentUser(renderAfter = true, requireSession = false) {
  try {
    const result = await api('/api/session/csrf');
    state.csrfToken = result.csrf_token || '';
    if (!state.csrfToken) throw new Error(t('cookieUnavailable'));
    state.user = safeUser(result.user);
    if (!state.user || typeof state.user !== 'object') throw new Error(t('requestFailed'));
    state.cookieSession = true;
    await loadBootstrapData();
  } catch (error) {
    clearSession();
    if (requireSession) throw new Error(error.message === t('sessionExpired') ? t('cookieUnavailable') : error.message);
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
      state.addressBookRevision = addressResult.value.revision;
    } catch {
      setNotice('error', t('requestFailed'));
    }
  }
}

async function loadAddressBook() {
  state.busy = true;
  render();
  try {
    const [result, entriesResult] = await Promise.all([
      api('/api/ab'),
      api('/api/ab/peers')
    ]);
    if (!Number.isSafeInteger(result.revision) || result.revision !== entriesResult.revision) throw new Error('address_book_revision_conflict');
    state.addressBookRevision = result.revision;
    const raw = result.data ?? result;
    const parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
    state.addressBook = JSON.stringify(parsed, null, 2);
    const entries = entriesResult.data?.list || entriesResult.data || entriesResult.list || entriesResult;
    state.addressBookEntries = Array.isArray(entries) ? entries : [];
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
    await api('/api/ab', { method: 'POST', body: JSON.stringify({ data: JSON.stringify(parsed), revision: state.addressBookRevision }) });
    await loadAddressBook();
    await loadTags();
    setNotice('success', t('saved'));
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function savePeer(event) {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const payload = {
    id: state.addressBookDraft.id || undefined,
    revision: state.addressBookDraft.revision ?? state.addressBookRevision,
    peer_id: String(data.get('peer-id') || '').trim(),
    username: String(data.get('peer-username') || '').trim(),
    hostname: String(data.get('peer-hostname') || '').trim(),
    alias: String(data.get('peer-alias') || '').trim(),
    platform: String(data.get('peer-platform') || '').trim(),
    tags: String(data.get('peer-tags') || '')
      .split(',')
      .map(tag => tag.trim())
      .filter(Boolean),
    force_always_relay: event.currentTarget.querySelector('[name="peer-relay"]')?.checked || false
  };
  if (!payload.peer_id) return;
  state.addressBookDraft = { ...state.addressBookDraft, ...payload, tags: payload.tags.join(', ') };
  state.busy = true;
  render();
  try {
    await api('/api/ab/peer', { method: 'POST', body: JSON.stringify(payload) });
    clearPeerDraft();
    await loadAddressBook();
    setNotice('success', t('saved'));
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

function preservePeerDraft(event) {
  const data = new FormData(event.currentTarget);
  state.addressBookDraft = {
    ...state.addressBookDraft,
    revision: state.addressBookDraft.revision ?? state.addressBookRevision,
    peer_id: String(data.get('peer-id') || ''),
    username: String(data.get('peer-username') || ''),
    hostname: String(data.get('peer-hostname') || ''),
    alias: String(data.get('peer-alias') || ''),
    platform: String(data.get('peer-platform') || ''),
    tags: String(data.get('peer-tags') || ''),
    force_always_relay: event.currentTarget.querySelector('[name="peer-relay"]')?.checked || false
  };
}

function editPeer(id) {
  const peer = state.addressBookEntries.find(value => value.id === id || value.peerId === id);
  if (!peer) return;
  state.addressBookDraft = {
    id: peer.id || '',
    revision: state.addressBookRevision,
    peer_id: peer.peerId || peer.id || '',
    username: peer.username || '',
    hostname: peer.hostname || '',
    alias: peer.alias || '',
    platform: peer.platform || '',
    tags: Array.isArray(peer.tags) ? peer.tags.join(', ') : '',
    force_always_relay: Boolean(peer.forceAlwaysRelay)
  };
  render();
}

function clearPeerDraft() {
  state.addressBookDraft = {
    id: '',
    peer_id: '',
    username: '',
    hostname: '',
    alias: '',
    platform: '',
    tags: '',
    force_always_relay: false
  };
}

async function deletePeer(id) {
  if (!id) return;
  state.busy = true;
  render();
  try {
    await api(`/api/ab/peer/${encodeURIComponent(id)}?revision=${state.addressBookRevision}`, { method: 'DELETE' });
    clearPeerDraft();
    await loadAddressBook();
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
    if (result.revision !== state.addressBookRevision) throw new Error('address_book_revision_conflict');
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
    old_name: state.tagDraft.old_name,
    revision: state.tagDraft.revision ?? state.addressBookRevision,
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
    await loadAddressBook();
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
      body: JSON.stringify({ name, revision: state.addressBookRevision })
    });
    await loadAddressBook();
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
    const [result, registry, users] = await Promise.all([api('/api/devices'), state.user.is_admin ? api(`/api/admin/device/registry?peer_id=${encodeURIComponent(state.registrySearch)}`.replace(/\?peer_id=$/, '')) : null, state.user.is_admin ? api('/api/admin/user/list') : null]);
    if (request !== state.devicesRequest || state.activeView !== 'devices') return;
    const value = result.data?.list || result.data || result.list || result;
    state.devices = Array.isArray(value) ? value : [];
    state.registeredDevices = registry?.data || [];
    state.registryUsers = users?.data || [];
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

async function bindRegisteredDevice(event) {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.currentTarget)); data.peer_id = event.currentTarget.dataset.bindPeer;
  state.busy = true; render();
  try { await api('/api/admin/device/bind', { method: 'POST', body: JSON.stringify(data) }); await loadDevices(); }
  catch (error) { state.busy = false; setNotice('error', error.message); render(); }
}

async function unbindRegisteredDevice(id) {
  state.busy = true; render();
  try { await api('/api/admin/device/unbind', { method: 'POST', body: JSON.stringify({ id }) }); await loadDevices(); }
  catch (error) { state.busy = false; setNotice('error', error.message); render(); }
}

async function deleteDevice(id) {
  if (!id) return;
  state.busy = true;
  render();
  try {
    await api('/api/admin/device/delete', {
      method: 'POST',
      body: JSON.stringify({ id })
    });
    await loadDevices();
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
    const [result, userResult] = await Promise.all([
      api('/api/groups'),
      apiAllPages('/api/users')
    ]);
    const value = result.data?.list || result.data || result.list || result;
    const users = userResult.data?.list || userResult.data || userResult.list || userResult;
    state.groups = Array.isArray(value) ? value : [];
    state.groupMemberships = Array.isArray(result.memberships) ? result.memberships : [];
    if (!Array.isArray(users) || users.some(user => typeof user.id !== 'string' || !user.id)) throw new Error(t('requestFailed'));
    state.groupUsers = users;
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function createGroup(event) {
  event.preventDefault();
  const name = String(new FormData(event.currentTarget).get('name') || '').trim();
  if (!name) return;
  state.groupDraft = name;
  state.busy = true;
  render();
  try {
    await api('/api/groups', { method: 'POST', body: JSON.stringify({ name }) });
    state.groupDraft = '';
    await loadGroups();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function deleteGroup(id) {
  state.busy = true;
  render();
  try {
    await api('/api/groups/delete', { method: 'POST', body: JSON.stringify({ id }) });
    await loadGroups();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function addGroupMember(groupId) {
  const userId = document.querySelector(`#group-select-${groupId}`)?.value || '';
  if (!userId) return;
  state.busy = true;
  render();
  try {
    await api('/api/groups/members', {
      method: 'POST',
      body: JSON.stringify({ group_id: groupId, user_id: userId })
    });
    await loadGroups();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function removeGroupMember(groupId, userId) {
  state.busy = true;
  render();
  try {
    await api('/api/groups/members/delete', {
      method: 'POST',
      body: JSON.stringify({ group_id: groupId, user_id: userId })
    });
    await loadGroups();
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

async function loadOauthProviders() {
  state.busy = true;
  render();
  try {
    const result = await api('/api/admin/oauth/providers');
    state.oauthProviders = result.data || [];
    state.oauthRedirectConfigured = Boolean(result.redirect_url_configured);
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function saveOauthProvider(event) {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.currentTarget)); data.enabled = event.currentTarget.elements.enabled.checked; data.kind = event.currentTarget.elements.kind.value;
  state.busy = true; render();
  const editing = state.oauthDraft?.id; if (editing) data.id = editing;
  try { await api(editing ? '/api/admin/oauth/providers/update' : '/api/admin/oauth/providers', { method: 'POST', body: JSON.stringify(data) }); state.oauthDraft = null; setNotice('success', t('providerSaved')); await loadOauthProviders(); } catch (error) { setNotice('error', error.message); state.busy = false; render(); }
}

function editOauthProvider(id) {
  const provider = state.oauthProviders.find(item => String(item.id || item.name) === String(id));
  if (provider) { state.oauthDraft = { ...provider, client_secret: '' }; render(); }
}

async function toggleOauthProvider(id, enabled) {
  state.busy = true; render();
  try { await api('/api/admin/oauth/providers/toggle', { method: 'POST', body: JSON.stringify({ id, enabled: !enabled }) }); setNotice('success', t('providerToggled')); await loadOauthProviders(); } catch (error) { setNotice('error', error.message); state.busy = false; render(); }
}

async function deleteOauthProvider(id) {
  state.busy = true; render();
  try { await api('/api/admin/oauth/providers/delete', { method: 'POST', body: JSON.stringify({ id }) }); setNotice('success', t('providerDeleted')); await loadOauthProviders(); } catch (error) { setNotice('error', error.message); state.busy = false; render(); }
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

function preserveUserDraft(event) {
  const data = new FormData(event.currentTarget);
  state.userDraft = {
    username: String(data.get('admin-user-name') || ''),
    email: String(data.get('admin-user-email') || ''),
    password: String(data.get('admin-user-password') || '')
  };
}

async function createAdminUser(event) {
  event.preventDefault();
  const data = new FormData(event.currentTarget);
  const payload = {
    username: String(data.get('admin-user-name') || '').trim(),
    email: String(data.get('admin-user-email') || '').trim(),
    password: String(data.get('admin-user-password') || '')
  };
  state.userDraft = payload;
  if (!payload.username || !payload.password) return;
  state.busy = true;
  render();
  try {
    await api('/api/admin/user/create', { method: 'POST', body: JSON.stringify(payload) });
    state.userDraft = { username: '', email: '', password: '' };
    setNotice('success', t('saved'));
    await loadUsers();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function updateUserStatus(id, status) {
  state.busy = true;
  render();
  try {
    await api('/api/admin/user/update', {
      method: 'POST',
      body: JSON.stringify({ id, status: Number(status) })
    });
    await loadUsers();
  } catch (error) {
    setNotice('error', error.message);
  } finally {
    state.busy = false;
    render();
  }
}

async function deleteAdminUser(id) {
  if (!window.confirm(t('remove'))) return;
  state.busy = true;
  render();
  try {
    await api('/api/admin/user/delete', {
      method: 'POST',
      body: JSON.stringify({ id })
    });
    await loadUsers();
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
    clearSession();
    setNotice(null, null);
  } catch (error) {
    setNotice('error', error.message);
  }
  render();
}

async function apiAllPages(path) {
  const data = [];
  let total = 0;
  for (let current = 1; ; current += 1) {
    const separator = path.includes('?') ? '&' : '?';
    const page = await api(`${path}${separator}current=${current}&pageSize=100`);
    if (!Number.isSafeInteger(page?.total) || page.total < 0 || !Array.isArray(page.data)) throw new Error(t('requestFailed'));
    if (current === 1) total = page.total;
    if (page.total !== total || (page.data.length === 0 && data.length < total)) throw new Error(t('requestFailed'));
    data.push(...page.data);
    if (data.length >= total) return { total, data };
  }
}

async function api(path, options = {}) {
  const headers = { Accept: 'application/json', ...options.headers };
  if (options.body) headers['Content-Type'] = 'application/json';
  if (options.authenticated !== false && state.csrfToken && !['GET', 'HEAD'].includes((options.method || 'GET').toUpperCase())) headers['X-CSRF-Token'] = state.csrfToken;
  let response;
  try {
    response = await fetch(`${API_BASE}${path}`, { ...options, credentials: 'include', headers });
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
  state.csrfToken = '';
  state.cookieSession = false;
  state.user = null;
  state.users = [];
  state.userDraft = { username: '', email: '', password: '' };
  state.sessions = [];
  state.oauthProviders = [];
  state.oauthLoginProviders = [];
  state.oauthRedirectConfigured = false;
  state.devices = [];
  state.registeredDevices = [];
  state.registryUsers = [];
  state.registrySearch = '';
  state.groups = [];
  state.groupMemberships = [];
  state.groupUsers = [];
  state.groupDraft = '';
  state.tags = [];
  state.addressBookEntries = [];
  state.addressBookRevision = null;
  state.addressBookDraft = {
    id: '',
    peer_id: '',
    username: '',
    hostname: '',
    alias: '',
    platform: '',
    tags: '',
    force_always_relay: false
  };
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
Promise.all([loadCurrentUser(false), loadLoginOptions()]).then(() => render());
