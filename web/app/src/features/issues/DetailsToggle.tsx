import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";

import { detailsSummary } from "./DetailsSummary.tsx";
import type { IssueView } from "./slots.ts";

export interface DetailsToggleProps {
  readonly issue: IssueView;
  readonly open: boolean;
  readonly onToggle: () => void;
}

/**
 * Where the details do not have a column of their own (UX §3.3): *All details* opens them under
 * the status and, while they are closed, the button still says who the issue is assigned to and
 * its priority (*Ana · High*).
 */
export function DetailsToggle({ issue, open, onToggle }: DetailsToggleProps) {
  return (
    <div {...partOf(part.issueDetailsToggle)}>
      <Button onPress={onToggle}>
        {open ? t("issues.hide-details") : t("issues.all-details")}
      </Button>
      {open ? null : <span {...partOf(part.muted)}>{detailsSummary(issue)}</span>}
    </div>
  );
}
