import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { EditLockNotices, editedThing, useEditLock } from "../../shared/editing/index.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate, useItemTrail } from "../shell/index.ts";
import { ThemeEditor } from "./ThemeEditor.tsx";
import { themeTarget, useThemeDetail } from "./themes.ts";

/**
 * The editor of one theme under its edit lock (ADR-0019): somebody else's lock is named under the
 * title. The top bar says where it is (`Organization settings › Themes › Harbor`, `My settings ›
 * Appearance › Harbor`), and on a phone it leads back to the list.
 */
function LockedThemeEditor({ detail }: { readonly detail: Schema<"ThemeDetailBody"> }) {
  const { theme } = detail;
  const lock = useEditLock(themeTarget(theme.id));
  useItemTrail(theme.kind === "for-everyone" ? paths.themes : paths.appearance, theme.name);
  return (
    // A save makes a new version: the editor starts again from what was saved.
    <ThemeEditor
      key={theme.version}
      detail={detail}
      lock={lock}
      notices={
        <EditLockNotices lock={lock} thing={editedThing.theme} lost={t("theme-editor.lost")} />
      }
    />
  );
}

/**
 * The page of the theme editor (UX §5.6, THM-03, THM-04): a theme for everyone under *Look ›
 * Themes*, a personal one under *My settings › Appearance*. A theme the person may not change is
 * answered as one that is not there.
 */
export function ThemeEditorPage({ theme }: { readonly theme: string }) {
  usePageTitle(t("theme-editor.title"));
  const detail = useThemeDetail(theme);
  const notFound = <p>{t("theme-editor.not-found")}</p>;
  return (
    <QueryGate error={detail.error} data={detail.data} notFound={notFound}>
      {(found: Schema<"ThemeDetailBody">) =>
        found.theme.may_change ? <LockedThemeEditor detail={found} /> : notFound
      }
    </QueryGate>
  );
}
