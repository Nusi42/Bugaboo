import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf, useLayout } from "@bugaboo/ui";
import { useEffect } from "react";

import { LegendButton } from "./LegendButton.tsx";
import { IssueLegend } from "./IssueLegend.tsx";
import { openNewIssue, rememberProject } from "./new-issue-state.ts";

export interface ProjectToolbarProps {
  readonly shortName: string;
  /** An issue is open beside the list, which leaves the toolbar little room. */
  readonly beside: boolean;
}

/**
 * The rows above a project's list: *New issue* with the key to the colors of the rows at the end (a *Legend* button where it has no room),
 * and what is ticked with the bars that act on it. The project is remembered for the next new issue.
 */
export function ProjectToolbar({ shortName, beside }: ProjectToolbarProps) {
  const room = useLayout() === "computer" && !beside;
  useEffect(() => {
    rememberProject(shortName);
  }, [shortName]);
  return (
    <div {...partOf(part.listToolbar)}>
      <Button
        look={buttonLook.primary}
        onPress={() => {
          openNewIssue(shortName);
        }}
      >
        {t("issues.new")}
      </Button>
      {room ? <IssueLegend /> : <LegendButton />}
    </div>
  );
}
