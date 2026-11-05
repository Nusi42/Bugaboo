import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";

/** One person's old work days, with *Delete them* for that person when they are named. */
export function OldPersonLine({
  line,
  deleting,
  onDelete,
}: {
  readonly line: Schema<"OldOfPersonBody">;
  readonly deleting: boolean;
  readonly onDelete: (person: string) => void;
}) {
  const name = personName(line.person);
  const id = line.id ?? undefined;
  return (
    <li {...partOf(part.row)}>
      <span {...partOf(part.strong)}>{name}</span>
      <span {...partOf(part.muted)}>{t("old-work-days.person-count", { count: line.count })}</span>
      {id === undefined ? null : (
        <ConfirmDialog
          triggerLabel={t("old-work-days.delete")}
          triggerName={t("old-work-days.delete-person-name", { name })}
          title={t("old-work-days.delete-person-title", { name })}
          text={t("old-work-days.delete-person-text", { count: line.count, name })}
          confirmLabel={t("old-work-days.delete")}
          cancelLabel={t("common.cancel")}
          confirmLook={buttonLook.danger}
          confirmDisabled={deleting}
          onConfirm={() => {
            onDelete(id);
          }}
        />
      )}
    </li>
  );
}
