import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf } from "@bugaboo/ui";
import { useEffect } from "react";

import { IssueLegend } from "./IssueLegend.tsx";
import { openNewIssue, rememberProject } from "./new-issue-state.ts";
import { Selected } from "./Selected.tsx";
import type { IssueSelection } from "./use-selection.ts";

export interface ProjectToolbarProps {
  readonly shortName: string;
  readonly selection: IssueSelection;
}

/**
 * The rows above a project's list: *New issue* with the key to the colors of the rows at the end,
 * and what is ticked with the bars that act on it. The project is remembered for the next new issue.
 */
export function ProjectToolbar({ shortName, selection }: ProjectToolbarProps) {
  useEffect(() => {
    rememberProject(shortName);
  }, [shortName]);
  return (
    <>
      <div {...partOf(part.listToolbar)}>
        <Button
          look={buttonLook.primary}
          onPress={() => {
            openNewIssue(shortName);
          }}
        >
          {t("issues.new")}
        </Button>
        <IssueLegend />
      </div>
      <Selected selection={selection} project={shortName} />
    </>
  );
}
