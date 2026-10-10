import { t } from "@bugaboo/messages";
import { Overlay, overlayEdge, useLayout } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress, issuePanelAddress } from "../../shared/paths.ts";
import { closeOverlay, useOverlay } from "../shell/index.ts";
import { NewIssueForm } from "./NewIssueForm.tsx";
import { useOpenIssuesIn } from "./open-in.ts";

/**
 * The new-issue dialog (ISS-11, UX §4.5): a dialog on computers, a full screen on phones. It opens
 * from the sidebar, the bar of a phone, the key `c` and the empty list, over any page, and the
 * issue that was made opens the way the person chose to open issues.
 */
export function NewIssueDialog() {
  const overlay = useOverlay("new-issue");
  const phone = useLayout() === "phone";
  const openIn = useOpenIssuesIn();
  const navigate = useNavigate();
  return (
    <Overlay
      {...overlay}
      label={t("issues.new-title")}
      from={phone ? overlayEdge.full : overlayEdge.center}
    >
      {overlay.isOpen ? (
        <NewIssueForm
          onClose={closeOverlay}
          onMade={(key, project) => {
            closeOverlay();
            void navigate({
              href: openIn === "focus" ? issueAddress(key) : issuePanelAddress(project, key),
            });
          }}
        />
      ) : null}
    </Overlay>
  );
}
