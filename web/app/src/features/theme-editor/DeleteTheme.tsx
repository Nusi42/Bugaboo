import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { startsFromName } from "./theme-facts.ts";
import { useDeleteTheme, useThemeDetail } from "./themes.ts";

/**
 * What the question says before a theme goes: for a theme for everyone how many people chose it
 * and what they will see instead (*12 people use this theme. They will see Dusk instead.*), for a
 * personal one what its maker sees where they chose it.
 */
function useQuestionText(theme: Schema<"ThemeBody">, asking: boolean): string {
  const detail = useThemeDetail(theme.id, asking && theme.kind === "for-everyone");
  const instead = startsFromName(theme.extends);
  if (theme.kind === "personal") {
    return t("theme-editor.delete-personal", { theme: instead });
  }
  const people = detail.data?.used_by;
  return people === undefined || people === null
    ? t("theme-editor.delete-everyone", { theme: instead })
    : t("theme-editor.delete-used", { count: people, theme: instead });
}

/** *Delete* of an own theme, after a question; a choice of it counts as the built-in theme it extended. */
export function DeleteTheme({ theme }: { readonly theme: Schema<"ThemeBody"> }) {
  const [asking, setAsking] = useState(false);
  const remove = useDeleteTheme();
  const text = useQuestionText(theme, asking);
  return (
    <>
      <Button
        look={buttonLook.quietDanger}
        aria-label={t("theme-editor.delete-named", { name: theme.name })}
        onPress={() => {
          setAsking(true);
        }}
      >
        {t("theme-editor.delete")}
      </Button>
      <AskDialog
        isOpen={asking}
        onOpenChange={setAsking}
        title={t("theme-editor.delete-title", { name: theme.name })}
        text={text}
        confirmLabel={t("theme-editor.delete")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          remove.mutate({
            params: { path: { theme: theme.id }, query: { version: theme.version } },
          });
        }}
      />
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
}
