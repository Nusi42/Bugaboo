import { t } from "@bugaboo/messages";

import { manyChangeKind, type ManyChangeKind } from "./many-changes.ts";

const names: Readonly<Record<ManyChangeKind, () => string>> = {
  [manyChangeKind.change]: () => t("many-changes.action-change"),
  [manyChangeKind.move]: () => t("many-changes.action-move"),
  [manyChangeKind.label]: () => t("many-changes.action-label"),
  [manyChangeKind.relation]: () => t("many-changes.action-relation"),
  [manyChangeKind.delete]: () => t("many-changes.action-delete"),
};

/** The name of one of the five things, as the buttons and the title of the dialog call it. */
export function actionName(kind: ManyChangeKind): string {
  return names[kind]();
}

/** What the button that applies the change says, with how many issues it is for: *Change 12 issues*. */
export function applyName(kind: ManyChangeKind, count: number): string {
  return kind === manyChangeKind.delete
    ? t("many-changes.apply-delete", { count })
    : t("many-changes.apply", { count });
}

/** What the button of the bar says, with how many issues are ticked: *Change 2 issues*, *Delete 2 issues*; the others keep their name. */
export function barName(kind: ManyChangeKind, count: number): string {
  return kind === manyChangeKind.change || kind === manyChangeKind.delete
    ? applyName(kind, count)
    : actionName(kind);
}
