import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { percentText } from "../../shared/number.ts";
import { detailsText } from "../../shared/list-text.ts";
import type { WorkTimes } from "./work-times.ts";

type Shares = NonNullable<WorkTimes["shares"]>;

/** One share as the line says it: "Mobile app 60 %"; a project the person may not see stays unnamed. */
function shareText(item: Shares["items"][number]): string {
  return t("work-times.share", {
    project: item.project.name ?? t("work-times.share-hidden"),
    percent: percentText(item.percent),
  });
}

/**
 * *Shares in projects* on one's own page (PLAN-15): read only, set by the project leads, with what
 * they add up to when that is more than 100 %. The server says which shares count and what they add
 * up to.
 */
export function SharesLine({ shares }: { readonly shares: Shares }) {
  const counted = shares.items.filter((item) => item.applies);
  return (
    <Card heading={t("work-times.shares")} intro={t("work-times.shares-intro")}>
      <p {...partOf(part.text)}>
        {counted.length === 0
          ? t("work-times.shares-none")
          : detailsText(counted.map((item) => shareText(item)))}
      </p>
      {shares.over_100 ? (
        <p {...partOf(part.muted, { variants: ["warning"] })}>
          {t("work-times.shares-over", { sum: percentText(shares.sum) })}
        </p>
      ) : null}
    </Card>
  );
}
