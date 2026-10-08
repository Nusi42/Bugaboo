import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Link } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { roleAddress } from "../../shared/paths.ts";
import { roleLinesOf, useRoles } from "./roles.ts";
import styles from "./Roles.module.css";

/** The roles in the order of roles, each a link to its page, with what is special about it said in words. */
export function RoleList() {
  const roles = useRoles();
  const lines = roleLinesOf(roles.data?.pages);
  return (
    <div className={styles.stack}>
      {roles.isError ? <Alert tone={alertTone.danger}>{messageOf(roles.error)}</Alert> : null}
      <ul className={styles.list} aria-label={t("roles.list")}>
        {lines.map((role) => (
          <li key={role.id} className={styles.item}>
            <Link href={roleAddress(role.id)}>{role.name}</Link>
            <span className={styles.muted}>
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
        <div className={styles.row}>
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
