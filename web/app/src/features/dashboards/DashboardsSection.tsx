import { t } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui";

import { itemsOf } from "../../shared/paged.ts";
import { dashboardAddress } from "../../shared/paths.ts";
import {
  madeListShown,
  MineAndShared,
  NavButton,
  type SidebarSectionView,
} from "../shell/index.ts";
import { useCreateDashboardWindow } from "./CreateDashboard.tsx";
import { dashboardSet, dashboardsHeading, useDashboardList } from "./dashboards.ts";

/** *Create a dashboard* in the sidebar: a row of it. */
function CreateRow({ view }: { readonly view: SidebarSectionView }) {
  const create = useCreateDashboardWindow(view.onNavigate);
  return (
    <li>
      <NavButton
        icon={iconName.plus}
        narrow={view.narrow}
        label={t("dashboards.create")}
        isPending={create.pending}
        onPress={create.open}
      />
      {create.window}
    </li>
  );
}

/**
 * *Dashboards* in the sidebar (UX §4.10): the dashboards the person made and the ones shared with them, each shared one
 * with who made it, and *Create a dashboard*. *Home* is the start page and is not listed.
 */
export function DashboardsSection({ view }: { readonly view: SidebarSectionView }) {
  const mine = useDashboardList(dashboardSet.mine, madeListShown);
  const shared = useDashboardList(dashboardSet.shared, madeListShown);
  return (
    <MineAndShared
      view={view}
      heading={dashboardsHeading}
      icon={iconName.dashboards}
      mine={{ ...mine, items: itemsOf(mine.data?.pages) }}
      shared={{ ...shared, items: itemsOf(shared.data?.pages) }}
      addressOf={dashboardAddress}
      none={t("dashboards.none")}
      after={<CreateRow view={view} />}
    />
  );
}
