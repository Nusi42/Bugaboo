import { t } from "@bugaboo/messages";
import { Window } from "@bugaboo/ui";
import { useId } from "react";

import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { closeOverlay, useOverlay } from "../shell/index.ts";
import { NewIssueActions } from "./NewIssueActions.tsx";
import { NewIssueForm } from "./NewIssueForm.tsx";
import { useNewIssue } from "./use-new-issue.ts";
import { useOnMade } from "./use-on-made.ts";

/**
 * The new-issue window (ISS-11, UX §4.5, §6.3): a window on computers that can be minimized,
 * maximized and resized, a full screen on phones. It opens from the sidebar, the bar of a phone,
 * the key `c` and the empty list, over any page. The values live here and not in the form,
 * because a minimized window draws its form again when it opens: what was typed is still there,
 * and the draft is kept in the browser and on the server as it is typed.
 */
export function NewIssueDialog() {
  const overlay = useOverlay("new-issue");
  const form = useNewIssue();
  const windowProps = useWindowProps(windowForms.newIssue);
  const formId = useId();
  const onMade = useOnMade();
  return (
    <Window
      {...overlay}
      {...windowProps}
      title={t("issues.new-title")}
      footer={
        <NewIssueActions form={form} formId={formId} onMade={onMade} onClose={closeOverlay} />
      }
    >
      <NewIssueForm form={form} formId={formId} onMade={onMade} />
    </Window>
  );
}
