import { t } from "@bugaboo/messages";
import { Alert, alertTone, LinkRow, partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { detailsText } from "../../shared/list-text.ts";
import { roleAddress } from "../../shared/paths.ts";
import { roleLinesOf, useRoles } from "./roles.ts";

/** The roles in the order of roles, each a link to its page, with what is special about it said in words. */
export function RoleList() {
  const roles = useRoles();
  const lines = roleLinesOf(roles.data?.pages);
  return (
    <div {...partOf(part.stack)}>
      {roles.isError ? <Alert tone={alertTone.danger}>{messageOf(roles.error)}</Alert> : null}
      <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("roles.list")}>
        {lines.map((role) => (
          <LinkRow
            key={role.id}
            href={roleAddress(role.id)}
            details={
              <span {...partOf(part.muted)}>
                {detailsText(
                  [
                    role.starting === null ? t("roles.own") : t("roles.starting-badge"),
                    role.twoStep ? t("roles.two-step-badge") : null,
                  ].filter((word) => word !== null),
                )}
              </span>
            }
          >
            {role.name}
          </LinkRow>
        ))}
      </ul>
      <ShowMoreOf label={t("roles.more")} list={roles} />
    </div>
  );
}
