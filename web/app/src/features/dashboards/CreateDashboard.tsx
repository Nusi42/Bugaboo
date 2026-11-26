import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";

import { dashboardAddress } from "../../shared/dashboard-address.ts";
import { useCreateDashboard, type Dashboard } from "./dashboards.ts";
import { NameDialog } from "./NameDialog.tsx";

/** A new dashboard has no name until the person types one. */
const noName = "";

/**
 * *Create a dashboard* (UX §4.10): its window asks for the name and opens the new dashboard, private and empty, in
 * *Change*. The sidebar and *Home* each draw their own control, which calls `open`; `onMade` runs before the dashboard
 * opens (the sheet of a phone closes).
 */
export function useCreateDashboardWindow(onMade?: () => void): {
  readonly open: () => void;
  readonly pending: boolean;
  readonly window: ReactNode;
} {
  const [isOpen, setOpen] = useState(false);
  const create = useCreateDashboard();
  const navigate = useNavigate();
  const window = (
    <NameDialog
      isOpen={isOpen}
      onOpenChange={setOpen}
      title={t("dashboards.create")}
      text={t("dashboards.create-text")}
      confirm={t("dashboards.create-confirm")}
      start={noName}
      onConfirm={(name) => {
        create.mutate(name, {
          onSuccess: (made: Dashboard) => {
            onMade?.();
            void navigate({ href: dashboardAddress(made.id) });
          },
        });
      }}
    />
  );
  return {
    open: () => {
      setOpen(true);
    },
    pending: create.isPending,
    window,
  };
}
