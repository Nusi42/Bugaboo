import { t } from "@bugaboo/messages";

import { bulkKind, type BulkKind } from "./bulk.ts";

const names: Readonly<Record<BulkKind, () => string>> = {
  [bulkKind.change]: () => t("bulk.action-change"),
  [bulkKind.move]: () => t("bulk.action-move"),
  [bulkKind.label]: () => t("bulk.action-label"),
  [bulkKind.relation]: () => t("bulk.action-relation"),
  [bulkKind.delete]: () => t("bulk.action-delete"),
};

/** The name of one of the five things, as the buttons and the title of the dialog call it. */
export function actionName(kind: BulkKind): string {
  return names[kind]();
}

/** What the button that applies the change says, with how many issues it is for: *Change 12 issues*. */
export function applyName(kind: BulkKind, count: number): string {
  return kind === bulkKind.delete ? t("bulk.apply-delete", { count }) : t("bulk.apply", { count });
}
