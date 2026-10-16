import type { RunTimeVariable } from "./tokens.ts";

/**
 * The CSS variable of a token that the theme file marks `run-time="true"`: the size of a window
 * while the person drags it. It is the only name the code may give to `style.setProperty`, and
 * only here (THEME-ONLY): a color or a spacing cannot be set from code.
 */
export function variableOf(name: RunTimeVariable): string {
  return `--${name}`;
}
