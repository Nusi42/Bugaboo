import { t } from "@bugaboo/messages";

import { useChangeDashboard, type Dashboard } from "./dashboards.ts";
import { NameDialog } from "./NameDialog.tsx";

/** *Rename* of a dashboard: its new name, saved on the version the person looked at. */
export function RenameDialog({
  dashboard,
  isOpen,
  onOpenChange,
}: {
  readonly dashboard: Dashboard;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const change = useChangeDashboard(dashboard);
  return (
    <NameDialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={t("dashboards.rename")}
      text={t("dashboards.rename-text")}
      confirm={t("dashboards.rename")}
      start={dashboard.name}
      onConfirm={(name) => {
        change.mutate({ name, version: dashboard.version });
      }}
    />
  );
}
