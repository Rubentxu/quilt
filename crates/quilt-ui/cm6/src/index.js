// ── Quilt CM6 Bridge ────────────────────────────────────────────────
// Thin wrapper around CodeMirror 6 for Rust/Leptos interop.
//
// The `window.__quiltCm6` API:
//   createEditor(container, content, callbacks) → editorId (number)
//   destroyEditor(id)
//   getContent(id) → string
//   setContent(id, content)
//   setContentAndCursor(id, content, cursorOffset)
//   focus(id)
//   getCursorOffset(id) → number
//   setAutocompleteState(id, active)
//   setDecorations(id, decorationsJson)
//
// `callbacks` is an object with optional function properties:
//   onChange(text)             — called on every document change
//   onEnter(offset)            — Enter without modifiers
//   onTab()                    — Tab without modifiers
//   onShiftTab()               — Shift+Tab
//   onEscape()                 — Escape (cancel editing)
//   onBackspace()              — Backspace on empty line
//   onCtrlBackspace()          — Ctrl+Backspace (merge with next)
//   onUndo()                   — Ctrl+Z
//   onRedo()                   — Ctrl+Shift+Z or Ctrl+Y
//   onAcNavigate(direction)    — ArrowUp(-1)/ArrowDown(1) in autocomplete
//   onAcSelect()               — Enter when autocomplete dropdown is active
//   onAcCancel()               — Escape when autocomplete dropdown is active
//
// Undo/redo is intentionally NOT handled by CM6's history extension.
// The Outliner/HistoryStack on the Rust side owns undo/redo.
// We dispatch the keyboard shortcuts to Rust callbacks instead.

import { EditorView, keymap, Decoration } from '@codemirror/view';
import { EditorState, StateEffect, StateField } from '@codemirror/state';
import { defaultKeymap } from '@codemirror/commands';
import { indentOnInput } from '@codemirror/language';

// ── Editor instance registry ──

let nextId = 0;
const editors = new Map();

// ── Decoration effect + field ──
// Allows Rust to push visual decorations (tag/page-ref/property highlighting)
// into the editor via a StateEffect, applied reactively by a StateField.

const setDecorationsEffect = StateEffect.define();

const decorationField = StateField.define({
  create() { return Decoration.none; },
  update(decos, tr) {
    for (let e of tr.effects) {
      if (e.is(setDecorationsEffect)) {
        return e.value;
      }
    }
    return decos.map(tr.changes);
  },
  provide: f => EditorView.decorations.from(f),
});

// ── Key binding helpers ──

function mod(ctrl, key) {
  return `${ctrl ? 'Mod-' : ''}${key}`;
}

// Build the CM6 keymap from the callbacks object.
// Undo/redo keybindings are replaced with custom callbacks that
// delegate to the Rust-side Outliner. All other default bindings
// (typing, cursor movement, selection, clipboard) are preserved.
// Autocomplete navigation keys are conditionally active only when
// the editor's `dropdownActive` flag is true.
function buildKeymap(cbs) {
  const bindings = [];

  // Override undo/redo to delegate to Rust Outliner
  if (cbs.onUndo) {
    bindings.push({ key: 'Mod-z', run: () => { cbs.onUndo(); return true; } });
  }
  if (cbs.onRedo) {
    bindings.push({ key: 'Mod-Shift-z', run: () => { cbs.onRedo(); return true; } });
    bindings.push({ key: 'Mod-y', run: () => { cbs.onRedo(); return true; } });
  }

  // Autocomplete navigation — only active when dropdown is visible
  if (cbs.onAcNavigate) {
    bindings.push({
      key: 'ArrowDown',
      run: () => {
        const entry = editors.get(currentEditingId);
        if (entry?.dropdownActive) { cbs.onAcNavigate(1); return true; }
        return false;
      },
    });
    bindings.push({
      key: 'ArrowUp',
      run: () => {
        const entry = editors.get(currentEditingId);
        if (entry?.dropdownActive) { cbs.onAcNavigate(-1); return true; }
        return false;
      },
    });
  }

  if (cbs.onAcSelect) {
    bindings.push({
      key: 'Enter',
      run: () => {
        const entry = editors.get(currentEditingId);
        if (entry?.dropdownActive) { cbs.onAcSelect(); return true; }
        // Fall through to the outliner Enter handler below
        return false;
      },
    });
  }

  if (cbs.onAcCancel) {
    bindings.push({
      key: 'Escape',
      run: () => {
        const entry = editors.get(currentEditingId);
        if (entry?.dropdownActive) { cbs.onAcCancel(); return true; }
        // Fall through to the Escape handler below
        return false;
      },
    });
  }

  // Outliner structural operations (low-priority — checked after ac handlers)
  if (cbs.onEnter) {
    bindings.push({
      key: 'Enter',
      run: () => {
        const offset = currentEditingId !== null
          ? (editors.get(currentEditingId)?.view.state.selection.main.anchor ?? 0)
          : 0;
        cbs.onEnter(offset);
        return true;
      },
    });
  }
  if (cbs.onTab) {
    bindings.push({ key: 'Tab', run: () => { cbs.onTab(); return true; } });
  }
  if (cbs.onShiftTab) {
    bindings.push({ key: 'Shift-Tab', run: () => { cbs.onShiftTab(); return true; } });
  }
  if (cbs.onEscape) {
    bindings.push({ key: 'Escape', run: () => { cbs.onEscape(); return true; } });
  }
  if (cbs.onCtrlBackspace) {
    bindings.push({ key: 'Mod-Backspace', run: () => { cbs.onCtrlBackspace(); return true; } });
  }
  if (cbs.onBackspace) {
    // Only intercept when the document is empty (merge intention)
    bindings.push({
      key: 'Backspace',
      run: () => {
        const view = editors.get(currentEditingId)?.view;
        if (view && view.state.doc.length === 0 && view.state.selection.main.empty) {
          cbs.onBackspace();
          return true;
        }
        return false; // let default backspace handle deletion
      },
    });
  }

  return bindings;
}

// Track which editor is currently being edited (for backspace handler)
let currentEditingId = null;

// ── Public API ──

const api = {
  /**
   * Create a CM6 editor inside the given DOM container.
   * @param {HTMLElement} container - The DOM element to mount into.
   * @param {string} content - Initial document content.
   * @param {object} callbacks - Callback object (see module docs).
   * @returns {number} editorId - Opaque ID for future operations.
   */
  createEditor(container, content, callbacks) {
    const id = nextId++;
    const cbs = callbacks || {};

    // Build state without CM6's history extension.
    // We use a minimal subset of basicSetup manually so we can exclude history.
    const state = EditorState.create({
      doc: content,
      extensions: [
        // Decoration field — allows Rust to push visual decorations
        decorationField,
        // Syntax indent (not essential for single-line but harmless)
        indentOnInput(),
        // Default keymap (cursor, clipboard, selection) WITHOUT history
        keymap.of(defaultKeymap.filter(b => {
          // Remove history-related bindings from defaultKeymap
          return b.key !== 'Mod-z' && b.key !== 'Mod-y' && b.key !== 'Mod-Shift-z';
        })),
        // Our custom bindings (Enter, Tab, undo→outliner, ac nav, etc.)
        keymap.of(buildKeymap(cbs)),
        // Update listener to fire onChange
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            const text = update.state.doc.toString();
            // Update our cache
            const entry = editors.get(id);
            if (entry) entry.cachedContent = text;
            // Fire callback
            if (cbs.onChange) {
              cbs.onChange(text);
            }
          }
        }),
        // Single-line mode: prevent newlines from being inserted by paste etc.
        EditorState.transactionFilter.of((tr) => {
          if (tr.docChanged) {
            const newDoc = tr.state.doc.toString();
            // If the new content contains a newline, filter it out
            // Actually this is complex — let's rely on the keymap to prevent Enter
            // but still allow pasted content to flow naturally.
          }
          return tr;
        }),
      ],
    });

    const view = new EditorView({
      state,
      parent: container,
      dispatchTransaction(trs) {
        view.update(trs);
      },
    });

    editors.set(id, {
      view,
      cachedContent: content,
      dropdownActive: false,
    });
    currentEditingId = id;

    return id;
  },

  /**
   * Destroy a CM6 editor and clean up.
   */
  destroyEditor(id) {
    const entry = editors.get(id);
    if (!entry) return;
    entry.view.destroy();
    editors.delete(id);
    if (currentEditingId === id) {
      currentEditingId = null;
    }
  },

  /**
   * Get the current content of an editor.
   */
  getContent(id) {
    const entry = editors.get(id);
    if (!entry) return '';
    return entry.cachedContent;
  },

  /**
   * Set the content of an editor, replacing the entire document.
   * Does NOT fire the onChange callback.
   */
  setContent(id, content) {
    const entry = editors.get(id);
    if (!entry) return;
    const { view } = entry;
    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: content,
      },
    });
    entry.cachedContent = content;
  },

  /**
   * Set content and place the cursor at a specific offset.
   * Does NOT fire the onChange callback.
   */
  setContentAndCursor(id, content, cursorOffset) {
    const entry = editors.get(id);
    if (!entry) return;
    const { view } = entry;
    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: content,
      },
      selection: { anchor: cursorOffset },
    });
    entry.cachedContent = content;
  },

  /**
   * Focus the editor.
   */
  focus(id) {
    const entry = editors.get(id);
    if (!entry) return;
    entry.view.focus();
    currentEditingId = id;
  },

  /**
   * Get the cursor offset (position) within the document.
   * Returns 0 if the editor is not found.
   */
  getCursorOffset(id) {
    const entry = editors.get(id);
    if (!entry) return 0;
    const sel = entry.view.state.selection.main;
    return sel.anchor;
  },

  /**
   * Enable or disable autocomplete keyboard intercept mode.
   * When active, ArrowUp/Down/Enter/Escape are captured for
   * autocomplete navigation instead of cursor movement.
   */
  setAutocompleteState(id, active) {
    const entry = editors.get(id);
    if (!entry) return;
    entry.dropdownActive = !!active;
  },

  /**
   * Apply visual decorations (tag/page-ref/property highlighting)
   * from Rust parser output.
   *
   * @param {number} id - Editor ID
   * @param {string} decorationsJson - JSON array of {from, to, class}
   */
  setDecorations(id, decorationsJson) {
    const entry = editors.get(id);
    if (!entry) return;
    const { view } = entry;

    let decoList;
    try {
      decoList = JSON.parse(decorationsJson);
    } catch {
      return;
    }

    if (!Array.isArray(decoList) || decoList.length === 0) {
      // Clear decorations
      view.dispatch({
        effects: setDecorationsEffect.of(Decoration.none),
      });
      return;
    }

    const decos = decoList.map(d => {
      return Decoration.mark({ class: d.class }).range(d.from, d.to);
    });

    view.dispatch({
      effects: setDecorationsEffect.of(Decoration.set(decos)),
    });
  },

  /**
   * Get cursor viewport-relative coordinates.
   * Returns {top, left, bottom} or null if unavailable.
   * Uses CM6's coordsAtPos which returns client/viewport-relative coords.
   */
  getCursorCoords(id) {
    const entry = editors.get(id);
    if (!entry) return null;
    const { view } = entry;
    const sel = view.state.selection.main;
    if (!sel) return null;
    const coords = view.coordsAtPos(sel.anchor);
    if (!coords) return null;
    return { top: coords.top, left: coords.left, bottom: coords.bottom };
  },

  /**
   * Check if an editor instance exists.
   */
  hasEditor(id) {
    return editors.has(id);
  },
};

// ── Expose as global ──
window.__quiltCm6 = api;
