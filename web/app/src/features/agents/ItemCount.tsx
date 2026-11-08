import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

/** The number beside the heading of a list: *(3)*. Nothing for an empty list. */
export function ItemCount({ count }: { readonly count: number }) {
  return count === 0 ? null : <span {...partOf(part.muted)}> {t("agents.count", { count })}</span>;
}
