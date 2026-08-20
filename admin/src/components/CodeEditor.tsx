import { EditorState } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import {
  HighlightStyle,
  StreamLanguage,
  bracketMatching,
  indentOnInput,
  syntaxHighlighting,
} from "@codemirror/language";
import { tags } from "@lezer/highlight";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { yaml } from "@codemirror/lang-yaml";
import { properties } from "@codemirror/legacy-modes/mode/properties";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { useEffect, useRef } from "react";

const darkTheme = EditorView.theme(
  {
    "&": {
      height: "100%",
      color: "#f0f6fc",
      backgroundColor: "#0d1117",
      fontSize: "13px",
    },
    ".cm-content": {
      caretColor: "#f0f6fc",
      fontFamily: "var(--font-mono)",
      lineHeight: "1.65",
      padding: "14px 0",
    },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#f0f6fc" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "#264f78",
    },
    ".cm-activeLine": { backgroundColor: "rgba(88, 166, 255, 0.08)" },
    ".cm-activeLineGutter": { backgroundColor: "rgba(88, 166, 255, 0.08)" },
    ".cm-gutters": {
      color: "#8b949e",
      backgroundColor: "#161b22",
      borderRight: "1px solid #30363d",
      fontFamily: "var(--font-mono)",
    },
    ".cm-lineNumbers .cm-gutterElement": { padding: "0 11px 0 8px", minWidth: "46px" },
    ".cm-matchingBracket": { backgroundColor: "rgba(88, 166, 255, 0.2)", outline: "1px solid #58a6ff" },
    ".cm-searchMatch": { backgroundColor: "rgba(210, 153, 34, 0.35)", outline: "1px solid #d29922" },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "rgba(63, 185, 80, 0.4)" },
    ".cm-tooltip": {
      color: "#f0f6fc",
      backgroundColor: "#161b22",
      border: "1px solid #30363d",
    },
    ".cm-panels": { color: "#f0f6fc", backgroundColor: "#161b22" },
    ".cm-panels.cm-panels-top": { borderBottom: "1px solid #30363d" },
    ".cm-panels.cm-panels-bottom": { borderTop: "1px solid #30363d" },
  },
  { dark: true },
);

const darkHighlightStyle = HighlightStyle.define([
  { tag: tags.comment, color: "#8b949e", fontStyle: "italic" },
  { tag: [tags.string, tags.special(tags.string)], color: "#a5d6ff" },
  { tag: [tags.number, tags.bool, tags.null], color: "#79c0ff" },
  { tag: [tags.keyword, tags.operatorKeyword], color: "#ff7b72" },
  { tag: [tags.propertyName, tags.attributeName], color: "#79c0ff" },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], color: "#d2a8ff" },
  { tag: tags.variableName, color: "#ffa657" },
  { tag: [tags.tagName, tags.typeName, tags.className], color: "#7ee787" },
  { tag: [tags.definition(tags.variableName), tags.definition(tags.propertyName)], color: "#ffa657" },
  { tag: tags.heading, color: "#7ee787", fontWeight: "700" },
  { tag: [tags.link, tags.url], color: "#58a6ff", textDecoration: "underline" },
  { tag: tags.meta, color: "#8b949e" },
  { tag: tags.invalid, color: "#ff7b72", textDecoration: "underline wavy" },
]);

function languageExtensions(language: string, format: string) {
  const normalized = language.toLowerCase();
  if (normalized === "yaml") return yaml();
  if (normalized === "json") return json();
  if (normalized === "markdown" || normalized === "md") return markdown();
  if (normalized === "shell" || normalized === "bash" || normalized === "sh") {
    return StreamLanguage.define(shell);
  }
  if (normalized === "properties" || normalized === "ini" || format === "dotenv") {
    return StreamLanguage.define(properties);
  }
  return [];
}

export interface CodeEditorProps {
  value: string;
  onChange?: (value: string) => void;
  language?: string;
  format?: string;
  readOnly?: boolean;
  ariaLabel?: string;
  height?: string;
}

export function CodeEditor({
  value,
  onChange,
  language = "text",
  format = "",
  readOnly = false,
  ariaLabel = "配置文件编辑器",
  height = "min(56vh, 620px)",
}: CodeEditorProps) {
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const applyingExternal = useRef(false);
  const initialValue = useRef(value);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (!host.current) return undefined;
    const state = EditorState.create({
      doc: initialValue.current,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        drawSelection(),
        history(),
        bracketMatching(),
        indentOnInput(),
        highlightSelectionMatches(),
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        keymap.of([indentWithTab]),
        languageExtensions(language, format),
        syntaxHighlighting(darkHighlightStyle),
        darkTheme,
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (!update.docChanged || applyingExternal.current) return;
          onChangeRef.current?.(update.state.doc.toString());
        }),
        EditorView.editable.of(!readOnly),
      ],
    });
    const editor = new EditorView({ state, parent: host.current });
    view.current = editor;
    return () => {
      editor.destroy();
      view.current = null;
    };
    // 语言和只读属性变化时重建编辑器；正文变化走下方同步逻辑。
  }, [language, format, readOnly]);

  useEffect(() => {
    const editor = view.current;
    if (!editor) return;
    if (editor.state.doc.toString() === value) return;
    applyingExternal.current = true;
    editor.dispatch({
      changes: { from: 0, to: editor.state.doc.length, insert: value },
    });
    applyingExternal.current = false;
  }, [value]);

  return (
    <div
      ref={host}
      className="code-editor"
      style={{ height }}
      role="textbox"
      aria-label={ariaLabel}
      aria-readonly={readOnly}
    />
  );
}
