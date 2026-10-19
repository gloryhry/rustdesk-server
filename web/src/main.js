import './style.css';
import { element, button, badge, pageHeader, card, formGrid, formSection, configField, checkboxField, selectField, field, detail, emptyState, tableShell, cell, emptyRow, tabs, tagChip } from './ui.js';
import { fingerprintCandidates, matchingFingerprint, chooseFingerprint } from './fingerprint.js';

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
    addressBookSubtitle: '管理个人地址簿中的设备、标签与原始数据',
    addressBookDevices: '设备列表',
    addressBookDevicesHint: '管理地址簿中的设备、别名和标签',
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
    oauthSubtitle: '配置 OAuth2/OIDC 登录提供商',
    oauthSignIn: '使用 OAuth 登录',
    provider: '提供商',
    providerList: '已配置提供商',
    authorizationEndpoint: '授权端点',
    userInfoEndpoint: '用户信息端点',
    redirectConfigured: '回调地址已配置',
    redirectNotConfigured: '回调地址未配置',
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
    noProviders: '暂无提供商',
    oauthTabIdentity: '基本信息',
    oauthTabClient: '客户端',
    oauthTabEndpoints: '端点',
    oauthTabScope: '范围与状态',
    oauthFormHint: '编辑时名称、类型和端点不可修改',
    requiredField: '请填写必填项',
    sessionId: '会话 ID',
    noSessions: '暂无会话',
    devices: '设备',
    devicesSubtitle: '查看已绑定设备与管理员登记记录',
    boundDevices: '已绑定设备',
    boundDevicesHint: '设备归属由管理员通过指纹绑定确认',
    deviceRegistry: '设备登记与绑定',
    deviceRegistrySubtitle: '登记记录来自 hbbs；未认证报告仅供参考',
    deviceRegistryEmpty: '暂无登记设备',
    deviceRegistryHint: '按 RustDesk ID 查询登记记录',
    untrustedReport: '未认证报告，仅供参考',
    verifiedBinding: '已核验归属',
    pendingBinding: '待核验归属',
    fingerprint: '设备指纹',
    fingerprintPlaceholder: '指纹: 8721 1612 8169 8f4f ...',
    fingerprintHelp: '支持带空格/换行的 64 位十六进制指纹；前端计算 SHA-256。也支持 sha256:<hex>。',
    fingerprintExample: '示例：8721 1612 8169 8f4f cd13 be20 1af1 d9b5 616c 8408 bc0f cbdc 3752 7e78 5b55 9dce',
    fingerprintStatusEmpty: '输入设备指纹后自动校验',
    fingerprintStatusReady: '格式有效，提交后由服务端校验',
    fingerprintStatusMatch: '指纹匹配',
    fingerprintStatusMismatch: '指纹不匹配，请核对设备显示值',
    fingerprintStatusInvalid: '请输入 64 位十六进制指纹或 sha256:<hex>',
    bindDevice: '确认绑定',
    unbindDevice: '解除绑定',
    bindAccount: '绑定账户',
    selectAccount: '选择账户',
    noAvailableAccounts: '暂无可用账户',
    boundTo: '已绑定账户',
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
    addDeviceTitle: '添加设备',
    editDeviceTitle: '编辑设备',
    peerEditorHint: '填写设备信息并选择标签',
    basicInfo: '基本信息',
    deviceInfo: '设备信息',
    advancedOptions: '高级选项',
    noAssignedDevices: '尚未分配设备',
    ldap: 'LDAP 配置',
    ldapSubtitle: '配置 LDAP 连接与用户搜索',
    ldapUrl: 'LDAP URL',
    bindDn: '绑定 DN',
    bindPassword: '绑定密码（留空则保持不变）',
    userBaseDn: '用户 Base DN',
    userFilter: '用户过滤器',
    usernameAttribute: '用户名属性',
    emailAttribute: '邮箱属性',
    useTls: '使用 StartTLS',
    timeoutSeconds: '超时（秒）',
    ldapTabConnection: '连接',
    ldapTabBind: '绑定',
    ldapTabSearch: '用户搜索',
    ldapStatus: '当前状态',
    ldapSettings: '连接与搜索配置',
    ldapConfigured: '已配置',
    ldapNotConfigured: '未配置',
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
    noMatchingDevices: '没有匹配的设备',
    deleteDevice: '删除设备',
    noGroups: '暂无用户组',
    noGroupMembers: '暂无成员',
    addMember: '添加成员',
    uuid: '设备 UUID',
    deviceUuid: '设备 UUID',
    rustdeskId: 'RustDesk ID',
    platform: '平台',
    deviceName: '设备名',
    registeredAt: '最近注册',
    search: '搜索',
    clear: '清空',
    searchDevices: '搜索设备 ID、别名、主机名或标签',
    deviceSearchPlaceholder: '搜索 UUID、RustDesk ID 或账户',
    allTags: '全部',
    clearFilters: '清除筛选',
    deviceCount: '共 {count} 台设备',
    deviceCountFiltered: '显示 {shown} / {total} 台设备',
    addTag: '添加',
    newTagPlaceholder: '输入新标签',
    noSelectedTags: '尚未选择标签',
    removeTag: '移除标签',
    tagTooLong: '标签名称不能超过 64 个字符',
    invalidPeerId: '请输入设备 ID',
    advancedJson: '高级：原始 JSON',
    advancedJsonHint: '直接编辑完整地址簿文档。修改后使用页面右上角的保存按钮。',
    jsonDraftBadge: '有未保存修改',
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
    tagsHint: '选择已有标签，或输入新标签后回车',
    tagName: '标签名称',
    tagColor: '颜色（#RRGGBB 或 #RRGGBBAA）',
    editTag: '编辑标签',
    jsonDraftChanged: 'JSON 有未保存修改；刷新和标签操作会保留草稿。',
    jsonDraftStale: '地址簿已变更，JSON 草稿仍保留原版本；保存会提示冲突。请复制需要的内容后加载最新。',
    discardJsonDraft: '放弃 JSON 草稿并加载最新',
    confirmDiscardJsonDraft: '确认放弃当前 JSON 草稿并加载最新地址簿？',
    savedRefreshFailed: '已保存，但刷新失败。草稿已保留，请刷新地址簿后继续。',
    noTags: '暂无标签',
    language: 'English',
    loading: '加载中...',
    signedInAs: '已登录账户',
    cookieUnavailable: 'Cookie 会话不可用，请允许站点 Cookie 或使用同源反向代理',
    apiUnavailable: '无法连接 API 服务',
    navPersonal: '个人',
    navDevices: '设备',
    navSharing: '共享',
    navAdmin: '管理'
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
    addressBookSubtitle: 'Manage devices, tags, and raw data in your personal address book',
    addressBookDevices: 'Devices',
    addressBookDevicesHint: 'Manage devices, aliases, and tags in your address book',
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
    oauthSubtitle: 'Configure OAuth2/OIDC sign-in providers',
    oauthSignIn: 'Sign in with OAuth',
    provider: 'Provider',
    providerList: 'Configured providers',
    authorizationEndpoint: 'Authorization endpoint',
    userInfoEndpoint: 'Userinfo endpoint',
    redirectConfigured: 'Callback configured',
    redirectNotConfigured: 'Callback not configured',
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
    noProviders: 'No providers',
    oauthTabIdentity: 'Basic',
    oauthTabClient: 'Client',
    oauthTabEndpoints: 'Endpoints',
    oauthTabScope: 'Scopes & status',
    oauthFormHint: 'Name, type, and endpoints are read-only while editing',
    requiredField: 'Complete the required fields',
    sessionId: 'Session ID',
    noSessions: 'No sessions found',
    devices: 'Devices',
    devicesSubtitle: 'View bound devices and administrator registration records',
    boundDevices: 'Bound devices',
    boundDevicesHint: 'Device ownership is confirmed by an administrator via fingerprint binding',
    deviceRegistry: 'Registration and binding',
    deviceRegistrySubtitle: 'Registration data comes from hbbs; unsigned reports are for reference only',
    deviceRegistryEmpty: 'No registered devices',
    deviceRegistryHint: 'Search registration records by RustDesk ID',
    untrustedReport: 'Unsigned report, for reference only',
    verifiedBinding: 'Verified ownership',
    pendingBinding: 'Ownership pending verification',
    fingerprint: 'Device fingerprint',
    fingerprintPlaceholder: 'Fingerprint: 8721 1612 8169 8f4f ...',
    fingerprintHelp: 'Supports 64-character hex fingerprints with spaces or line breaks; SHA-256 is computed in the browser. sha256:<hex> is also accepted.',
    fingerprintExample: 'Example: 8721 1612 8169 8f4f cd13 be20 1af1 d9b5 616c 8408 bc0f cbdc 3752 7e78 5b55 9dce',
    fingerprintStatusEmpty: 'Enter a device fingerprint to verify',
    fingerprintStatusReady: 'Valid format; the server will verify on submit',
    fingerprintStatusMatch: 'Fingerprint matches',
    fingerprintStatusMismatch: 'Fingerprint does not match; check the value shown on the device',
    fingerprintStatusInvalid: 'Enter a 64-character hex fingerprint or sha256:<hex>',
    bindDevice: 'Confirm binding',
    unbindDevice: 'Unbind',
    bindAccount: 'Account',
    selectAccount: 'Select account',
    noAvailableAccounts: 'No accounts available',
    boundTo: 'Bound to',
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
    addDeviceTitle: 'Add device',
    editDeviceTitle: 'Edit device',
    peerEditorHint: 'Fill in device details and select tags',
    basicInfo: 'Basic information',
    deviceInfo: 'Device information',
    advancedOptions: 'Advanced options',
    noAssignedDevices: 'No assigned devices',
    ldap: 'LDAP configuration',
    ldapSubtitle: 'Configure LDAP connection and user search',
    ldapUrl: 'LDAP URL',
    bindDn: 'Bind DN',
    bindPassword: 'Bind password (leave blank to keep)',
    userBaseDn: 'User base DN',
    userFilter: 'User filter',
    usernameAttribute: 'Username attribute',
    emailAttribute: 'Email attribute',
    useTls: 'Use StartTLS',
    timeoutSeconds: 'Timeout (seconds)',
    ldapTabConnection: 'Connection',
    ldapTabBind: 'Bind',
    ldapTabSearch: 'User search',
    ldapStatus: 'Current status',
    ldapSettings: 'Connection and search settings',
    ldapConfigured: 'Configured',
    ldapNotConfigured: 'Not configured',
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
    noMatchingDevices: 'No matching devices',
    deleteDevice: 'Delete device',
    noGroups: 'No groups found',
    noGroupMembers: 'No members found',
    addMember: 'Add member',
    uuid: 'Device UUID',
    deviceUuid: 'Device UUID',
    rustdeskId: 'RustDesk ID',
    platform: 'Platform',
    deviceName: 'Device name',
    registeredAt: 'Last registered',
    search: 'Search',
    clear: 'Clear',
    searchDevices: 'Search device ID, alias, hostname, or tag',
    deviceSearchPlaceholder: 'Search UUID, RustDesk ID, or account',
    allTags: 'All',
    clearFilters: 'Clear filters',
    deviceCount: '{count} devices',
    deviceCountFiltered: 'Showing {shown} of {total} devices',
    addTag: 'Add',
    newTagPlaceholder: 'New tag',
    noSelectedTags: 'No tags selected',
    removeTag: 'Remove tag',
    tagTooLong: 'Tag names must be 64 characters or fewer',
    invalidPeerId: 'Enter a device ID',
    advancedJson: 'Advanced: raw JSON',
    advancedJsonHint: 'Edit the full address book document. Use the Save button in the page header.',
    jsonDraftBadge: 'Unsaved changes',
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
    tagsHint: 'Select existing tags or type a new tag and press Enter',
    tagName: 'Tag name',
    tagColor: 'Color (#RRGGBB or #RRGGBBAA)',
    editTag: 'Edit tag',
    jsonDraftChanged: 'JSON has unsaved changes; refresh and tag actions will keep the draft.',
    jsonDraftStale: 'The address book changed. This JSON draft keeps its original revision and will conflict on save. Copy what you need before loading the latest version.',
    discardJsonDraft: 'Discard JSON draft and load latest',
    confirmDiscardJsonDraft: 'Discard the current JSON draft and load the latest address book?',
    savedRefreshFailed: 'Saved, but refresh failed. The draft was kept; refresh the address book before continuing.',
    noTags: 'No tags found',
    language: '简体中文',
    loading: 'Loading...',
    signedInAs: 'Signed in as',
    cookieUnavailable: 'Cookie session unavailable. Allow site cookies or use a same-origin reverse proxy.',
    apiUnavailable: 'Unable to reach the API server',
    navPersonal: 'Personal',
    navDevices: 'Devices',
    navSharing: 'Sharing',
    navAdmin: 'Administration'
  }
};
const state = {
  locale: localStorage.getItem(LOCALE_KEY) || 'zh-CN',
  csrfToken: '',
  cookieSession: false,
  user: null,
  activeView: 'profile',
  authMode: 'login',
  addressBookSnapshot: null,
  addressBookJsonDraft: { text: '', baseText: '', baseRevision: null },
  addressBookWritable: false,
  addressBookBusy: false,
  addressBookRequest: 0,
  addressBookSearch: '',
  addressBookTagFilter: '',
  addressBookFormOpen: false,
  addressBookAdvancedOpen: false,
  peerNewTag: '',
  addressBookDraft: {
    id: '',
    peer_id: '',
    username: '',
    hostname: '',
    alias: '',
    platform: '',
    tags: [],
    force_always_relay: false
  },
  tagDraft: { name: '', color: '' },
  serverConfig: null,
  users: [],
  userDraft: { username: '', email: '', password: '' },
  sessions: [],
  oauthProviders: [],
  oauthLoginProviders: [],
  oauthRedirectConfigured: false,
  oauthDraft: null,
  oauthTab: 'identity',
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
  deviceSearch: '',
  deviceBindDrafts: {},
  ldap: null,
  ldapDraft: null,
  ldapTab: 'connection',
  busy: false,
  notice: null
};

const app = document.querySelector('#app');
const t = key => messages[state.locale][key] || key;
const format = (key, values = {}) => t(key).replace(/\{(\w+)\}/g, (_, name) => values[name] ?? '');

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
  updateAddressBookDraftStatus();
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
  nav.append(navSection(t('navPersonal'), [
    ['profile', t('profile')],
    ['addressBook', t('addressBook')]
  ]));
  nav.append(navSection(t('navDevices'), [
    ['devices', t('devices')],
    ['deviceGroups', t('deviceGroups')]
  ]));
  nav.append(navSection(t('navSharing'), [['groups', t('groups')]]));
  if (state.user.is_admin) {
    nav.append(navSection(t('navAdmin'), [
      ['users', t('users')],
      ['sessions', t('sessions')],
      ['oauth', t('oauth')],
      ['ldap', t('ldap')]
    ]));
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
  const view = element('div', 'view');
  view.append(pageHeader(t('profile'), ''));
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
  view.append(card({ body: grid }));
  return view;
}

function emptyPeerDraft() {
  return {
    id: '',
    peer_id: '',
    username: '',
    hostname: '',
    alias: '',
    platform: '',
    tags: [],
    force_always_relay: false
  };
}

function addressBookEntries() {
  return state.addressBookSnapshot?.entries || [];
}

function tagColor(name) {
  return (state.addressBookSnapshot?.tags || []).find(tag => tag.name === name)?.color || '';
}

function tagChips(names) {
  const values = Array.isArray(names) ? names : [];
  if (!values.length) return '—';
  const wrap = element('div', 'tag-chip-list');
  values.forEach(name => wrap.append(tagChip(name, tagColor(name))));
  return wrap;
}

function filteredAddressBookEntries() {
  const query = state.addressBookSearch.trim().toLowerCase();
  return addressBookEntries().filter(peer => {
    if (state.addressBookTagFilter && !(peer.tags || []).includes(state.addressBookTagFilter)) return false;
    if (!query) return true;
    return [peer.peerId, peer.id, peer.alias, peer.username, peer.hostname, peer.platform, ...(peer.tags || [])]
      .filter(Boolean).join(' ').toLowerCase().includes(query);
  });
}

function updateAddressBookCount() {
  const node = document.querySelector('#address-book-count');
  if (!node) return;
  const total = addressBookEntries().length;
  const shown = filteredAddressBookEntries().length;
  node.textContent = shown === total ? format('deviceCount', { count: total }) : format('deviceCountFiltered', { shown, total });
}

function addressBookRow(peer) {
  const row = document.createElement('tr');
  row.dataset.peerEntry = peer.id || peer.peerId || '';
  row.append(cell(t('peerId'), peer.peerId || peer.id || '—', 'strong-cell mono'));
  const alias = element('div', 'stacked-cell');
  alias.append(element('span', '', peer.alias || '—'));
  if (peer.username) alias.append(element('small', 'muted', peer.username));
  row.append(cell(t('alias'), alias));
  row.append(cell(t('hostname'), peer.hostname || '—'));
  row.append(cell(t('platform'), peer.platform || '—'));
  row.append(cell(t('tags'), tagChips(peer.tags)));
  const actions = element('div', 'button-group row-actions');
  const edit = button('', t('updatePeer'), 'secondary');
  edit.dataset.editPeer = peer.id || peer.peerId || '';
  edit.disabled = state.addressBookBusy || !state.addressBookWritable;
  actions.append(edit);
  const remove = button('', t('delete'), 'secondary');
  remove.dataset.deletePeer = peer.id || peer.peerId || '';
  remove.disabled = state.addressBookBusy || !state.addressBookWritable;
  actions.append(remove);
  row.append(cell(t('actions'), actions, 'actions-cell'));
  return row;
}

function fillAddressBookRows(tbody) {
  const entries = filteredAddressBookEntries();
  tbody.replaceChildren();
  if (!entries.length) {
    const emptyText = addressBookEntries().length ? t('noMatchingDevices') : t('noPeers');
    tbody.append(emptyRow(6, emptyText));
    return;
  }
  entries.forEach(peer => tbody.append(addressBookRow(peer)));
}

function renderAddressBookRows() {
  const tbody = document.querySelector('#address-book-tbody');
  if (tbody) fillAddressBookRows(tbody);
  updateAddressBookCount();
  updateAddressBookFilterButtons();
}

function updateAddressBookFilterButtons() {
  document.querySelectorAll('[data-tag-filter]').forEach(node => {
    const active = (node.dataset.tagFilter || '') === state.addressBookTagFilter;
    node.classList.toggle('active', active);
    node.setAttribute('aria-pressed', String(active));
  });
}

function addressBookListCard() {
  const search = document.createElement('input');
  search.id = 'address-book-search';
  search.type = 'search';
  search.className = 'search-input';
  search.placeholder = t('searchDevices');
  search.value = state.addressBookSearch;
  const count = element('span', 'toolbar-count');
  count.id = 'address-book-count';
  const total = addressBookEntries().length;
  const shown = filteredAddressBookEntries().length;
  count.textContent = shown === total ? format('deviceCount', { count: total }) : format('deviceCountFiltered', { shown, total });
  const add = button('add-peer', t('addDeviceTitle'), 'primary');
  add.disabled = state.addressBookBusy || !state.addressBookWritable;
  const toolbar = element('div', 'toolbar');
  toolbar.append(search, count, add);

  const filter = element('div', 'tag-filter');
  filter.id = 'address-book-tag-filter';
  const all = button('', t('allTags'), 'tag-filter-button');
  all.dataset.tagFilter = '';
  all.classList.toggle('active', !state.addressBookTagFilter);
  all.setAttribute('aria-pressed', String(!state.addressBookTagFilter));
  filter.append(all);
  (state.addressBookSnapshot?.tags || []).forEach(tag => {
    const node = button('', tag.name, 'tag-filter-button');
    node.dataset.tagFilter = tag.name;
    node.classList.toggle('active', state.addressBookTagFilter === tag.name);
    node.setAttribute('aria-pressed', String(state.addressBookTagFilter === tag.name));
    if (tag.color) {
      const swatch = element('span', 'tag-swatch');
      swatch.style.backgroundColor = tag.color;
      node.prepend(swatch);
    }
    filter.append(node);
  });

  const headers = [t('peerId'), t('alias'), t('hostname'), t('platform'), t('tags'), t('actions')];
  const table = tableShell(headers, [], { bodyId: 'address-book-tbody' });
  fillAddressBookRows(table.tbody);
  const node = card({ id: 'address-book-list', title: t('addressBookDevices'), subtitle: t('addressBookDevicesHint'), body: [toolbar, filter, table.wrap] });
  updateAddressBookFilterButtons();
  updateAddressBookCount();
  return node;
}

function addressBookView() {
  const view = element('div', 'view');
  const refresh = button('refresh-address-book', t('refresh'), 'secondary');
  refresh.disabled = state.addressBookBusy;
  const save = button('save-address-book', state.busy ? t('saving') : t('save'), 'primary');
  save.disabled = state.addressBookBusy || !state.addressBookWritable;
  view.append(pageHeader(t('addressBook'), t('addressBookSubtitle'), [refresh, save]));
  const status = element('div', 'draft-status');
  status.id = 'address-book-draft-status';
  view.append(status);
  view.append(peerEditor());
  view.append(addressBookListCard());
  view.append(tagsPanel());
  view.append(advancedJsonPanel());
  return view;
}

function peerEditor() {
  const draft = state.addressBookDraft;
  const disabled = state.addressBookBusy || !state.addressBookWritable;
  const details = document.createElement('details');
  details.id = 'peer-editor';
  details.className = 'card collapsible';
  details.open = state.addressBookFormOpen;
  const summary = element('summary', 'card-header collapsible-summary');
  const heading = element('div', 'card-heading');
  heading.append(element('h2', '', draft.id ? t('editDeviceTitle') : t('addDeviceTitle')));
  heading.append(element('p', 'card-subtitle', t('peerEditorHint')));
  summary.append(heading, element('span', 'chevron'));
  details.append(summary);

  const form = document.createElement('form');
  form.id = 'peer-form';
  form.className = 'form-stack';
  form.append(formSection(t('basicInfo'), '', [
    configField('peer-id', t('peerId'), draft.peer_id, 'text', { required: true, maxLength: 128, disabled }),
    configField('peer-alias', t('alias'), draft.alias, 'text', { maxLength: 256, disabled })
  ]));
  form.append(formSection(t('deviceInfo'), '', [
    configField('peer-username', t('username'), draft.username, 'text', { maxLength: 256, disabled }),
    configField('peer-hostname', t('hostname'), draft.hostname, 'text', { maxLength: 256, disabled }),
    configField('peer-platform', t('platform'), draft.platform, 'text', { maxLength: 128, disabled })
  ]));
  form.append(formSection(t('tags'), t('tagsHint'), [tagPicker(disabled)]));
  form.append(formSection(t('advancedOptions'), '', [
    checkboxField('peer-relay', t('forceRelay'), draft.force_always_relay, { disabled })
  ]));
  const actions = element('div', 'form-actions');
  const save = button('save-peer', state.busy ? t('saving') : (draft.id ? t('updatePeer') : t('addPeer')), 'primary');
  save.type = 'submit';
  save.disabled = disabled;
  actions.append(save);
  const clear = button('clear-peer', t('cancel'), 'secondary');
  clear.disabled = disabled;
  actions.append(clear);
  form.append(actions);
  const body = element('div', 'card-body');
  body.append(form);
  details.append(body);
  return details;
}

function tagPicker(disabled) {
  const wrap = element('div', 'tag-picker span-2');
  const existing = state.addressBookSnapshot?.tags || [];
  const toggles = element('div', 'tag-toggle-list');
  if (!existing.length) toggles.append(element('small', 'muted', t('noTags')));
  existing.forEach(tag => {
    const node = button('', tag.name, 'tag-toggle');
    node.dataset.tagToggle = tag.name;
    node.disabled = disabled;
    const selectedTag = (state.addressBookDraft.tags || []).includes(tag.name);
    node.classList.toggle('active', selectedTag);
    node.setAttribute('aria-pressed', String(selectedTag));
    if (tag.color) {
      const swatch = element('span', 'tag-swatch');
      swatch.style.backgroundColor = tag.color;
      node.prepend(swatch);
    }
    toggles.append(node);
  });
  wrap.append(toggles);
  const selected = element('div', 'tag-chip-list');
  selected.id = 'peer-selected-tags';
  const selectedTags = state.addressBookDraft.tags || [];
  if (!selectedTags.length) selected.append(element('small', 'muted', t('noSelectedTags')));
  selectedTags.forEach(name => selected.append(selectedTagChip(name)));
  wrap.append(selected);
  const custom = element('div', 'inline-form compact');
  const input = document.createElement('input');
  input.id = 'peer-new-tag';
  input.name = 'peer-new-tag';
  input.placeholder = t('newTagPlaceholder');
  input.maxLength = 64;
  input.value = state.peerNewTag || '';
  input.disabled = disabled;
  const add = button('add-peer-tag', t('addTag'), 'secondary');
  add.disabled = disabled;
  custom.append(input, add);
  wrap.append(custom);
  return wrap;
}

function selectedTagChip(name) {
  const chip = tagChip(name, tagColor(name));
  const remove = element('button', 'tag-remove', '×');
  remove.type = 'button';
  remove.dataset.removePeerTag = name;
  remove.disabled = state.addressBookBusy || !state.addressBookWritable;
  remove.setAttribute('aria-label', `${t('removeTag')}: ${name}`);
  chip.append(remove);
  return chip;
}

function renderSelectedPeerTags() {
  const wrap = document.querySelector('#peer-selected-tags');
  if (!wrap) return;
  wrap.replaceChildren();
  const tags = state.addressBookDraft.tags || [];
  if (!tags.length) wrap.append(element('small', 'muted', t('noSelectedTags')));
  tags.forEach(name => wrap.append(selectedTagChip(name)));
}

function syncPeerTagToggles() {
  document.querySelectorAll('[data-tag-toggle]').forEach(node => {
    const active = (state.addressBookDraft.tags || []).includes(node.dataset.tagToggle);
    node.classList.toggle('active', active);
    node.setAttribute('aria-pressed', String(active));
  });
}

function togglePeerTag(name) {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const tags = state.addressBookDraft.tags || (state.addressBookDraft.tags = []);
  const index = tags.indexOf(name);
  if (index >= 0) tags.splice(index, 1);
  else tags.push(name);
  renderSelectedPeerTags();
  syncPeerTagToggles();
}

function addPeerTag() {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const input = document.querySelector('#peer-new-tag');
  const name = String(input?.value || '').trim();
  if (!name) return;
  if (name.length > 64) { setNotice('error', t('tagTooLong')); render(); return; }
  const tags = state.addressBookDraft.tags || (state.addressBookDraft.tags = []);
  if (!tags.includes(name)) tags.push(name);
  state.peerNewTag = '';
  if (input) input.value = '';
  renderSelectedPeerTags();
  syncPeerTagToggles();
}

function removePeerTag(name) {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const tags = state.addressBookDraft.tags || [];
  const index = tags.indexOf(name);
  if (index >= 0) tags.splice(index, 1);
  renderSelectedPeerTags();
  syncPeerTagToggles();
}

function openPeerEditor(draft = null) {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  state.addressBookDraft = draft || emptyPeerDraft();
  state.peerNewTag = '';
  state.addressBookFormOpen = true;
  render();
  requestAnimationFrame(() => document.querySelector('#peer-id')?.focus());
}

function tagsPanel() {
  const disabled = state.addressBookBusy || !state.addressBookWritable;
  const form = document.createElement('form');
  form.id = 'tag-form';
  form.className = 'inline-form compact';
  const name = document.createElement('input');
  name.name = 'name';
  name.placeholder = t('tagName');
  name.maxLength = 64;
  name.required = true;
  name.value = state.tagDraft.name || '';
  name.disabled = disabled;
  const color = document.createElement('input');
  color.name = 'color';
  color.placeholder = t('tagColor');
  color.maxLength = 9;
  color.value = state.tagDraft.color || '';
  color.disabled = disabled;
  form.append(name, color);
  const save = button('save-tag', t('save'), 'primary');
  save.type = 'submit';
  save.disabled = disabled;
  form.append(save);

  const list = element('div', 'tag-list');
  const tags = state.addressBookSnapshot?.tags || [];
  if (!tags.length) list.append(element('p', 'empty-state', t('noTags')));
  tags.forEach(tag => {
    const row = element('div', 'tag-row');
    row.append(tagChip(tag.name, tag.color));
    const edit = button('', t('editTag'), 'secondary');
    edit.dataset.editTag = tag.name || '';
    edit.disabled = disabled;
    row.append(edit);
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteTag = tag.name || '';
    remove.disabled = disabled;
    row.append(remove);
    list.append(row);
  });
  return card({ id: 'tags-panel', title: t('tags'), body: [form, list] });
}

function advancedJsonPanel() {
  const details = document.createElement('details');
  details.id = 'address-book-advanced';
  details.className = 'card collapsible advanced-json';
  details.open = state.addressBookAdvancedOpen;
  const summary = element('summary', 'card-header collapsible-summary');
  const heading = element('div', 'card-heading');
  heading.append(element('h2', '', t('advancedJson')));
  heading.append(element('p', 'card-subtitle', t('advancedJsonHint')));
  const dirty = element('span', 'badge warning');
  dirty.id = 'address-book-advanced-badge';
  dirty.textContent = t('jsonDraftBadge');
  dirty.hidden = true;
  summary.append(heading, dirty, element('span', 'chevron'));
  details.append(summary);
  const body = element('div', 'card-body');
  const editor = document.createElement('textarea');
  editor.id = 'address-book-editor';
  editor.className = 'json-editor';
  editor.spellcheck = false;
  editor.value = state.addressBookJsonDraft.text;
  editor.disabled = state.addressBookBusy;
  body.append(editor);
  details.append(body);
  return details;
}

function usersView() {
  const view = element('div', 'view');
  const refresh = button('refresh-users', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('users'), '', [refresh]));

  const form = document.createElement('form');
  form.id = 'admin-user-form';
  form.className = 'form-stack';
  const grid = formGrid([
    configField('admin-user-name', t('username'), state.userDraft.username, 'text', { disabled: state.busy }),
    configField('admin-user-email', t('email'), state.userDraft.email, 'email', { disabled: state.busy }),
    configField('admin-user-password', t('password'), state.userDraft.password, 'password', { disabled: state.busy })
  ]);
  const actions = element('div', 'form-actions');
  const submit = button('create-admin-user', state.busy ? t('creatingUser') : t('createUser'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  actions.append(submit);
  form.append(grid, actions);
  view.append(card({ title: t('createUser'), body: form }));

  const rows = state.users.map(user => {
    const row = document.createElement('tr');
    row.append(cell(t('account'), user.name || user.username || '', 'strong-cell'));
    row.append(cell(t('email'), user.email || '—'));
    row.append(cell(t('role'), badge(user.is_admin ? t('administrator') : t('member'), user.is_admin ? 'info' : 'neutral')));
    row.append(cell(t('status'), badge(Number(user.status) === 1 ? t('enabled') : t('disabled'), Number(user.status) === 1 ? 'success' : 'warning')));
    row.append(cell(t('createdAt'), user.created_at || user.createdAt || '—'));
    const actionsCell = element('div', 'button-group row-actions');
    const toggle = button('', Number(user.status) === 1 ? t('disable') : t('enable'), 'secondary');
    toggle.dataset.toggleUser = user.id;
    toggle.dataset.userStatus = Number(user.status) === 1 ? '0' : '1';
    toggle.disabled = state.busy;
    actionsCell.append(toggle);
    const remove = button('', t('remove'), 'secondary');
    remove.dataset.deleteUser = user.id;
    remove.disabled = state.busy;
    actionsCell.append(remove);
    row.append(cell(t('actions'), actionsCell, 'actions-cell'));
    return row;
  });
  if (!rows.length) rows.push(emptyRow(6, t('noUsers')));
  const table = tableShell([t('account'), t('email'), t('role'), t('status'), t('createdAt'), t('actions')], rows);
  view.append(card({ body: table.wrap }));
  return view;
}

function sessionsView() {
  const view = element('div', 'view');
  const refresh = button('refresh-sessions', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('sessions'), '', [refresh]));
  const rows = state.sessions.map(session => {
    const row = document.createElement('tr');
    row.append(cell(t('sessionId'), session.id || '—', 'strong-cell mono'));
    row.append(cell(t('account'), session.username || session.user_id || '—'));
    row.append(cell(t('platform'), [session.device_os, session.device_type].filter(Boolean).join(' / ') || '—'));
    const revoke = button('', t('logout'), 'secondary');
    revoke.dataset.revokeSession = session.id;
    revoke.disabled = Boolean(session.revoked_at) || state.busy;
    row.append(cell(t('actions'), revoke, 'actions-cell'));
    return row;
  });
  if (!rows.length) rows.push(emptyRow(4, t('noSessions')));
  const table = tableShell([t('sessionId'), t('account'), t('platform'), t('actions')], rows);
  view.append(card({ body: table.wrap }));
  return view;
}

function oauthTabItems() {
  return [
    { id: 'identity', label: t('oauthTabIdentity') },
    { id: 'client', label: t('oauthTabClient') },
    { id: 'endpoints', label: t('oauthTabEndpoints') },
    { id: 'scope', label: t('oauthTabScope') }
  ];
}

function oauthForm() {
  const draft = state.oauthDraft || {};
  const editing = Boolean(draft.id);
  const form = document.createElement('form');
  form.id = 'oauth-provider-form';
  form.className = 'form-stack';
  form.noValidate = true;
  const tabNode = tabs('oauth-tabs', oauthTabItems(), state.oauthTab);
  form.append(tabNode);

  const panels = {};
  const makePanel = (id, children) => {
    const panel = element('div', 'tab-panel');
    panel.id = `oauth-tabs-${id}-panel`;
    panel.dataset.oauthPanel = id;
    panel.setAttribute('role', 'tabpanel');
    panel.setAttribute('aria-labelledby', `oauth-tabs-${id}`);
    panel.hidden = state.oauthTab !== id;
    panel.append(formGrid(children));
    panels[id] = panel;
    return panel;
  };

  form.append(makePanel('identity', [
    selectField('oauth-kind', 'OAuth / OIDC', [
      { value: 'oauth2', label: 'OAuth2' },
      { value: 'oidc', label: 'OIDC' }
    ], draft.kind || 'oidc', { name: 'kind', disabled: editing }),
    configField('oauth-name', t('providerName'), draft.name || '', 'text', { name: 'name', readOnly: editing, disabled: state.busy }),
    checkboxField('oauth-enabled', t('enabled'), draft.enabled !== false, { name: 'enabled', disabled: state.busy })
  ]));
  form.append(makePanel('client', [
    configField('oauth-client_id', t('clientId'), draft.client_id || '', 'text', { name: 'client_id', readOnly: editing, disabled: state.busy }),
    configField('oauth-client_secret', t('clientSecret'), draft.client_secret || '', 'password', { name: 'client_secret', disabled: state.busy })
  ]));
  form.append(makePanel('endpoints', [
    configField('oauth-authorization_url', t('authorizationEndpoint'), draft.authorization_url || '', 'url', { name: 'authorization_url', readOnly: editing, disabled: state.busy, className: 'span-2' }),
    configField('oauth-token_url', t('tokenEndpoint'), draft.token_url || '', 'url', { name: 'token_url', readOnly: editing, disabled: state.busy, className: 'span-2' }),
    configField('oauth-userinfo_url', t('userInfoEndpoint'), draft.userinfo_url || '', 'url', { name: 'userinfo_url', readOnly: editing, disabled: state.busy, className: 'span-2' }),
    configField('oauth-issuer_url', t('issuerEndpoint'), draft.issuer_url || '', 'url', { name: 'issuer_url', readOnly: editing, disabled: state.busy, className: 'span-2' }),
    configField('oauth-jwks_url', t('jwksEndpoint'), draft.jwks_url || '', 'url', { name: 'jwks_url', readOnly: editing, disabled: state.busy, className: 'span-2' })
  ]));
  form.append(makePanel('scope', [
    configField('oauth-scopes', t('scopes'), draft.scopes || '', 'text', { name: 'scopes', disabled: state.busy, className: 'span-2' })
  ]));

  const actions = element('div', 'form-actions');
  const save = button('save-oauth-provider', editing ? t('saveProvider') : t('addProvider'), 'primary');
  save.type = 'submit';
  save.disabled = state.busy;
  actions.append(save);
  if (editing) {
    const cancel = button('cancel-oauth-provider', t('cancel'), 'secondary');
    cancel.disabled = state.busy;
    actions.append(cancel);
  }
  form.append(actions);

  return card({
    title: editing ? `${t('editProvider')}: ${draft.name || ''}` : t('addProvider'),
    subtitle: editing ? t('oauthFormHint') : '',
    body: form
  });
}

function oauthProvidersCard() {
  const rows = state.oauthProviders.map(provider => {
    const row = document.createElement('tr');
    row.append(cell(t('provider'), provider.name || '—', 'strong-cell'));
    row.append(cell('OAuth / OIDC', provider.kind === 'oauth2' ? 'OAuth2' : (provider.kind === 'oidc' ? 'OIDC' : '—')));
    row.append(cell(t('authorizationEndpoint'), provider.authorization_url || '—', 'mono muted-cell'));
    const status = provider.read_only ? badge(t('providerReadOnly'), 'neutral') : badge(provider.enabled === false ? t('providerDisabled') : t('providerEnabled'), provider.enabled === false ? 'warning' : 'success');
    row.append(cell(t('status'), status));
    const actions = element('div', 'button-group row-actions');
    const edit = button('', t('editProvider'), 'secondary');
    edit.dataset.editOauth = provider.id || provider.name;
    edit.disabled = Boolean(provider.read_only) || state.busy;
    actions.append(edit);
    const toggle = button('', provider.enabled === false ? t('enable') : t('disable'), 'secondary');
    toggle.dataset.toggleOauth = provider.id || provider.name;
    toggle.dataset.oauthEnabled = provider.enabled === false ? 'false' : 'true';
    toggle.disabled = Boolean(provider.read_only) || state.busy;
    actions.append(toggle);
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteOauth = provider.id || provider.name;
    remove.disabled = Boolean(provider.read_only) || state.busy;
    actions.append(remove);
    row.append(cell(t('actions'), actions, 'actions-cell'));
    return row;
  });
  if (!rows.length) rows.push(emptyRow(5, t('noProviders')));
  const table = tableShell([t('provider'), 'OAuth / OIDC', t('authorizationEndpoint'), t('status'), t('actions')], rows);
  return card({ title: t('providerList'), body: table.wrap });
}

function oauthView() {
  const view = element('div', 'view');
  const refresh = button('refresh-oauth', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('oauth'), t('oauthSubtitle'), [refresh]));
  const status = element('div', 'status-banner');
  status.append(badge(state.oauthRedirectConfigured ? t('enabled') : t('disabled'), state.oauthRedirectConfigured ? 'success' : 'warning'));
  status.append(element('span', '', state.oauthRedirectConfigured ? t('redirectConfigured') : t('redirectNotConfigured')));
  view.append(status);
  view.append(oauthForm());
  view.append(oauthProvidersCard());
  return view;
}

function deviceAccountName(device) {
  const user = state.registryUsers.find(item => item.id === device.user_id);
  if (user) return user.name || user.username || user.id;
  if (state.user && state.user.id === device.user_id) return state.user.name || state.user.username || state.user.id;
  return device.user_id || '—';
}

function filteredDevices() {
  const query = state.deviceSearch.trim().toLowerCase();
  if (!query) return state.devices;
  return state.devices.filter(device => [device.uuid, device.peer_id, device.name, device.os, device.device_type, deviceAccountName(device)]
    .filter(Boolean).join(' ').toLowerCase().includes(query));
}

function deviceStatusCell(device) {
  const wrap = element('div', 'status-badges');
  wrap.append(badge(device.online ? t('online') : t('offline'), device.online ? 'success' : 'neutral'));
  wrap.append(badge(Number(device.status) === 1 ? t('enabled') : t('disabled'), Number(device.status) === 1 ? 'success' : 'warning'));
  wrap.append(badge(device.verified ? t('verifiedBinding') : t('pendingBinding'), device.verified ? 'info' : 'warning'));
  return wrap;
}

function deviceRow(device) {
  const row = document.createElement('tr');
  const uuid = element('div', 'stacked-cell');
  uuid.append(element('code', 'mono', device.uuid || '—'));
  if (device.name) uuid.append(element('small', 'muted', device.name));
  row.append(cell(t('deviceUuid'), uuid));
  row.append(cell(t('account'), deviceAccountName(device)));
  row.append(cell(t('rustdeskId'), device.peer_id || '—', 'mono'));
  row.append(cell(t('platform'), [device.os, device.device_type].filter(Boolean).join(' / ') || '—'));
  row.append(cell(t('status'), deviceStatusCell(device)));
  if (state.user.is_admin) {
    const remove = button('', t('deleteDevice'), 'secondary');
    remove.dataset.deleteDevice = device.id;
    remove.disabled = state.busy;
    row.append(cell(t('actions'), remove, 'actions-cell'));
  }
  return row;
}

function fillDeviceRows(tbody) {
  const devices = filteredDevices();
  tbody.replaceChildren();
  if (!devices.length) {
    tbody.append(emptyRow(state.user.is_admin ? 6 : 5, state.devices.length ? t('noMatchingDevices') : t('noDevices')));
    return;
  }
  devices.forEach(device => tbody.append(deviceRow(device)));
}

function renderDeviceRows() {
  const tbody = document.querySelector('#device-tbody');
  if (tbody) fillDeviceRows(tbody);
  const count = document.querySelector('#device-count');
  if (count) {
    const total = state.devices.length;
    const shown = filteredDevices().length;
    count.textContent = shown === total ? format('deviceCount', { count: total }) : format('deviceCountFiltered', { shown, total });
  }
}

function devicesListCard() {
  const search = document.createElement('input');
  search.id = 'device-search';
  search.type = 'search';
  search.className = 'search-input';
  search.placeholder = t('deviceSearchPlaceholder');
  search.value = state.deviceSearch;
  const count = element('span', 'toolbar-count');
  count.id = 'device-count';
  const total = state.devices.length;
  const shown = filteredDevices().length;
  count.textContent = shown === total ? format('deviceCount', { count: total }) : format('deviceCountFiltered', { shown, total });
  const toolbar = element('div', 'toolbar');
  toolbar.append(search, count);
  const headers = [t('deviceUuid'), t('account'), t('rustdeskId'), t('platform'), t('status')];
  if (state.user.is_admin) headers.push(t('actions'));
  const table = tableShell(headers, [], { bodyId: 'device-tbody' });
  fillDeviceRows(table.tbody);
  const node = card({ title: t('boundDevices'), subtitle: t('boundDevicesHint'), body: [toolbar, table.wrap] });
  renderDeviceRows();
  return node;
}

function formatTimestamp(ms) {
  if (!Number.isFinite(ms) || ms <= 0) return '—';
  return new Date(ms).toLocaleString(state.locale);
}

function registryCard(peer) {
  const draft = state.deviceBindDrafts[peer.peer_id] || {};
  const users = state.registryUsers;
  const node = element('section', 'registry-card');
  node.dataset.registryPeer = peer.peer_id;
  const header = element('div', 'registry-header');
  const title = element('div', 'registry-title');
  title.append(element('strong', 'mono', peer.peer_id));
  title.append(badge(peer.online ? t('online') : t('offline'), peer.online ? 'success' : 'neutral'));
  title.append(badge(peer.verified ? t('verifiedBinding') : t('pendingBinding'), peer.verified ? 'info' : 'warning'));
  header.append(title);
  header.append(element('span', 'muted', `${t('registeredAt')}: ${formatTimestamp(peer.registered_at_ms)}`));
  node.append(header);

  const facts = element('dl', 'registry-facts');
  const uuidFact = element('div', 'fact');
  uuidFact.append(element('dt', '', t('deviceUuid')));
  uuidFact.append(element('dd', '', peer.uuid || '—'));
  facts.append(uuidFact);
  const report = peer.untrusted_sysinfo || {};
  const reportFact = element('div', 'fact');
  reportFact.append(element('dt', '', t('untrustedReport')));
  reportFact.append(element('dd', '', [report.hostname || report.device_name, report.username, report.os].filter(Boolean).join(' / ') || '—'));
  facts.append(reportFact);
  node.append(facts);

  const form = document.createElement('form');
  form.className = 'bind-form';
  form.dataset.bindPeer = peer.peer_id;
  const ownerOptions = users.length
    ? [{ value: '', label: t('selectAccount') }, ...users.map(user => ({ value: user.id, label: user.name || user.username || user.id }))]
    : [{ value: '', label: t('noAvailableAccounts') }];
  form.append(configField(`bind-owner-${peer.peer_id}`, t('bindAccount'), draft.user_id ?? peer.owner_id ?? '', 'select', {
    name: 'user_id', options: ownerOptions, required: true, disabled: state.busy || !users.length
  }));
  form.append(configField(`bind-fingerprint-${peer.peer_id}`, t('fingerprint'), draft.pk_fingerprint || '', 'textarea', {
    name: 'pk_fingerprint', rows: 2, placeholder: t('fingerprintPlaceholder'), help: t('fingerprintHelp'), title: t('fingerprintExample'),
    className: 'span-2', spellcheck: false, disabled: state.busy
  }));
  const status = element('p', 'fingerprint-status');
  status.dataset.fingerprintStatus = '';
  status.textContent = draft.pk_fingerprint ? '' : t('fingerprintStatusEmpty');
  form.append(status);
  const submit = button('', t('bindDevice'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy || !users.length || !String(draft.pk_fingerprint || '').trim();
  form.append(submit);
  node.append(form);

  if (peer.verified && peer.device_id) {
    const bound = element('div', 'registry-actions');
    const owner = state.registryUsers.find(user => user.id === peer.owner_id);
    bound.append(element('span', 'muted', `${t('boundTo')}: ${owner?.name || owner?.username || peer.owner_id || '—'}`));
    const unbind = button('', t('unbindDevice'), 'secondary');
    unbind.dataset.unbindDevice = peer.device_id;
    unbind.disabled = state.busy;
    bound.append(unbind);
    node.append(bound);
  }
  return node;
}

function deviceRegistryView() {
  const search = document.createElement('form');
  search.id = 'registry-search';
  search.className = 'toolbar';
  const query = document.createElement('input');
  query.name = 'peer_id';
  query.className = 'search-input';
  query.placeholder = t('deviceRegistryHint');
  query.value = state.registrySearch;
  const submit = button('', t('search'), 'secondary');
  submit.type = 'submit';
  search.append(query, submit);
  const list = element('div', 'registry-list');
  if (!state.registeredDevices.length) list.append(emptyState(t('deviceRegistryEmpty')));
  state.registeredDevices.forEach(peer => list.append(registryCard(peer)));
  return card({ title: t('deviceRegistry'), subtitle: t('deviceRegistrySubtitle'), body: [search, list] });
}

function devicesView() {
  const view = element('div', 'view');
  const refresh = button('refresh-devices', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('devices'), t('devicesSubtitle'), [refresh]));
  view.append(devicesListCard());
  if (state.user.is_admin) view.append(deviceRegistryView());
  return view;
}

function groupsView() {
  const view = element('div', 'view');
  const refresh = button('refresh-groups', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('groups'), '', [refresh]));
  const form = document.createElement('form');
  form.id = 'group-form';
  form.className = 'inline-form';
  const input = document.createElement('input');
  input.name = 'name';
  input.placeholder = t('groupName');
  input.maxLength = 128;
  input.required = true;
  input.value = state.groupDraft;
  input.disabled = state.busy;
  const submit = button('create-group', t('create'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(input, submit);
  view.append(card({ body: form }));

  const list = element('div', 'group-list');
  if (!state.groups.length) list.append(emptyState(t('noGroups')));
  state.groups.forEach(group => {
    const item = element('section', 'group-card');
    const heading = element('div', 'group-row');
    heading.append(element('strong', '', group.name || '—'));
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteGroup = group.id;
    remove.disabled = state.busy;
    heading.append(remove);
    item.append(heading);
    const members = state.groupMemberships.filter(member => member.group_id === group.id);
    const memberList = element('div', 'membership-list');
    if (!members.length) memberList.append(element('small', 'muted', t('noGroupMembers')));
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
      const assign = element('div', 'inline-form compact');
      const select = document.createElement('select');
      select.id = `group-select-${group.id}`;
      available.forEach(user => {
        const option = document.createElement('option');
        option.value = user.id;
        option.textContent = user.name || user.username || user.id;
        select.append(option);
      });
      select.disabled = state.busy;
      const add = button('', t('addMember'), 'primary');
      add.dataset.addGroup = group.id;
      add.disabled = state.busy;
      assign.append(select, add);
      item.append(assign);
    }
    list.append(item);
  });
  view.append(card({ body: list }));
  return view;
}

function deviceGroupsView() {
  const view = element('div', 'view');
  const refresh = button('refresh-device-groups', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('deviceGroups'), '', [refresh]));
  const form = document.createElement('form');
  form.id = 'device-group-form';
  form.className = 'inline-form';
  const input = document.createElement('input');
  input.id = 'device-group-name';
  input.name = 'name';
  input.placeholder = t('groupName');
  input.value = state.deviceGroupDraft;
  input.maxLength = 128;
  input.required = true;
  input.disabled = state.busy;
  const submit = button('create-device-group', t('create'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  form.append(input, submit);
  view.append(card({ body: form }));
  const list = element('div', 'group-list');
  if (!state.deviceGroups.length) list.append(emptyState(t('noDeviceGroups')));
  state.deviceGroups.forEach(group => {
    const item = element('div', 'group-card');
    const heading = element('div', 'group-row');
    heading.append(element('strong', '', group.name || '—'));
    const remove = button('', t('delete'), 'secondary');
    remove.dataset.deleteDeviceGroup = group.id;
    remove.disabled = state.busy;
    heading.append(remove);
    item.append(heading);
    const members = state.deviceGroupMemberships.filter(member => member.group_id === group.id);
    const assigned = element('div', 'membership-list');
    if (!members.length) assigned.append(element('small', 'muted', t('noAssignedDevices')));
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
      const assign = element('div', 'inline-form compact');
      const select = document.createElement('select');
      select.id = `device-group-select-${group.id}`;
      available.forEach(device => {
        const option = document.createElement('option');
        option.value = device.id;
        option.textContent = device.name || device.uuid || device.id;
        select.append(option);
      });
      select.disabled = state.busy;
      const add = button('', t('addDevice'), 'primary');
      add.dataset.addDeviceGroup = group.id;
      add.disabled = state.busy;
      assign.append(select, add);
      item.append(assign);
    }
    list.append(item);
  });
  view.append(card({ body: list }));
  return view;
}

function ldapTabItems() {
  return [
    { id: 'connection', label: t('ldapTabConnection') },
    { id: 'bind', label: t('ldapTabBind') },
    { id: 'search', label: t('ldapTabSearch') }
  ];
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
  const view = element('div', 'view');
  const refresh = button('refresh-ldap', t('refresh'), 'secondary');
  refresh.disabled = state.busy;
  view.append(pageHeader(t('ldap'), t('ldapSubtitle'), [refresh]));
  const status = element('div', 'status-banner');
  status.append(badge(config.enabled ? t('enabled') : t('disabled'), config.enabled ? 'success' : 'neutral'));
  status.append(element('span', '', config.configured ? t('ldapConfigured') : t('ldapNotConfigured')));
  view.append(status);

  const form = document.createElement('form');
  form.id = 'ldap-form';
  form.className = 'form-stack';
  form.append(element('p', 'notice-inline', t('runtimeOnly')));
  form.append(tabs('ldap-tabs', ldapTabItems(), state.ldapTab));
  const disabled = state.busy;
  const makePanel = (id, children) => {
    const panel = element('div', 'tab-panel');
    panel.id = `ldap-tabs-${id}-panel`;
    panel.dataset.ldapPanel = id;
    panel.setAttribute('role', 'tabpanel');
    panel.setAttribute('aria-labelledby', `ldap-tabs-${id}`);
    panel.hidden = state.ldapTab !== id;
    panel.append(formGrid(children));
    return panel;
  };
  const connectionToggles = element('div', 'checkbox-row span-2');
  connectionToggles.append(
    checkboxField('ldap-enabled', t('enabled'), config.enabled, { disabled }),
    checkboxField('ldap-use-tls', t('useTls'), config.use_tls, { disabled })
  );
  form.append(makePanel('connection', [
    connectionToggles,
    configField('ldap-url', t('ldapUrl'), config.url || '', 'text', { disabled, className: 'span-2' }),
    configField('ldap-timeout', t('timeoutSeconds'), String(config.timeout_seconds ?? 5), 'number', { min: 1, max: 60, disabled })
  ]));
  form.append(makePanel('bind', [
    configField('ldap-bind-dn', t('bindDn'), config.bind_dn || '', 'text', { disabled }),
    configField('ldap-bind-password', t('bindPassword'), config.bind_password || '', 'password', { disabled })
  ]));
  form.append(makePanel('search', [
    configField('ldap-user-base-dn', t('userBaseDn'), config.user_base_dn || '', 'text', { disabled }),
    configField('ldap-user-filter', t('userFilter'), config.user_filter || '', 'text', { disabled }),
    configField('ldap-username-attribute', t('usernameAttribute'), config.username_attribute || '', 'text', { disabled }),
    configField('ldap-email-attribute', t('emailAttribute'), config.email_attribute || '', 'text', { disabled })
  ]));
  const actions = element('div', 'form-actions');
  const submit = button('save-ldap', state.busy ? t('saving') : t('save'), 'primary');
  submit.type = 'submit';
  submit.disabled = state.busy;
  actions.append(submit);
  form.append(actions);
  view.append(card({ title: t('ldapSettings'), body: form }));
  return view;
}

function setOauthTab(tab) {
  state.oauthTab = tab;
  document.querySelectorAll('#oauth-tabs [data-tab]').forEach(node => {
    const active = node.dataset.tab === tab;
    node.classList.toggle('active', active);
    node.setAttribute('aria-selected', String(active));
    node.tabIndex = active ? 0 : -1;
  });
  document.querySelectorAll('[data-oauth-panel]').forEach(panel => { panel.hidden = panel.dataset.oauthPanel !== tab; });
}

function setLdapTab(tab) {
  state.ldapTab = tab;
  document.querySelectorAll('#ldap-tabs [data-tab]').forEach(node => {
    const active = node.dataset.tab === tab;
    node.classList.toggle('active', active);
    node.setAttribute('aria-selected', String(active));
    node.tabIndex = active ? 0 : -1;
  });
  document.querySelectorAll('[data-ldap-panel]').forEach(panel => { panel.hidden = panel.dataset.ldapPanel !== tab; });
}

function notice() {
  const node = element('div', `notice ${state.notice.type}`, state.notice.text);
  node.setAttribute('role', 'status');
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

function navSection(label, views) {
  const section = element('div', 'nav-section');
  section.append(element('span', 'nav-section-label', label));
  const list = element('div', 'nav-section-items');
  views.forEach(([view, text]) => list.append(navButton(view, text)));
  section.append(list);
  return section;
}

async function updateFingerprintStatus(form, peer, value) {
  const status = form.querySelector('[data-fingerprint-status]');
  const submit = form.querySelector('[type="submit"]');
  if (!status) return;
  const token = String(Number(form.dataset.fingerprintToken || 0) + 1);
  form.dataset.fingerprintToken = token;
  if (!String(value || '').trim()) {
    status.textContent = t('fingerprintStatusEmpty');
    status.className = 'fingerprint-status';
    if (submit) submit.disabled = true;
    return;
  }
  const result = await fingerprintCandidates(value);
  if (form.dataset.fingerprintToken !== token) return;
  if (!result.valid) {
    status.textContent = t('fingerprintStatusInvalid');
    status.className = 'fingerprint-status error';
    if (submit) submit.disabled = true;
    return;
  }
  const match = matchingFingerprint(result.candidates, peer?.pk_fingerprint);
  if (match) {
    status.textContent = t('fingerprintStatusMatch');
    status.className = 'fingerprint-status success';
  } else if (peer?.pk_fingerprint) {
    status.textContent = t('fingerprintStatusMismatch');
    status.className = 'fingerprint-status warning';
  } else {
    status.textContent = t('fingerprintStatusReady');
    status.className = 'fingerprint-status';
  }
  if (submit) submit.disabled = state.busy || !state.registryUsers.length;
}

function preserveBindDraft(form, target = null) {
  const peerId = form.dataset.bindPeer;
  if (!peerId) return;
  const data = new FormData(form);
  state.deviceBindDrafts[peerId] = {
    user_id: String(data.get('user_id') || ''),
    pk_fingerprint: String(data.get('pk_fingerprint') || '')
  };
  if (target?.name === 'pk_fingerprint') {
    const peer = state.registeredDevices.find(item => item.peer_id === peerId);
    if (peer) {
      clearTimeout(form._fingerprintTimer);
      form._fingerprintTimer = setTimeout(() => updateFingerprintStatus(form, peer, target.value), 120);
    }
  }
}

function refreshFingerprintStatuses() {
  document.querySelectorAll('[data-bind-peer]').forEach(form => {
    const peer = state.registeredDevices.find(item => item.peer_id === form.dataset.bindPeer);
    const input = form.querySelector('[name="pk_fingerprint"]');
    const submit = form.querySelector('[type="submit"]');
    if (!peer || !input) return;
    if (!input.value.trim()) {
      if (submit) submit.disabled = true;
      return;
    }
    updateFingerprintStatus(form, peer, input.value);
  });
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
    if (state.activeView === 'addressBook') await loadAddressBook();
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
  document.querySelector('#add-peer')?.addEventListener('click', () => openPeerEditor());
  const peerEditorNode = document.querySelector('#peer-editor');
  peerEditorNode?.addEventListener('toggle', () => { state.addressBookFormOpen = peerEditorNode.open; });
  const advancedNode = document.querySelector('#address-book-advanced');
  advancedNode?.addEventListener('toggle', () => { state.addressBookAdvancedOpen = advancedNode.open; });
  document.querySelector('#address-book-search')?.addEventListener('input', event => {
    state.addressBookSearch = event.currentTarget.value;
    renderAddressBookRows();
  });
  document.querySelectorAll('[data-tag-filter]').forEach(node => node.addEventListener('click', () => {
    state.addressBookTagFilter = node.dataset.tagFilter || '';
    updateAddressBookFilterButtons();
    renderAddressBookRows();
  }));
  document.querySelector('#add-peer-tag')?.addEventListener('click', addPeerTag);
  document.querySelector('#peer-new-tag')?.addEventListener('keydown', event => {
    if (event.key === 'Enter') { event.preventDefault(); addPeerTag(); }
  });
  document.querySelector('#peer-new-tag')?.addEventListener('input', event => { state.peerNewTag = event.currentTarget.value; });
  document.querySelector('#peer-selected-tags')?.addEventListener('click', event => {
    const node = event.target.closest('[data-remove-peer-tag]');
    if (node) removePeerTag(node.dataset.removePeerTag);
  });
  document.querySelector('#peer-form')?.addEventListener('submit', savePeer);
  document.querySelector('#peer-form')?.addEventListener('input', preservePeerDraft);
  document.querySelector('#peer-form')?.addEventListener('change', preservePeerDraft);
  document.querySelector('#clear-peer')?.addEventListener('click', () => { clearPeerDraft(); render(); });
  document.querySelectorAll('[data-edit-peer]').forEach(node => {
    node.addEventListener('click', () => editPeer(node.dataset.editPeer));
  });
  document.querySelectorAll('[data-delete-peer]').forEach(node => {
    node.addEventListener('click', () => deletePeer(node.dataset.deletePeer));
  });
  document.querySelector('#address-book-editor')?.addEventListener('input', event => {
    state.addressBookJsonDraft.text = event.currentTarget.value;
    updateAddressBookDraftStatus();
  });
  const tagForm = document.querySelector('#tag-form');
  tagForm?.addEventListener('submit', saveTag);
  tagForm?.addEventListener('input', event => {
    const data = new FormData(event.currentTarget);
    state.tagDraft = {
      ...state.tagDraft,
      revision: state.tagDraft.revision ?? state.addressBookSnapshot?.revision,
      name: String(data.get('name') || ''),
      color: String(data.get('color') || '')
    };
  });
  document.querySelectorAll('[data-edit-tag]').forEach(node => {
    node.addEventListener('click', () => {
      if (state.addressBookBusy || !state.addressBookWritable) return;
      const tag = (state.addressBookSnapshot?.tags || []).find(item => item.name === node.dataset.editTag);
      if (tag) { state.tagDraft = { old_name: tag.name, name: tag.name, color: tag.color || '', revision: state.addressBookSnapshot?.revision }; render(); }
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
  document.querySelectorAll('[data-revoke-session]').forEach(node => {
    node.addEventListener('click', () => revokeSession(node.dataset.revokeSession));
  });

  document.querySelector('#refresh-oauth')?.addEventListener('click', loadOauthProviders);
  const oauthFormNode = document.querySelector('#oauth-provider-form');
  oauthFormNode?.addEventListener('submit', saveOauthProvider);
  oauthFormNode?.addEventListener('input', preserveOauthDraft);
  oauthFormNode?.addEventListener('change', preserveOauthDraft);
  document.querySelector('#cancel-oauth-provider')?.addEventListener('click', () => { state.oauthDraft = null; state.oauthTab = 'identity'; render(); });
  document.querySelectorAll('[data-tab]').forEach(node => {
    if (node.closest('#oauth-tabs')) node.addEventListener('click', () => setOauthTab(node.dataset.tab));
    if (node.closest('#ldap-tabs')) node.addEventListener('click', () => setLdapTab(node.dataset.tab));
  });
  document.querySelectorAll('[data-edit-oauth]').forEach(node => node.addEventListener('click', () => editOauthProvider(node.dataset.editOauth)));
  document.querySelectorAll('[data-toggle-oauth]').forEach(node => node.addEventListener('click', () => toggleOauthProvider(node.dataset.toggleOauth, node.dataset.oauthEnabled === 'true')));
  document.querySelectorAll('[data-delete-oauth]').forEach(node => node.addEventListener('click', () => deleteOauthProvider(node.dataset.deleteOauth)));

  document.querySelector('#refresh-devices')?.addEventListener('click', loadDevices);
  document.querySelector('#device-search')?.addEventListener('input', event => {
    state.deviceSearch = event.currentTarget.value;
    renderDeviceRows();
  });
  document.querySelector('#registry-search')?.addEventListener('submit', event => {
    event.preventDefault();
    state.registrySearch = new FormData(event.currentTarget).get('peer_id') || '';
    loadDevices();
  });
  document.querySelectorAll('[data-bind-peer]').forEach(form => {
    form.addEventListener('submit', bindRegisteredDevice);
    form.addEventListener('input', event => preserveBindDraft(event.currentTarget, event.target));
    form.addEventListener('change', event => preserveBindDraft(event.currentTarget, event.target));
  });
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
    node.addEventListener('click', () => removeGroupMember(node.dataset.removeGroup, node.dataset.userId));
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
    node.addEventListener('click', () => removeDeviceGroupMember(node.dataset.removeDeviceGroup, node.dataset.deviceId));
  });

  document.querySelector('#refresh-ldap')?.addEventListener('click', () => loadLdap(true));
  const ldapFormNode = document.querySelector('#ldap-form');
  ldapFormNode?.addEventListener('submit', saveLdap);
  ldapFormNode?.addEventListener('input', preserveLdapDraft);
  ldapFormNode?.addEventListener('change', preserveLdapDraft);

  refreshFingerprintStatuses();
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
    // A transient public lookup failure must not erase previously loaded choices.
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
  const [serverResult] = await Promise.allSettled([
    api('/api/server-config', { method: 'POST', body: '{}' }),
    loadAddressBook()
  ]);
  if (serverResult.status === 'fulfilled') {
    state.serverConfig = serverResult.value.data || serverResult.value;
  }
}

function jsonDraftChanged() {
  return state.addressBookJsonDraft.text !== state.addressBookJsonDraft.baseText;
}

function updateAddressBookDraftStatus() {
  const status = document.querySelector('#address-book-draft-status');
  const badgeNode = document.querySelector('#address-book-advanced-badge');
  if (!status) return;
  status.replaceChildren();
  const stale = state.addressBookSnapshot && state.addressBookJsonDraft.baseRevision !== state.addressBookSnapshot.revision;
  const changed = jsonDraftChanged() || stale;
  if (badgeNode) badgeNode.hidden = !changed;
  if (!changed) return;
  status.append(element('p', 'empty-state', t(stale ? 'jsonDraftStale' : 'jsonDraftChanged')));
  const discard = button('discard-address-book-draft', t('discardJsonDraft'), 'secondary');
  discard.disabled = state.addressBookBusy;
  discard.addEventListener('click', () => {
    if (!state.addressBookBusy && window.confirm(t('confirmDiscardJsonDraft'))) {
      loadAddressBook({ discardJsonDraft: true });
    }
  });
  status.append(discard);
}

async function fetchAddressBookSnapshot() {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const [book, entries, tags] = await Promise.all([
      api('/api/ab'), api('/api/web/ab/entries'), api('/api/web/ab/tags')
    ]);
    const results = [book, entries, tags];
    if (results.some(result => !Number.isSafeInteger(result?.revision) || result.revision < 1 || typeof result.guid !== 'string' || !result.guid)) {
      throw new Error(t('requestFailed'));
    }
    if (results.some(result => result.revision !== book.revision || result.guid !== book.guid)) {
      if (attempt === 0) continue;
      throw new Error('address_book_revision_conflict');
    }
    const parsed = typeof book.data === 'string' ? JSON.parse(book.data) : book.data;
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object' || !Array.isArray(entries.data) || !Array.isArray(tags.data)
      || entries.data.some(entry => !entry || typeof entry !== 'object' || Array.isArray(entry))
      || tags.data.some(tag => !tag || typeof tag.name !== 'string' || typeof tag.color !== 'string')) {
      throw new Error(t('requestFailed'));
    }
    return { guid: book.guid, revision: book.revision, text: JSON.stringify(parsed, null, 2), entries: entries.data, tags: tags.data };
  }
}

function applyAddressBookSnapshot(snapshot, discardJsonDraft) {
  if (discardJsonDraft || !jsonDraftChanged()) {
    state.addressBookJsonDraft = { text: snapshot.text, baseText: snapshot.text, baseRevision: snapshot.revision };
  }
  state.addressBookSnapshot = snapshot;
  state.addressBookWritable = true;
}

async function addressBookOperation(write, onSaved, discardJsonDraft = false) {
  if (state.addressBookBusy || !state.user || (write && !state.addressBookWritable)) return false;
  const user = state.user;
  const request = ++state.addressBookRequest;
  const current = () => state.user === user && state.addressBookRequest === request;
  let saved = false;
  state.addressBookBusy = true;
  state.busy = true;
  render();
  try {
    if (write) {
      await write();
      if (!current()) return false;
      saved = true;
      state.addressBookWritable = false;
      onSaved?.();
    }
    const snapshot = await fetchAddressBookSnapshot();
    if (!current()) return false;
    applyAddressBookSnapshot(snapshot, discardJsonDraft);
    setNotice(saved ? 'success' : null, saved ? t('saved') : null);
    return true;
  } catch (error) {
    if (current()) {
      if (!write || saved) state.addressBookWritable = false;
      setNotice('error', saved ? t('savedRefreshFailed') : error.message);
    }
    return false;
  } finally {
    if (current()) {
      state.addressBookBusy = false;
      state.busy = false;
      render();
    }
  }
}

async function loadAddressBook({ discardJsonDraft = false } = {}) {
  return addressBookOperation(null, null, discardJsonDraft);
}

async function saveAddressBook() {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const draft = state.addressBookJsonDraft;
  let parsed;
  try {
    parsed = JSON.parse(draft.text);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error();
  } catch {
    setNotice('error', t('invalidJson'));
    render();
    return;
  }
  await addressBookOperation(() => api('/api/ab', {
    method: 'POST', body: JSON.stringify({ data: JSON.stringify(parsed), revision: draft.baseRevision })
  }), null, true);
}

async function savePeer(event) {
  event.preventDefault();
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const data = new FormData(event.currentTarget);
  const tags = Array.isArray(state.addressBookDraft.tags) ? [...state.addressBookDraft.tags] : [];
  const payload = {
    id: state.addressBookDraft.id || undefined,
    revision: state.addressBookDraft.revision ?? state.addressBookSnapshot.revision,
    peer_id: String(data.get('peer-id') || '').trim(),
    username: String(data.get('peer-username') || '').trim(),
    hostname: String(data.get('peer-hostname') || '').trim(),
    alias: String(data.get('peer-alias') || '').trim(),
    platform: String(data.get('peer-platform') || '').trim(),
    tags,
    force_always_relay: event.currentTarget.querySelector('[name="peer-relay"]')?.checked || false
  };
  if (!payload.peer_id) { setNotice('error', t('invalidPeerId')); render(); return; }
  state.addressBookDraft = { ...state.addressBookDraft, ...payload };
  await addressBookOperation(() => api('/api/web/ab/entries', {
    method: 'POST', body: JSON.stringify(payload)
  }), clearPeerDraft);
}

function preservePeerDraft(event) {
  const data = new FormData(event.currentTarget);
  state.addressBookDraft = {
    ...state.addressBookDraft,
    revision: state.addressBookDraft.revision ?? state.addressBookSnapshot?.revision,
    peer_id: String(data.get('peer-id') || ''),
    username: String(data.get('peer-username') || ''),
    hostname: String(data.get('peer-hostname') || ''),
    alias: String(data.get('peer-alias') || ''),
    platform: String(data.get('peer-platform') || ''),
    tags: Array.isArray(state.addressBookDraft.tags) ? state.addressBookDraft.tags : [],
    force_always_relay: event.currentTarget.querySelector('[name="peer-relay"]')?.checked || false
  };
}

function editPeer(id) {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const peer = (state.addressBookSnapshot?.entries || []).find(value => value.id === id || value.peerId === id);
  if (!peer) return;
  openPeerEditor({
    id: peer.id || '',
    revision: state.addressBookSnapshot?.revision,
    peer_id: peer.peerId || peer.id || '',
    username: peer.username || '',
    hostname: peer.hostname || '',
    alias: peer.alias || '',
    platform: peer.platform || '',
    tags: Array.isArray(peer.tags) ? [...peer.tags] : [],
    force_always_relay: Boolean(peer.forceAlwaysRelay)
  });
}

function clearPeerDraft() {
  state.addressBookDraft = emptyPeerDraft();
  state.peerNewTag = '';
  state.addressBookFormOpen = false;
}

async function deletePeer(id) {
  if (!id || state.addressBookBusy || !state.addressBookWritable) return;
  const revision = state.addressBookSnapshot.revision;
  await addressBookOperation(() => api(`/api/web/ab/entries/${encodeURIComponent(id)}?revision=${revision}`, {
    method: 'DELETE'
  }), clearPeerDraft);
}

async function saveTag(event) {
  event.preventDefault();
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const data = new FormData(event.currentTarget);
  const payload = {
    old_name: state.tagDraft.old_name,
    revision: state.tagDraft.revision ?? state.addressBookSnapshot.revision,
    name: String(data.get('name') || '').trim(),
    color: String(data.get('color') || '').trim()
  };
  if (!payload.name) return;
  state.tagDraft = payload;
  await addressBookOperation(() => api('/api/web/ab/tags', {
    method: 'POST', body: JSON.stringify(payload)
  }), () => { state.tagDraft = { name: '', color: '' }; });
}

async function deleteTag(name) {
  if (state.addressBookBusy || !state.addressBookWritable) return;
  const revision = state.addressBookSnapshot.revision;
  await addressBookOperation(() => api('/api/web/ab/tags/delete', {
    method: 'POST', body: JSON.stringify({ name, revision })
  }));
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
  const form = event.currentTarget;
  const peerId = form.dataset.bindPeer;
  const data = Object.fromEntries(new FormData(form));
  const peer = state.registeredDevices.find(item => item.peer_id === peerId);
  const result = await fingerprintCandidates(data.pk_fingerprint);
  if (!result.valid) {
    state.deviceBindDrafts[peerId] = { user_id: String(data.user_id || ''), pk_fingerprint: String(data.pk_fingerprint || '') };
    setNotice('error', t('fingerprintStatusInvalid'));
    render();
    return;
  }
  const pkFingerprint = chooseFingerprint(result.candidates, result.primary, peer?.pk_fingerprint);
  state.deviceBindDrafts[peerId] = { user_id: String(data.user_id || ''), pk_fingerprint: String(data.pk_fingerprint || '') };
  state.busy = true;
  render();
  try {
    await api('/api/admin/device/bind', {
      method: 'POST',
      body: JSON.stringify({ peer_id: peerId, user_id: data.user_id, pk_fingerprint: pkFingerprint })
    });
    delete state.deviceBindDrafts[peerId];
    await loadDevices();
  } catch (error) {
    state.busy = false;
    setNotice('error', error.message);
    render();
  }
}

async function unbindRegisteredDevice(id) {
  const peer = state.registeredDevices.find(item => item.device_id === id);
  if (peer) delete state.deviceBindDrafts[peer.peer_id];
  state.busy = true; render();
  try { await api('/api/admin/device/unbind', { method: 'POST', body: JSON.stringify({ id }) }); await loadDevices(); }
  catch (error) { state.busy = false; setNotice('error', error.message); render(); }
}

async function deleteDevice(id) {
  if (!id) return;
  const device = state.devices.find(item => item.id === id);
  if (device?.peer_id) delete state.deviceBindDrafts[device.peer_id];
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

function preserveOauthDraft(event) {
  const form = event.currentTarget;
  const data = new FormData(form);
  if (event.target?.removeAttribute) event.target.removeAttribute('aria-invalid');
  state.oauthDraft = {
    ...(state.oauthDraft || {}),
    kind: form.elements.kind?.value || state.oauthDraft?.kind || 'oidc',
    name: String(data.get('name') || ''),
    client_id: String(data.get('client_id') || ''),
    client_secret: String(data.get('client_secret') || ''),
    authorization_url: String(data.get('authorization_url') || ''),
    token_url: String(data.get('token_url') || ''),
    userinfo_url: String(data.get('userinfo_url') || ''),
    issuer_url: String(data.get('issuer_url') || ''),
    jwks_url: String(data.get('jwks_url') || ''),
    scopes: String(data.get('scopes') || ''),
    enabled: Boolean(form.elements.enabled?.checked)
  };
}

async function saveOauthProvider(event) {
  event.preventDefault();
  const form = event.currentTarget;
  const data = Object.fromEntries(new FormData(form));
  const editing = state.oauthDraft?.id;
  const required = [
    ['name', 'identity'],
    ['client_id', 'client'],
    ['authorization_url', 'endpoints'],
    ['token_url', 'endpoints'],
    ['userinfo_url', 'endpoints']
  ];
  if (!editing) required.push(['client_secret', 'client']);
  const missing = required.find(([key]) => !String(data[key] || '').trim());
  if (missing) {
    state.oauthTab = missing[1];
    setNotice('error', t('requiredField'));
    render();
    requestAnimationFrame(() => {
      const input = document.querySelector(`#oauth-${missing[0]}`);
      input?.focus();
      input?.setAttribute('aria-invalid', 'true');
    });
    return;
  }
  data.enabled = Boolean(form.elements.enabled?.checked);
  data.kind = form.elements.kind?.value || 'oidc';
  if (editing) data.id = editing;
  state.busy = true;
  render();
  try {
    await api(editing ? '/api/admin/oauth/providers/update' : '/api/admin/oauth/providers', {
      method: 'POST', body: JSON.stringify(data)
    });
    state.oauthDraft = null;
    state.oauthTab = 'identity';
    setNotice('success', t('providerSaved'));
    await loadOauthProviders();
  } catch (error) {
    setNotice('error', error.message);
    state.busy = false;
    render();
  }
}

function editOauthProvider(id) {
  const provider = state.oauthProviders.find(item => String(item.id || item.name) === String(id));
  if (provider) {
    state.oauthDraft = { ...provider, client_secret: '' };
    state.oauthTab = 'identity';
    render();
  }
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
    await loadLoginOptions();
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
  const user = state.user;
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
      if (state.user === user) {
        clearSession();
        render();
      }
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
  state.oauthRedirectConfigured = false;
  state.oauthDraft = null;
  state.oauthTab = 'identity';
  state.devices = [];
  state.registeredDevices = [];
  state.registryUsers = [];
  state.registrySearch = '';
  state.deviceSearch = '';
  state.deviceBindDrafts = {};
  state.groups = [];
  state.groupMemberships = [];
  state.groupUsers = [];
  state.groupDraft = '';
  state.addressBookSnapshot = null;
  state.addressBookJsonDraft = { text: '', baseText: '', baseRevision: null };
  state.addressBookWritable = false;
  state.addressBookBusy = false;
  state.addressBookRequest += 1;
  state.addressBookSearch = '';
  state.addressBookTagFilter = '';
  state.addressBookFormOpen = false;
  state.addressBookAdvancedOpen = false;
  state.peerNewTag = '';
  state.busy = false;
  state.addressBookDraft = emptyPeerDraft();
  state.tagDraft = { name: '', color: '' };
  state.deviceGroups = [];
  state.deviceGroupMemberships = [];
  state.deviceGroupDevices = [];
  state.deviceGroupDraft = '';
  state.deviceGroupsRequest += 1;
  state.devicesRequest += 1;
  state.ldap = null;
  state.ldapDraft = null;
  state.ldapTab = 'connection';
  state.serverConfig = null;
  state.activeView = 'profile';
  sessionStorage.removeItem(TOKEN_KEY);
}

render();
Promise.all([loadCurrentUser(false), loadLoginOptions()]).then(() => render());
