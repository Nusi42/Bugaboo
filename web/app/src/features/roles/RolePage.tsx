import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { paths } from "../../shared/paths.ts";
import { NotFoundPage, QueryGate, useItemTrail } from "../shell/index.ts";
import { RoleSections } from "./RoleSections.tsx";
import { useRole } from "./roles.ts";

/**
 * The page of one role (SEC-08, UX §5.2): its name and what it is used for, the switch for two-step
 * sign-in, the permissions as checkboxes with their sentences, and what can be done with the whole
 * role: reset it to the starting permissions, copy it, delete it.
 */
export function RolePage({ id }: { readonly id: string }) {
  const found = useRole(id);
  const detail: Schema<"RoleDetailBody"> | undefined = found.data;
  const role = detail?.role;
  usePageTitle(role?.name ?? t("nav.roles"));
  useItemTrail(paths.roles, role?.name);
  const shown =
    detail !== undefined && role !== undefined ? { role, used: detail.used_by } : undefined;
  return (
    <QueryGate error={found.error} data={shown} notFound={<NotFoundPage />}>
      {(page) => (
        <div {...partOf(part.stack)}>
          <RoleSections role={page.role} used={page.used} />
        </div>
      )}
    </QueryGate>
  );
}
