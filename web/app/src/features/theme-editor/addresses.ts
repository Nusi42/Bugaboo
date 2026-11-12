import type { Schema } from "@bugaboo/api-client";

import { filled, paths, routeParam } from "../../shared/paths.ts";

/**
 * The address of the theme editor of an own theme: a theme for everyone in *Organization settings ›
 * Look › Themes* (`/settings/themes/<ID>`), a personal one in *My settings › Appearance*
 * (`/me/appearance/themes/<ID>`).
 */
export function themeEditorAddress(kind: Schema<"ThemeKind">, id: string): string {
  return filled(kind === "for-everyone" ? paths.theme : paths.myTheme, routeParam.theme, id);
}
