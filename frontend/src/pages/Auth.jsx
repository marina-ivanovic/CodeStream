import { useState } from 'react';
import { auth } from '../api';

export default function Auth({ onLogin }) {
  const [tab,      setTab]      = useState('login');
  const [email,    setEmail]    = useState('');
  const [password, setPassword] = useState('');
  const [msg,      setMsg]      = useState({ text: '', ok: false });
  const [loading,  setLoading]  = useState(false);

  const switchTab = (t) => { setTab(t); setMsg({ text: '', ok: false }); };

  const handleLogin = async (e) => {
    e.preventDefault();
    setMsg({ text: '', ok: false }); setLoading(true);
    try {
      const data = await auth.login(email, password);
      onLogin(data.token, data.user);
    } catch (err) {
      setMsg({ text: `Login failed: ${err.message}`, ok: false });
    } finally { setLoading(false); }
  };

  const handleRegister = async (e) => {
    e.preventDefault();
    setMsg({ text: '', ok: false }); setLoading(true);
    try {
      await auth.register(email, password);
      setMsg({ text: 'Account created — please sign in.', ok: true });
      setPassword('');
      switchTab('login');
    } catch (err) {
      setMsg({ text: `Registration failed: ${err.message}`, ok: false });
    } finally { setLoading(false); }
  };

  return (
    <div className="auth-root">
      <div className="auth-card">
        <h1 className="auth-logo">CodeStream</h1>
        <p className="auth-subtitle">Distributed collaborative code editor</p>

        <div className="tab-row">
          <button className={`tab-btn ${tab === 'login'    ? 'active' : ''}`} onClick={() => switchTab('login')}>Sign In</button>
          <button className={`tab-btn ${tab === 'register' ? 'active' : ''}`} onClick={() => switchTab('register')}>Register</button>
        </div>

        <form className="auth-form" onSubmit={tab === 'login' ? handleLogin : handleRegister}>
          <input
            type="email" placeholder="Email" value={email} required
            onChange={e => setEmail(e.target.value)} autoComplete="email"
          />
          <input
            type="password"
            placeholder={tab === 'register' ? 'Password (min 8 chars)' : 'Password'}
            value={password} required minLength={tab === 'register' ? 8 : undefined}
            onChange={e => setPassword(e.target.value)}
            autoComplete={tab === 'login' ? 'current-password' : 'new-password'}
          />
          <button type="submit" className="submit-btn" disabled={loading}>
            {loading ? 'Please wait…' : tab === 'login' ? 'Sign In' : 'Create Account'}
          </button>
        </form>

        {msg.text && <p className={`auth-msg ${msg.ok ? 'ok' : 'err'}`}>{msg.text}</p>}
      </div>
    </div>
  );
}
