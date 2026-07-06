import { WidgetType, Decoration, EditorView } from '@codemirror/view';
import { StateEffect, StateField } from '@codemirror/state';

export const setCursorEffect    = StateEffect.define();
export const removeCursorEffect = StateEffect.define();

class CursorWidget extends WidgetType {
  constructor(color, label) {
    super();
    this.color = color;
    this.label = label;
  }

  toDOM() {
    const wrap = document.createElement('span');
    wrap.setAttribute('aria-hidden', 'true');
    wrap.style.cssText = [
      'position: relative',
      `border-left: 2px solid ${this.color}`,
      'margin-left: -1px',
      'height: 1.1em',
      'display: inline-block',
      'vertical-align: text-bottom',
      'pointer-events: none',
    ].join(';');

    const label = document.createElement('span');
    label.textContent = this.label;
    label.style.cssText = [
      'position: absolute',
      'bottom: 100%',
      'left: 0',
      `background: ${this.color}`,
      'color: #fff',
      'font-size: 10px',
      'font-family: system-ui, sans-serif',
      'padding: 1px 5px',
      'border-radius: 3px 3px 3px 0',
      'white-space: nowrap',
      'pointer-events: none',
      'z-index: 999',
      'line-height: 1.5',
    ].join(';');

    wrap.appendChild(label);
    return wrap;
  }

  eq(other) {
    return this.color === other.color && this.label === other.label;
  }

  ignoreEvent() { return true; }
}

export const remoteCursorsField = StateField.define({
  create: () => ({ cursors: new Map(), decorations: Decoration.none }),

  update({ cursors, decorations }, tr) {
    let next    = cursors;
    let changed = false;

    for (const eff of tr.effects) {
      if (eff.is(setCursorEffect)) {
        next    = new Map(next);
        const { userId, pos, color, label } = eff.value;
        next.set(userId, { pos: Math.max(0, Math.min(pos, tr.state.doc.length)), color, label });
        changed = true;
      } else if (eff.is(removeCursorEffect)) {
        next = new Map(next);
        next.delete(eff.value);
        changed = true;
      }
    }

    if (!changed && tr.docChanged && next.size > 0) {
      next = new Map();
      for (const [id, data] of cursors) {
        next.set(id, { ...data, pos: tr.changes.mapPos(data.pos) });
      }
      changed = true;
    }

    if (!changed) return { cursors, decorations };

    const decs = [];
    for (const [, { pos, color, label }] of next) {
      const p = Math.max(0, Math.min(pos, tr.state.doc.length));
      decs.push(
        Decoration.widget({ widget: new CursorWidget(color, label), side: 1 }).range(p)
      );
    }
    decs.sort((a, b) => a.from - b.from);

    return { cursors: next, decorations: Decoration.set(decs) };
  },

  provide: f => EditorView.decorations.from(f, v => v.decorations),
});
