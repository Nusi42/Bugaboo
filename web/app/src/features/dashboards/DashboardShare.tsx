import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";
import { useState } from "react";

import { shareKind } from "../../shared/share.ts";
import { ShareWindow } from "../../shared/ShareWindow.tsx";
import { useChangeDashboardShare, useDashboardShareChoices, type Dashboard } from "./dashboards.ts";

/**
 * *Share* of a dashboard (ADR-0050, decision 2): the one window of sharing of saved searches, with the groups and the
 * projects the person may share a dashboard with (*Share dashboards*).
 */
export function DashboardShare({ dashboard }: { readonly dashboard: Dashboard }) {
  const [open, setOpen] = useState(false);
  const groups = useDashboardShareChoices(shareKind.group, open);
  const projects = useDashboardShareChoices(shareKind.project, open);
  const change = useChangeDashboardShare(dashboard.id);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("common.share")}
      </Button>
      <ShareWindow
        title={t("common.share-title", { name: dashboard.name })}
        note={t("dashboards.share-note")}
        isOpen={open}
        onClose={() => {
          setOpen(false);
        }}
        sharing={{
          sharedWith: dashboard.shared_with,
          choices: { group: groups, project: projects },
          change,
        }}
      />
    </>
  );
}
