import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { buttonLook, Link, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { DeleteTheme } from "./DeleteTheme.tsx";
import { ActionFailure, ThemeActions } from "./ThemeActions.tsx";
import { themeFacts } from "./theme-facts.ts";
import { themeEditorAddress } from "./addresses.ts";

/**
 * One theme of a list: its name with *For everyone* or *Personal*, its facts, and four outlined
 * actions of one width: *Change* (a link), *Save a copy*, *Download* and *Delete* (two by two on a
 * phone); a failure shows under them.
 */
export function ThemeRow({ theme }: { readonly theme: Schema<"ThemeBody"> }) {
  const [failure, setFailure] = useState<unknown>(null);
  return (
    <li {...partOf(part.themeListItem)}>
      <div {...partOf(part.themeListText)}>
        <span {...partOf(part.row)}>
          <span>{theme.name}</span>
          <span {...partOf(part.themeListTag)}>
            {t(
              theme.kind === "for-everyone" ? "theme-editor.for-everyone" : "theme-editor.personal",
            )}
          </span>
        </span>
        <span {...partOf(part.themeListFacts)}>{themeFacts(theme)}</span>
      </div>
      <div {...partOf(part.themeListActions)}>
        {theme.may_change ? (
          <Link
            href={themeEditorAddress(theme.kind, theme.id)}
            button
            label={t("theme-editor.change-named", { name: theme.name })}
          >
            {t("theme-editor.change")}
          </Link>
        ) : null}
        <ThemeActions theme={theme} look={buttonLook.plain} onFailed={setFailure} />
        {theme.may_change ? <DeleteTheme theme={theme} onFailed={setFailure} /> : null}
      </div>
      <ActionFailure failure={failure} />
    </li>
  );
}
