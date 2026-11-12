import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { EditLockNotices, editedThing, useEditLock } from "../../shared/editing/index.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { ThemeEditor } from "./ThemeEditor.tsx";
import { themeTarget, useThemeDetail } from "./themes.ts";

/** Where the editor goes back to: the list the theme is in. */
function backOf(kind: Schema<"ThemeKind">) {
  return kind === "for-everyone"
    ? { href: paths.themes, label: t("theme-editor.back-everyone") }
    : { href: paths.appearance, label: t("theme-editor.back-personal") };
}

/** The editor of one theme under its edit lock (ADR-0019): somebody else's lock is named above it. */
function LockedThemeEditor({ detail }: { readonly detail: Schema<"ThemeDetailBody"> }) {
  const lock = useEditLock(themeTarget(detail.theme.id));
  return (
    <Card heading={detail.theme.name} level={1} back={backOf(detail.theme.kind)}>
      <EditLockNotices lock={lock} thing={editedThing.theme} lost={t("theme-editor.lost")} />
      {/* A save makes a new version: the editor starts again from what was saved. */}
      <ThemeEditor key={detail.theme.version} detail={detail} lock={lock} />
    </Card>
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
  return (
    <QueryGate
      error={detail.error}
      data={detail.data}
      notFound={<p>{t("theme-editor.not-found")}</p>}
    >
      {(found: Schema<"ThemeDetailBody">) => <LockedThemeEditor detail={found} />}
    </QueryGate>
  );
}
