import {
  dataOf,
  readDraft,
  saveDraft,
  themeEditPart,
  type components,
  type EditTarget,
  type paths,
  type Schema,
} from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { refreshRoutes } from "../../shared/query-keys.ts";

/** An own theme as a person who may use it sees it. */
export type OwnTheme = Schema<"ThemeBody">;
/** What an own theme changes of the built-in theme it extends. */
export type ThemeValues = Schema<"ThemeValuesBody">;
/** One layer of a shadow. */
export type ShadowLayer = Schema<"ShadowLayerBody">;
/** A pair of colors that is hard to read, as the server names it. */
export type HardPair = Schema<"HardPairBody">;

/**
 * An own theme was made, saved, copied or taken away: the lists of themes, the theme itself and the
 * settings (whose answer holds the address of the sheet of a chosen theme) are read again.
 */
function refreshThemes(): void {
  refreshRoutes(["/api/v1/themes", "/api/v1/me/settings"]);
}

/** The type of a theme file: JSON. */
const JSON_TYPE = "application/json";

/** What the edit lock of a theme and the draft of its editor are about: the theme as a whole. */
export function themeTarget(theme: string): EditTarget {
  return { record: { kind: "theme", id: theme }, part: themeEditPart };
}

/** The own themes the person may use: the themes for everyone and their own personal ones. */
export function useOwnThemes() {
  return api.query.useQuery("get", "/api/v1/themes");
}

/**
 * One own theme with its values, the pairs that are hard to read and, for who looks after it, who
 * chose it; read only while it is `wanted`.
 */
export function useThemeDetail(theme: string, wanted = true) {
  return api.query.useQuery(
    "get",
    "/api/v1/themes/{theme}",
    { params: { path: { theme } } },
    { enabled: wanted },
  );
}

/** Makes a new own theme; it opens in the editor. */
export function useCreateTheme() {
  return api.query.useMutation("post", "/api/v1/themes", { onSuccess: refreshThemes });
}

/** Saves the name and the values of a theme under the lock the person holds. */
export function useSaveTheme() {
  return api.query.useMutation("put", "/api/v1/themes/{theme}", { onSuccess: refreshThemes });
}

/** *Save a copy*: keeps a theme as a new one of the same kind, saved at once. */
export function useCopyTheme() {
  return api.query.useMutation("post", "/api/v1/themes/{theme}/copy", {
    onSuccess: refreshThemes,
  });
}

/** Takes a theme away; a choice of it counts as the built-in theme it extended. */
export function useDeleteTheme() {
  return api.query.useMutation("delete", "/api/v1/themes/{theme}", { onSuccess: refreshThemes });
}

/**
 * *Upload*: makes a theme from a file. The text of the file goes to the server as it is, so the
 * server judges all of it: its size, its shape and every value.
 */
export function useUploadTheme() {
  return api.query.useMutation("post", "/api/v1/themes/upload", { onSuccess: refreshThemes });
}

/**
 * What an upload sends: the text of the file exactly as it is (`bodySerializer`), so the server
 * sees the whole file, its size and every part of it, and judges it. The client's type asks for a
 * body; `standIn` is that body, and it never leaves the browser.
 */
export function asFileText(text: string) {
  const standIn: components["schemas"]["ThemeFileBody"] = { name: "", extends: "", values: {} };
  return {
    body: standIn,
    bodySerializer: () => text,
    headers: { "Content-Type": JSON_TYPE },
  };
}

/** The values as the body of a request takes them: a fresh copy the client may hand on. */
export function valuesBody(values: ThemeValues): components["schemas"]["ThemeValuesBody"] {
  return {
    ...(values.colors === undefined
      ? {}
      : {
          colors: Object.fromEntries(
            Object.keys(values.colors).flatMap((token) => {
              const color = values.colors?.[token];
              return color === undefined ? [] : [[token, { ...color }]];
            }),
          ),
        }),
    ...(values.radii === undefined ? {} : { radii: { ...values.radii } }),
    ...(values.fonts === undefined ? {} : { fonts: { ...values.fonts } }),
    ...(values.shadow === undefined
      ? {}
      : { shadow: values.shadow === null ? null : values.shadow.map((layer) => ({ ...layer })) }),
  };
}

/** The theme as a file, in the shape an upload takes. */
export async function themeFile(theme: string): Promise<Schema<"ThemeFileBody">> {
  return dataOf(
    await api.client.GET("/api/v1/themes/{theme}/file", { params: { path: { theme } } }),
  );
}

/** The person's draft of a theme: what the editor kept of the last time it was open. */
export async function keptValues(theme: string): Promise<string | null> {
  return (await readDraft(api.client, themeTarget(theme)))?.text ?? null;
}

/** Keeps the values on screen as the person's draft, which the sheet of the preview is made from. */
export async function keepValues(
  theme: string,
  values: ThemeValues,
  version: number,
): Promise<void> {
  await saveDraft(api.client, themeTarget(theme), {
    text: JSON.stringify(values),
    base_version: version,
  });
}

/** The address of the sheet of the person's draft of a theme, new for each `round`. */
export function draftSheetOf(theme: string, round: number): string {
  const route: keyof paths = "/themes/{theme}/draft.css";
  return `${route.replace("{theme}", encodeURIComponent(theme))}?v=${String(round)}`;
}
