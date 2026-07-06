import { useState, useEffect } from 'react';
import { auth } from '../api';

function formatDate(iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  return isNaN(d.getTime()) ? '—' : d.toLocaleDateString();
}

export default function Projects({ token, user, onOpenProject, onLogout }) {
  const [projects,    setProjects]    = useState([]);
  const [loading,     setLoading]     = useState(true);
  const [error,       setError]       = useState('');
  const [createOpen,  setCreateOpen]  = useState(false);
  const [newName,     setNewName]     = useState('');
  const [creating,    setCreating]    = useState(false);

  const load = async () => {
    setLoading(true); setError('');
    try {
      setProjects(await auth.projects(token));
    } catch (err) {
      setError(err.message);
    } finally { setLoading(false); }
  };

  useEffect(() => { load(); }, []);

  const handleCreate = async (e) => {
    e.preventDefault();
    setCreating(true);
    try {
      const project = await auth.create(newName.trim(), token);
      setCreateOpen(false);
      setNewName('');
      await load();
      onOpenProject(project);
    } catch (err) {
      alert(`Could not create project: ${err.message}`);
    } finally { setCreating(false); }
  };

  return (
    <div className="projects-root">
      <header className="top-bar">
        <span className="logo">CodeStream</span>
        <span className="user-chip">{user.email}</span>
        <button onClick={onLogout}>Logout</button>
      </header>

      <div className="projects-wrap">
        <div className="projects-header">
          <h2>My Projects</h2>
          <button className="btn-primary" onClick={() => setCreateOpen(true)}>+ New Project</button>
        </div>

        <div className="projects-grid">
          {loading && <p className="empty-state">Loading…</p>}
          {error   && <p className="empty-state" style={{ color: 'var(--err)' }}>Error: {error}</p>}
          {!loading && !error && projects.length === 0 && (
            <p className="empty-state">No projects yet — create one to get started!</p>
          )}
          {projects.map(p => (
            <div key={p.id} className="project-card" onClick={() => onOpenProject(p)}>
              <div className="pname">{p.name}</div>
              <div className="pmeta">
                <span className={`role-badge ${p.role}`}>{p.role}</span>
                <span>{formatDate(p.created_at)}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {createOpen && (
        <div className="modal-overlay" onClick={() => setCreateOpen(false)}>
          <div className="modal-box" onClick={e => e.stopPropagation()}>
            <h3>New Project</h3>
            <form className="modal-form" onSubmit={handleCreate}>
              <input
                type="text" placeholder="Project name" required maxLength={80}
                value={newName} onChange={e => setNewName(e.target.value)}
                autoFocus
              />
              <div className="modal-row">
                <button type="button" onClick={() => setCreateOpen(false)}>Cancel</button>
                <button type="submit" className="btn-primary" disabled={creating}>
                  {creating ? 'Creating…' : 'Create'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
