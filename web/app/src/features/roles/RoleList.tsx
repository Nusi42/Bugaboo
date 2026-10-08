import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Link, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { roleAddress } from "../../shared/paths.ts";
import { roleLinesOf, useRoles } from "./roles.ts";

/** The roles in the order of roles, each a link to its page, with what is special about it said in words. */
export function RoleList() {
  const roles = useRoles();
  const lines = roleLinesOf(roles.data?.pages);
  return (
    <div {...partOf(part.stack)}>
      {roles.isError ? <Alert tone={alertTone.danger}>{messageOf(roles.error)}</Alert> : null}
      <ul {...partOf(part.list)} aria-label={t("roles.list")}>
        {lines.map((role) => (
          <li key={role.id} {...partOf(part.rolesItem)}>
            <Link href={roleAddress(role.id)}>{role.name}</Link>
            <span {...partOf(part.muted)}>
              {[
                role.starting === null ? t("roles.own") : t("roles.starting-badge"),
                role.twoStep ? t("roles.two-step-badge") : null,
              ]
                .filter((word) => word !== null)
                .join(" · ")}
            </span>
          </li>
        ))}
      </ul>
      {roles.hasNextPage ? (
        <div {...partOf(part.row)}>
          <Button
            isPending={roles.isFetchingNextPage}
            onPress={() => {
              void roles.fetchNextPage();
            }}
          >
            {t("roles.more")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
