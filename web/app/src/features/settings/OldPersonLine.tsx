import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { buttonLook, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";
import { recordsText, type DataKind } from "./keep-rules.ts";

/** One person's old records of a kind, with *Delete them* for that person when they are named. */
export function OldPersonLine({
  kind,
  line,
  deleting,
  onDelete,
}: {
  readonly kind: DataKind;
  readonly line: Schema<"OldOfPersonBody">;
  readonly deleting: boolean;
  readonly onDelete: (person: string) => void;
}) {
  const name = personName(line.person);
  const id = line.id ?? undefined;
  const records = recordsText(kind, line.count);
  return (
    <li {...partOf(part.row)}>
      <span {...partOf(part.strong)}>{name}</span>
      <span {...partOf(part.muted)}>{records}</span>
      {id === undefined ? null : (
        <ConfirmDialog
          triggerLabel={t("old-records.delete")}
          triggerName={t("old-records.delete-person-name", { name })}
          title={t("old-records.delete-person-title", { name })}
          text={t("old-records.delete-person-text", { records, count: line.count, name })}
          confirmLabel={t("old-records.delete")}
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
