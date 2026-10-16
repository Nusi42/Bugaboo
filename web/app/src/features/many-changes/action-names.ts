import { t } from "@bugaboo/messages";

import { bulkKind, type BulkKind } from "./bulk.ts";

const names: Readonly<Record<BulkKind, () => string>> = {
  [bulkKind.change]: () => t("many-changes.action-change"),
  [bulkKind.move]: () => t("many-changes.action-move"),
  [bulkKind.label]: () => t("many-changes.action-label"),
  [bulkKind.relation]: () => t("many-changes.action-relation"),
  [bulkKind.delete]: () => t("many-changes.action-delete"),
};

/** The name of one of the five things, as the buttons and the title of the dialog call it. */
export function actionName(kind: BulkKind): string {
  return names[kind]();
}

/** What the button that applies the change says, with how many issues it is for: *Change 12 issues*. */
export function applyName(kind: BulkKind, count: number): string {
  return kind === bulkKind.delete
    ? t("many-changes.apply-delete", { count })
    : t("many-changes.apply", { count });
}
