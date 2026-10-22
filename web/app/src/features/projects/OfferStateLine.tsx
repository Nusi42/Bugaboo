import { t } from "@bugaboo/messages";
import { Icon, iconName, iconSize, partOf, part } from "@bugaboo/ui";

import { offerStateName } from "./offer-state.ts";

/** *Website · Offered* with a check or a cross before it, so the state is never the words alone (R5). */
export function OfferStateLine({
  name,
  offered,
}: {
  readonly name: string;
  readonly offered: boolean;
}) {
  return (
    <span {...partOf(part.row)}>
      <Icon name={offered ? iconName.check : iconName.close} size={iconSize.small} />
      <span>{t("projects.offer-line", { name, state: offerStateName(offered) })}</span>
    </span>
  );
}
