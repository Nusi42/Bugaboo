import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { DeleteTheme } from "./DeleteTheme.tsx";
import { ThemeActions } from "./ThemeActions.tsx";
import { themeFacts } from "./theme-facts.ts";
import { themeEditorAddress } from "./addresses.ts";

/** *Change*: opens the theme in the editor. */
function ChangeTheme({ theme }: { readonly theme: Schema<"ThemeBody"> }) {
  const navigate = useNavigate();
  return (
    <Button
      look={buttonLook.quiet}
      aria-label={t("theme-editor.change-named", { name: theme.name })}
      onPress={() => {
        void navigate({ to: themeEditorAddress(theme.kind, theme.id) });
      }}
    >
      {t("theme-editor.change")}
    </Button>
  );
}

/**
 * One theme of a list: its name with *For everyone* or *Personal*, its facts, and four buttons of
 * one width: *Change*, *Save a copy*, *Download* and *Delete* (two by two on a phone).
 */
export function ThemeRow({ theme }: { readonly theme: Schema<"ThemeBody"> }) {
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
        {theme.may_change ? <ChangeTheme theme={theme} /> : null}
        <ThemeActions theme={theme} />
        {theme.may_change ? <DeleteTheme theme={theme} /> : null}
      </div>
    </li>
  );
}
