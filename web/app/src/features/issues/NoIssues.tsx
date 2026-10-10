import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { openNewIssue } from "./new-issue-state.ts";

/** An empty list says so in one sentence and offers the one thing to do (UX §6.1). */
export function NoIssues({ project }: { readonly project: string }) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("issues.nothing")}</p>
      <Button
        look={buttonLook.primary}
        onPress={() => {
          openNewIssue(project);
        }}
      >
        {t("issues.create-first")}
      </Button>
    </div>
  );
}
