import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { DeleteTheme } from "./DeleteTheme.tsx";
import { CopyThemeButton, DownloadThemeButton } from "./ThemeActions.tsx";
import { themeFacts } from "./theme-facts.ts";
import { themeEditorAddress } from "./addresses.ts";

/** One theme of a list: its name and facts, and *Change*, *Save a copy*, *Download* and *Delete*. */
export function ThemeRow({ theme }: { readonly theme: Schema<"ThemeBody"> }) {
  return (
    <li {...partOf(part.themeListItem)}>
      <div {...partOf(part.themeListText)}>
        <span>{theme.name}</span>
        <span {...partOf(part.themeListFacts)}>{themeFacts(theme)}</span>
      </div>
      <div {...partOf(part.row)}>
        {theme.may_change ? (
          <Link href={themeEditorAddress(theme.kind, theme.id)}>
            {t("theme-editor.change-named", { name: theme.name })}
          </Link>
        ) : null}
        {theme.saved ? <CopyThemeButton theme={theme} /> : null}
        {theme.saved ? <DownloadThemeButton theme={theme} /> : null}
        {theme.may_change ? <DeleteTheme theme={theme} /> : null}
      </div>
    </li>
  );
}
