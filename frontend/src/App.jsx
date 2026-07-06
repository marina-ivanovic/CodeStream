import { useState, useEffect } from 'react';
import Auth from './pages/Auth';
import Projects from './pages/Projects';
import Editor from './pages/Editor';
import { auth } from './api';

export default function App() {
  const [session, setSession] = useState(() => {
    const token = localStorage.getItem('cs_token');
    const user  = localStorage.getItem('cs_user');
    return token && user ? { token, user: JSON.parse(user) } : null;
  });
  const [project, setProject] = useState(null);

  useEffect(() => {
    if (!session) return;
    auth.me(session.token).catch(() => {
      localStorage.removeItem('cs_token');
      localStorage.removeItem('cs_user');
      setSession(null);
    });
  }, []);

  const handleLogin = (token, user) => {
    localStorage.setItem('cs_token', token);
    localStorage.setItem('cs_user', JSON.stringify(user));
    setSession({ token, user });
  };

  const handleLogout = () => {
    localStorage.removeItem('cs_token');
    localStorage.removeItem('cs_user');
    setSession(null);
    setProject(null);
  };

  if (!session) return <Auth onLogin={handleLogin} />;

  if (project) {
    return (
      <Editor
        token={session.token}
        user={session.user}
        project={project}
        onBack={() => setProject(null)}
        onLogout={handleLogout}
      />
    );
  }

  return (
    <Projects
      token={session.token}
      user={session.user}
      onOpenProject={setProject}
      onLogout={handleLogout}
    />
  );
}
