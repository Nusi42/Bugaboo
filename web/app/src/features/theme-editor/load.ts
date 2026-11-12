/** Loads the code of the theme editor, a chunk of its own (PERF-04): only when it is first opened. */
export function loadThemeEditor() {
  return import("./ThemeEditorPage.tsx");
}
