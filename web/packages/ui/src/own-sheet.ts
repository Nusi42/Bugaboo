import { ownThemeSheets } from "@bugaboo/api-client";

/** The text of a pattern that matches `text` itself, every sign meaning only itself. */
function literally(text: string): string {
  return text.replaceAll(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
}

/**
 * The only addresses of the sheet of an own theme that the page links (ADR-0026 update M8): the server's sheet of a
 * saved theme, `/themes/{id}.css?v=<version>`, or of a draft, `/themes/{id}/draft.css?v=<n>`, on the page's own
 * origin. Nothing that climbs out of the folder, is written with escapes, or points elsewhere matches: the page waits
 * for a sheet it links, with the person's cookies. The text of the pattern, so the script of the first look, which
 * imports nothing, can use it too.
 */
export const ownSheetPattern = String.raw`^${literally(ownThemeSheets)}[\w-]+(?:/draft)?\.css(?:\?v=\d+)?$`;

/** Whether an address is one of the server's sheets of an own theme. */
export function isOwnSheet(address: string): boolean {
  return new RegExp(ownSheetPattern, "u").test(address);
}
