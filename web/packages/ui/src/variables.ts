import type { TokenName } from "./tokens.ts";

/**
 * The CSS variable of a token of the theme file. It is the only name the code may give to
 * `style.setProperty` (THEME-ONLY): a variable that the theme file declares, so a size the person
 * drags reaches the page without a style written in code.
 */
export function variableOf(name: TokenName): string {
  return `--${name}`;
}
