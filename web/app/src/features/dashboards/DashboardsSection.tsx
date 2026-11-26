import { t } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { itemsOf } from "../../shared/paged.ts";
import { dashboardAddress } from "../../shared/paths.ts";
import { MineAndShared, NavButton, type SidebarSectionView } from "../shell/index.ts";
import {
  dashboardSet,
  dashboardsHeading,
  useCreateDashboard,
  useDashboardList,
  type Dashboard,
} from "./dashboards.ts";
import { NameDialog } from "./NameDialog.tsx";

/** A new dashboard has no name until the person types one. */
const noName = "";

/** How many dashboards of one list the sidebar shows. */
const shown = 20;

/** *Create a dashboard*: asks for its name and opens it, private and empty, in *Change*. */
function CreateDashboard({ view }: { readonly view: SidebarSectionView }) {
  const [open, setOpen] = useState(false);
  const create = useCreateDashboard();
  const navigate = useNavigate();
  return (
    <li>
      <NavButton
        icon={iconName.plus}
        narrow={view.narrow}
        label={t("dashboards.create")}
        isPending={create.isPending}
        onPress={() => {
          setOpen(true);
        }}
      />
      <NameDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={t("dashboards.create")}
        text={t("dashboards.create-text")}
        confirm={t("dashboards.create-confirm")}
        start={noName}
        onConfirm={(name) => {
          create.mutate(name, {
            onSuccess: (made: Dashboard) => {
              view.onNavigate?.();
              void navigate({ href: dashboardAddress(made.id) });
            },
          });
        }}
      />
    </li>
  );
}

/**
 * *Dashboards* in the sidebar (UX §4.10): the dashboards the person made and the ones shared with them, each shared one
 * with who made it, and *Create a dashboard*. *Home* is the start page and is not listed.
 */
export function DashboardsSection({ view }: { readonly view: SidebarSectionView }) {
  const mine = useDashboardList(dashboardSet.mine, shown);
  const shared = useDashboardList(dashboardSet.shared, shown);
  return (
    <MineAndShared
      view={view}
      heading={dashboardsHeading}
      icon={iconName.dashboards}
      mine={{ ...mine, items: itemsOf(mine.data?.pages) }}
      shared={{ ...shared, items: itemsOf(shared.data?.pages) }}
      addressOf={dashboardAddress}
      none={t("dashboards.none")}
      after={<CreateDashboard view={view} />}
    />
  );
}
