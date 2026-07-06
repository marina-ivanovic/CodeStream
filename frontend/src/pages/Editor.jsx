import { useState, useEffect, useRef, useCallback } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { python }     from '@codemirror/lang-python';
import { javascript } from '@codemirror/lang-javascript';
import { oneDark }    from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';

import { remoteCursorsField, setCursorEffect } from '../lib/remoteCursors';
import { crdt, executeCode, auth, WS_URL } from '../api';
import { jsPDF } from 'jspdf';

const userColorMap = new Map();

function userColor(userId) {
  if (!userId) return '#ccc';

  const palette = [
    '#ef4444',
    '#3b82f6',
    '#10b981',
    '#f59e0b',
    '#8b5cf6',
    '#ec4899',
    '#06b6d4',
  ];

  if (userColorMap.has(userId)) {
    return userColorMap.get(userId);
  }

  const nextColor = palette[userColorMap.size % palette.length];
  userColorMap.set(userId, nextColor);
  return nextColor;
}

function charPosToLine(pos, text) {
  if (!text || pos == null) return 1;
  return text.slice(0, Math.min(pos, text.length)).split('\n').length;
}

function charsToText(chars) {
  return chars.filter(c => !c.deleted).map(c => c.content).join('');
}

function applyRemoteOp(op, chars) {
  if (op.type === 'insert') {
    const exists = chars.some(
      c => c.id.clock === op.char_id.clock && c.id.user_id === op.char_id.user_id
    );
    if (exists) return;
    const entry = { id: { ...op.char_id }, content: op.content, deleted: false };
    if (!op.after_char_id) {
      chars.unshift(entry);
    } else {
      const idx = chars.findIndex(
        c => c.id.clock === op.after_char_id.clock && c.id.user_id === op.after_char_id.user_id
      );
      chars.splice(idx !== -1 ? idx + 1 : chars.length, 0, entry);
    }
  } else if (op.type === 'delete') {
    const c = chars.find(
      c => c.id.clock === op.char_id.clock && c.id.user_id === op.char_id.user_id
    );
    if (c) c.deleted = true;
  }
}

let tempSeq = 0;

export default function Editor({ token, user, project, onBack, onLogout }) {
  const [code, setCode] = useState('');
  const [language, setLanguage] = useState('python');
  const [wsStatus, setWsStatus] = useState('disconnected');
  const [output, setOutput] = useState(null);
  const [running, setRunning] = useState(false);
  const [collabs, setCollabs] = useState(new Map());
  const [shareOpen, setShareOpen] = useState(false);
  const [shareEmail, setShareEmail] = useState('');
  const [shareRole, setShareRole] = useState('write');
  const [shareMsg, setShareMsg] = useState('');

  const isReadOnly = project.role === 'read';

  const wsRef = useRef(null);
  const charsRef = useRef([]);
  const lastTextRef = useRef('');
  const viewRef = useRef(null);
  const suppressRef = useRef(false);

  const opQueueRef = useRef([]);
  const inflightRef = useRef(false);
  const inflightTempRef = useRef(null);

  function drain() {
    if (inflightRef.current) return;
    if (!opQueueRef.current.length) return;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;

    const op = opQueueRef.current.shift();
    inflightRef.current = true;
    inflightTempRef.current = op.tempEntry ?? null;

    ws.send(JSON.stringify(
      op.type === 'delete'
        ? { type: 'delete', char_id: op.char_id }
        : { type: 'insert', after_char_id: op.after_char_id, content: op.content }
    ));
  }

  useEffect(() => {
    let ws;
    let isMounted = true;

    (async () => {
      try {
        const { chars } = await crdt.state(project.id, token);
        if (!isMounted) return;
        charsRef.current = (chars ?? []).map(c => ({ ...c, id: { ...c.id } }));
        const text = charsToText(charsRef.current);
        setCode(text);
        lastTextRef.current = text;
      } catch {}

      if (!isMounted) return;

      ws = new WebSocket(`${WS_URL}/ws/${project.id}?token=${encodeURIComponent(token)}`);
      wsRef.current = ws;

      ws.onopen = () => setWsStatus('connected');
      ws.onclose = () => { setWsStatus('disconnected'); inflightRef.current = false; };
      ws.onerror = () => setWsStatus('error');

      ws.onmessage = (event) => {
        let envelope;
        try { envelope = JSON.parse(event.data); } catch { return; }

        const { from, from_email, data } = envelope;
        const isMe = from === user.id;

        if (data.cursor_position !== undefined) {
          const pos = typeof data.cursor_position === 'number' ? data.cursor_position : null;
          setCollabs(prev => {
            const next = new Map(prev);
            next.set(from, { email: from_email || from, pos });
            return next;
          });
          if (!isMe && viewRef.current && pos !== null) {
            const docLen = viewRef.current.state.doc.length;
            viewRef.current.dispatch({
              effects: setCursorEffect.of({
                userId: from,
                pos: Math.min(pos, docLen),
                color: userColor(from),
                label: (from_email || from).split('@')[0],
              }),
            });
          }
          return;
        }

        if (data.disconnect) {
          setCollabs(prev => { const n = new Map(prev); n.delete(from); return n; });
          if (viewRef.current) {
            viewRef.current.dispatch({ effects: removeCursorEffect.of(from) });
          }
          return;
        }

        if (data.type === 'language' && data.language) {
          setLanguage(data.language);
          return;
        }

        const { resolved_operation, document_text } = data;
        if (!resolved_operation || document_text === undefined) return;

        if (isMe) {
          if (resolved_operation.type === 'insert' && inflightTempRef.current) {
            const real = resolved_operation.char_id;
            const temp = inflightTempRef.current.id;
            temp.clock = real.clock;
            temp.user_id = real.user_id;
            delete temp.temp;
            inflightTempRef.current = null;
          }
          lastTextRef.current = document_text;
          inflightRef.current = false;
          drain();
        } else {
          applyRemoteOp(resolved_operation, charsRef.current);
          const mergedText = charsToText(charsRef.current);
          lastTextRef.current = document_text;

          if (viewRef.current) {
            suppressRef.current = true;
            const view = viewRef.current;
            const cur = view.state.doc.toString();
            if (cur !== mergedText) {
              view.dispatch({ changes: { from: 0, to: cur.length, insert: mergedText } });
            }
            suppressRef.current = false;
          }
          setCode(mergedText);

          setCollabs(prev => {
            const next = new Map(prev);
            const existing = next.get(from) ?? {};
            next.set(from, { ...existing, email: from_email || from });
            return next;
          });
        }
      };
    })();

    return () => {
      isMounted = false;
      if (ws?.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'disconnect' }));
      }
      ws?.close();
    };
  }, [project.id, token, user.id]);

  const onUpdate = useCallback((viewUpdate) => {
    if (!viewUpdate.selectionSet) return;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    const pos = viewUpdate.state.selection.main.head;
    ws.send(JSON.stringify({ type: 'cursor', position: pos }));
  }, []);

  const handleChange = useCallback((newText, viewUpdate) => {
    if (isReadOnly) return;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;

    if (suppressRef.current) return;
    const isUserTyped = viewUpdate.transactions.some(
      tr => tr.isUserEvent('input') || tr.isUserEvent('delete') || tr.isUserEvent('paste'),
    );
    if (!isUserTyped) return;

    for (const tr of viewUpdate.transactions) {
      if (!tr.docChanged) continue;

      tr.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
        const visible = charsRef.current.filter(c => !c.deleted);

        for (let i = toA - 1; i >= fromA; i--) {
          const target = visible[i];
          if (!target) continue;
          target.deleted = true;
          opQueueRef.current.push({ type: 'delete', char_id: target.id });
        }

        const text = inserted.sliceString(0);
        for (let ci = 0; ci < text.length; ci++) {
          const vis = charsRef.current.filter(c => !c.deleted);
          const afterIdx = fromA - 1 + ci;
          const afterEntry = afterIdx >= 0 ? vis[afterIdx] : null;
          const after_char_id = afterEntry ? afterEntry.id : null;

          const tempEntry = {
            id: { clock: -(++tempSeq), user_id: 'pending', temp: true },
            content: text[ci],
            deleted: false,
          };

          if (!afterEntry) {
            charsRef.current.unshift(tempEntry);
          } else {
            const rawIdx = charsRef.current.indexOf(afterEntry);
            charsRef.current.splice(
              rawIdx !== -1 ? rawIdx + 1 : charsRef.current.length,
              0,
              tempEntry,
            );
          }

          opQueueRef.current.push({ type: 'insert', after_char_id, content: text[ci], tempEntry });
        }
      });
    }

    lastTextRef.current = newText;
    drain();
  }, [isReadOnly]);

  const handleRun = async () => {
    const currentCode = viewRef.current?.state.doc.toString() ?? code;
    setRunning(true); setOutput(null);
    try {
      const { status, data } = await executeCode(language, currentCode, token);
      if (status === 408) {
        setOutput({ error: 'Execution timed out (10 s limit).' });
      } else if (status < 200 || status >= 300) {
        setOutput({ error: typeof data === 'string' ? data : JSON.stringify(data) });
      } else {
        setOutput(data);
      }
    } catch (err) {
      setOutput({ error: err.message });
    } finally { setRunning(false); }
  };

  const handleLanguageChange = (e) => {
    const newLang = e.target.value;
    setLanguage(newLang);

    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'language', language: newLang }));
    }
  };

  const handleExportPdf = () => {
    const currentCode = viewRef.current?.state.doc.toString() ?? code;
    if (!currentCode.trim()) { alert('Nothing to export — editor is empty.'); return; }

    const doc = new jsPDF({ orientation: 'portrait', unit: 'mm', format: 'a4' });
    const W = doc.internal.pageSize.getWidth();
    const H = doc.internal.pageSize.getHeight();

    doc.setFont('helvetica', 'bold'); doc.setFontSize(15);
    doc.text(project.name, 10, 14);

    doc.setFont('helvetica', 'normal'); doc.setFontSize(9);
    doc.setTextColor(110, 110, 110);
    doc.text(`Language: ${language}    Exported: ${new Date().toLocaleString()}`, 10, 21);
    doc.setTextColor(0);
    doc.setDrawColor(200, 200, 200); doc.line(10, 24, W - 10, 24);

    doc.setFont('courier', 'normal'); doc.setFontSize(9);
    const lines = doc.splitTextToSize(currentCode, W - 20);
    const lh = 4.8;
    let y = 30;

    for (const line of lines) {
      if (y + lh > H - 12) { doc.addPage(); y = 12; }
      doc.text(line, 10, y); y += lh;
    }
    doc.save(`${project.name.replace(/[^a-z0-9_\-]/gi, '_')}.pdf`);
  };

  const handleShare = async (e) => {
    e.preventDefault(); setShareMsg('');
    try {
      await auth.share(project.id, shareEmail, shareRole, token);
      setShareMsg(`✓ Access granted to ${shareEmail}`);
      setShareEmail('');
    } catch (err) {
      setShareMsg(`Error: ${err.message}`);
    }
  };

  const extensions = [
    language === 'python' ? python() : javascript(),
    remoteCursorsField,
    EditorView.editable.of(!isReadOnly),
    oneDark,
  ];

  const collabList = [...collabs.entries()];

  return (
    <div className="editor-root">
      <header className="editor-bar">
        <button onClick={onBack}>← Projects</button>
        <span className="project-title">{project.name}</span>
        {isReadOnly && <span className="read-only-badge">Read-only</span>}
        <span className={`ws-dot ${wsStatus}`} title={wsStatus}>●</span>

        <div className="editor-actions">
          <select value={language} onChange={handleLanguageChange}>
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
          </select>
          <button className="btn-primary" onClick={handleRun} disabled={running}>
            {running ? '⟳ Running…' : '▶ Run'}
          </button>
          <button onClick={handleExportPdf}>⤓ PDF</button>
          {project.role === 'owner' && (
            <button onClick={() => { setShareOpen(true); setShareMsg(''); }}>Share</button>
          )}
          <button onClick={onLogout}>Logout</button>
        </div>
      </header>

      <div className="editor-body">
        <div className="editor-main">
          <CodeMirror
            value={code}
            extensions={extensions}
            onChange={handleChange}
            onUpdate={onUpdate}
            onCreateEditor={view => { viewRef.current = view; }}
            style={{ flex: 1, overflow: 'hidden' }}
            height="100%"
          />

          <div className="output-panel">
            <div className="panel-title">
              Output
              {output?.execution_time_ms != null && (
                <span className="exec-time">{output.execution_time_ms} ms</span>
              )}
            </div>
            <div className="output-scroll">
              {!output && <p className="out-empty">Run your code to see output here.</p>}
              {output?.error && <pre className="out-block stderr">{output.error}</pre>}
              {output?.stdout != null && (
                <pre className="out-block">{output.stdout || '(no output)'}</pre>
              )}
              {output?.stderr && <pre className="out-block stderr">{output.stderr}</pre>}
              {output?.exit_code != null && (
                <div className="out-meta">Exit code: {output.exit_code}</div>
              )}
            </div>
          </div>
        </div>

        <aside className="editor-sidebar">
          <div className="sidebar-title">Collaborators</div>
          <div className="sidebar-body">
            <div className="collab-item">
              <span className="c-dot" style={{ color: userColor(user.id) }}>●</span>
              <span className="c-email">{user.email}</span>
              <span className="c-you">you</span>
            </div>

            {collabList
              .filter(([id]) => id !== user.id)
              .map(([id, info]) => (
                <div key={id} className="collab-item">
                  <span className="c-dot" style={{ color: userColor(id) }}>●</span>
                  <span className="c-email" title={info.email}>{info.email}</span>
                  {info.pos != null && (
                    <span className="c-line">
                      ln {charPosToLine(info.pos, lastTextRef.current)}
                    </span>
                  )}
                </div>
              ))}

            {collabList.filter(([id]) => id !== user.id).length === 0 && (
              <p className="sidebar-empty">No other users connected.</p>
            )}
          </div>
        </aside>
      </div>

      {shareOpen && (
        <div className="modal-overlay" onClick={() => setShareOpen(false)}>
          <div className="modal-box" onClick={e => e.stopPropagation()}>
            <h3>Share "{project.name}"</h3>
            <form className="modal-form" onSubmit={handleShare}>
              <input
                type="email" placeholder="Collaborator email" required
                value={shareEmail} onChange={e => setShareEmail(e.target.value)}
                autoFocus
              />
              <select value={shareRole} onChange={e => setShareRole(e.target.value)}>
                <option value="read">Read-only</option>
                <option value="write">Can edit</option>
              </select>
              <div className="modal-row">
                <button type="button" onClick={() => setShareOpen(false)}>Cancel</button>
                <button type="submit" className="btn-primary">Grant Access</button>
              </div>
            </form>
            {shareMsg && (
              <p className="modal-msg" style={{ color: shareMsg.startsWith('✓') ? 'var(--ok)' : 'var(--err)' }}>
                {shareMsg}
              </p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
        {/* Collaborators sidebar */}
