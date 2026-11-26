import { t } from "@bugaboo/messages";
import { ActionMenu, Button, MoreButton } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { paths } from "../../shared/paths.ts";
import { dashboardAddress } from "../../shared/places.ts";
import {
  restoreDashboard,
  useCopyDashboard,
  useDeleteDashboard,
  type Dashboard,
} from "./dashboards.ts";
import { NameDialog } from "./NameDialog.tsx";

/** *More* of the maker: *Rename* and *Delete* (to the trash, with *Undo*). */
export function MakerMenu({
  dashboard,
  onRename,
}: {
  readonly dashboard: Dashboard;
  readonly onRename: () => void;
}) {
  const remove = useDeleteDashboard();
  const navigate = useNavigate();
  const deleteIt = (): void => {
    remove.mutate(dashboard.id, {
      onSuccess: () => {
        undoStore.offer({
          message: t("dashboards.deleted", { name: dashboard.name }),
          undo: () => {
            void restoreDashboard(dashboard.id);
          },
        });
        void navigate({ href: paths.home });
      },
    });
  };
  return (
    <ActionMenu
      label={t("dashboards.more", { name: dashboard.name })}
      cancelLabel={t("common.cancel")}
      trigger={MoreButton}
      actions={[
        { id: "rename", label: t("dashboards.rename"), onAction: onRename },
        { id: "delete", label: t("dashboards.delete"), onAction: deleteIt },
      ]}
    />
  );
}

/** *Save a copy* of a dashboard of another person: private and their own, with the boxes they see. */
export function SaveCopy({ dashboard }: { readonly dashboard: Dashboard }) {
  const [open, setOpen] = useState(false);
  const copy = useCopyDashboard(dashboard.id);
  const navigate = useNavigate();
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("dashboards.save-copy")}
      </Button>
      <NameDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={t("dashboards.save-copy")}
        text={t("dashboards.save-copy-text")}
        confirm={t("dashboards.save-copy")}
        start={t("dashboards.copy-name", { name: dashboard.name })}
        onConfirm={(name) => {
          copy.mutate(name, {
            onSuccess: (made: Dashboard) => {
              void navigate({ href: dashboardAddress(made.id) });
            },
          });
        }}
      />
    </>
  );
}
