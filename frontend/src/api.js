export const WS_URL = window.location.origin.replace(/^http/, 'ws');

async function call(method, url, body, token) {
  const headers = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const res = await fetch(url, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return text ? JSON.parse(text) : null;
}

export const auth = {
  login:    (email, password)         => call('POST', '/api/auth/login',    { email, password }),
  register: (email, password)         => call('POST', '/api/auth/register', { email, password }),
  me:       (token)                   => call('GET',  '/api/auth/me',       undefined, token),
  projects: (token)                   => call('GET',  '/api/auth/projects', undefined, token),
  create:   (name, token)             => call('POST', '/api/auth/projects', { name }, token),
  share:    (pid, email, role, token) =>
    call('POST', `/api/auth/projects/${pid}/access`, { email, role }, token),
};

export const crdt = {
  state: (docId, token) =>
    call('GET', `/api/crdt/documents/${docId}/state`, undefined, token),
};

export async function executeCode(language, code, token) {
  const res = await fetch('/api/exec/execute', {
    method:  'POST',
    headers: {
      'Content-Type':  'application/json',
      'Authorization': `Bearer ${token}`,
    },
    body: JSON.stringify({ language, code, timeout_seconds: 10 }),
  });
  const data = await res.json();
  return { status: res.status, data };
}
