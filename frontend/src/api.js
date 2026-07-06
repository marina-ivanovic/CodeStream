const AUTH = 'http://localhost:3000';
const CRDT  = 'http://localhost:3002';
const EXEC  = 'http://localhost:3003';

export const WS_URL = 'ws://localhost:3001';

async function call(method, url, body, token) {
  const headers = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const res  = await fetch(url, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  if (!res.ok) throw new Error(text || `HTTP ${res.status}`);
  return text ? JSON.parse(text) : null;
}

export const auth = {
  login:    (email, password)              => call('POST', `${AUTH}/login`,    { email, password }),
  register: (email, password)              => call('POST', `${AUTH}/register`, { email, password }),
  me:       (token)                        => call('GET',  `${AUTH}/me`,       undefined, token),
  projects: (token)                        => call('GET',  `${AUTH}/projects`, undefined, token),
  create:   (name, token)                  => call('POST', `${AUTH}/projects`, { name }, token),
  share:    (pid, email, role, token)      =>
    call('POST', `${AUTH}/projects/${pid}/access`, { email, role }, token),
};

export const crdt = {
  state: (docId, token) =>
    call('GET', `${CRDT}/documents/${docId}/state`, undefined, token),
};

export async function executeCode(language, code, token) {
  const res = await fetch(`${EXEC}/execute`, {
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
