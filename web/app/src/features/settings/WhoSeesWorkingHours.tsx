import { t } from "@bugaboo/messages";
import { Card, Link, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { QueryGate } from "../shell/index.ts";
import { useWhoSeesWorkingHours } from "./working-hours.ts";

/** The way to *Roles and permissions*, where the permission is given or taken away. */
function RolesLink() {
  return (
    <div {...partOf(part.row)}>
      <Link href={paths.roles}>{t("nav.roles")}</Link>
    </div>
  );
}

/** The groups and people that hold the permission, read through the routes of roles and rows. */
function Holders() {
  const { holders, failed } = useWhoSeesWorkingHours();
  return (
    <QueryGate error={failed} data={holders} loading={null}>
      {(shown) => (
        <div {...partOf(part.stack)}>
          {shown.length === 0 ? (
            <p {...partOf(part.text)}>{t("working-hours.who-nobody")}</p>
          ) : (
            <ul {...partOf(part.list)}>
              {shown.map((who) => (
                <li key={who.id}>{who.name}</li>
              ))}
            </ul>
          )}
          <RolesLink />
        </div>
      )}
    </QueryGate>
  );
}

/**
 * Who holds *See working hours of others* today: the groups and people that hold a role with it for
 * the whole organization, with the way to *Roles and permissions* (ADR-0035, decision 8). The roles
 * and the rows of *Who has which role* need *Manage roles* and *Decide who has which role*, which
 * the page does not: without both the card says so and points to the roles instead of asking.
 */
export function WhoSeesWorkingHours() {
  const held = usePermissions();
  const mayRead = held.includes("manage-roles") && held.includes("decide-roles");
  return (
    <Card heading={t("working-hours.who-title")} intro={t("working-hours.who-intro")}>
      {mayRead ? (
        <Holders />
      ) : (
        <div {...partOf(part.stack)}>
          <p {...partOf(part.text)}>
            {t("working-hours.who-not-allowed", {
              manage: t("permission.manage-roles"),
              decide: t("permission.decide-roles"),
            })}
          </p>
          <RolesLink />
        </div>
      )}
    </Card>
  );
}
