import { t } from "@bugaboo/messages";
import { focusFirstField, useLayout, Window, withId } from "@bugaboo/ui";
import { useEffect, useId } from "react";

import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { closeOverlay, useOverlay } from "../shell/index.ts";
import { NewIssueActions, NewIssueMoreActions } from "./NewIssueActions.tsx";
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
  const whyId = useId();
  const onMade = useOnMade();
  const phone = useLayout() === "phone";
  // *Create the issue and start another* made one: the focus goes back to the title, for the next
  // (the button that was pressed waits while the issue is made, and would drop the focus). On a phone
  // the title may be out of view at the end of the form: the form scrolls it into view, in its own
  // box, so the focus is never out of sight (WCAG 2.4.11).
  useEffect(() => {
    if (form.lastKey !== null) {
      focusFirstField(formId);
      const field = document.activeElement;
      if (field instanceof HTMLElement && field.closest(withId(formId)) !== null) {
        field.scrollIntoView({ block: "nearest" });
      }
    }
  }, [form.lastKey, formId]);
  return (
    <Window
      {...overlay}
      {...windowProps}
      ready={form.ready}
      title={t("issues.new-title")}
      footer={
        <NewIssueActions
          form={form}
          formId={formId}
          onMade={onMade}
          onClose={closeOverlay}
          whyId={whyId}
        />
      }
    >
      <NewIssueForm form={form} formId={formId} onMade={onMade} />
      {phone ? (
        <NewIssueMoreActions form={form} onMade={onMade} onClose={closeOverlay} whyId={whyId} />
      ) : null}
    </Window>
  );
}
