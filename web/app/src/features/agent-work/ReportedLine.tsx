import { t } from "@bugaboo/messages";
import { Facts, Length, partOf, part } from "@bugaboo/ui";

import { minutesText } from "../../shared/own-value-text.ts";
import { amountText, type Attempt } from "./agent-work.ts";

/** The time and the cost the agent reported, in one line: the time in the code font, then the cost. */
export function ReportedLine({ attempt }: { readonly attempt: Attempt }) {
  if (attempt.reported_minutes <= 0 && attempt.reported_cost <= 0) {
    return null;
  }
  return (
    <p {...partOf(part.muted)}>
      <Facts
        items={[
          { key: "time", node: <Length text={minutesText(attempt.reported_minutes)} /> },
          {
            key: "cost",
            node: t("agent-work.attempt-reported", { amount: amountText(attempt.reported_cost) }),
          },
        ]}
      />
    </p>
  );
}
