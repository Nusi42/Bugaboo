import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { AskDialog, Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import { startsFromName } from "./theme-facts.ts";
import { useDeleteTheme, useThemeDetail } from "./themes.ts";

/**
 * What the question says before a theme goes: for a theme for everyone how many people chose it
 * and what they will see instead (*12 people use this theme. They will see Dusk instead.*), and
 * when it is the organization's choice, that everyone who follows that choice sees Dusk too; for a
 * personal one what its maker sees where they chose it.
 */
function useQuestionText(theme: Schema<"ThemeBody">, asking: boolean): string {
  const detail = useThemeDetail(theme.id, asking && theme.kind === "for-everyone");
  const instead = startsFromName(theme.extends);
  if (theme.kind === "personal") {
    return t("theme-editor.delete-personal", { theme: instead });
  }
  const people = detail.data?.used_by;
  if (people === undefined || people === null) {
    return t("theme-editor.delete-everyone", { theme: instead });
  }
  return detail.data?.organization_choice === true
    ? t("theme-editor.delete-organization", { count: people, theme: instead })
    : t("theme-editor.delete-used", { count: people, theme: instead });
}

/**
 * *Delete* of an own theme, after a question; a choice of it counts as the built-in theme it
 * extended. A delete that does not work is told under the row, like a copy or a download.
 */
export function DeleteTheme({
  theme,
  onFailed,
}: {
  readonly theme: Schema<"ThemeBody">;
  readonly onFailed: (failure: unknown) => void;
}) {
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
          onFailed(null);
          remove.mutate(
            { params: { path: { theme: theme.id }, query: { version: theme.version } } },
            { onError: onFailed },
          );
        }}
      />
    </>
  );
}
