import { t } from "@bugaboo/messages";
import { Switch } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import styles from "./Roles.module.css";

export interface RoleSwitchProps {
  readonly id: string;
  readonly on: boolean;
  readonly onChange: (on: boolean) => void;
}

/** How many people with the role still have to set up a second step: the number that goes next to the switch. */
function Gap({ id }: { readonly id: string }) {
  const gap = useQuery(
    api.query.queryOptions("get", "/api/v1/roles/{role_id}/second-step-gap", {
      params: { path: { role_id: id } },
    }),
  );
  if (gap.data === undefined) {
    return null;
  }
  return (
    <p className={styles.muted}>
      {t("roles.second-step-gap", { count: gap.data.still_to_set_up })}
    </p>
  );
}

/** The switch *Two-step sign-in for everyone with this role* (AUTH-03), with the people who still need to set it up. */
export function RoleSwitch({ id, on, onChange }: RoleSwitchProps) {
  return (
    <div className={styles.stack}>
      <Switch label={t("roles.two-step")} isSelected={on} onChange={onChange} />
      <p className={styles.muted}>{t("roles.two-step-help")}</p>
      {on ? <Gap id={id} /> : null}
    </div>
  );
}
