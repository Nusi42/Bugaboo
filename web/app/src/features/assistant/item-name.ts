import { t } from "@bugaboo/messages";

import type { LookingAtItem } from "./looking-at.ts";

/** The name of an item in words, for the button that leaves it out. */
export function itemName(item: LookingAtItem): string {
  return item.kind === "search" ? t("assistant.looking-list") : item.value;
}
