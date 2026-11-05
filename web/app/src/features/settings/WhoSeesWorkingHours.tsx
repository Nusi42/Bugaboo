import { t } from "@bugaboo/messages";
import { Card, Link, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { QueryGate } from "../shell/index.ts";
import { useWhoSeesWorkingHours } from "./working-hours.ts";

/**
 * Who holds *See working hours of others* today: the groups and people that hold a role with it
 * for the whole organization, with a link to *Roles and permissions*, where it is given or taken
 * away (ADR-0035, decision 8). Who sees working hours is no setting here.
 */
export function WhoSeesWorkingHours() {
  const { holders, failed } = useWhoSeesWorkingHours();
  return (
    <Card heading={t("working-hours.who-title")} intro={t("working-hours.who-intro")}>
      <QueryGate error={failed} data={holders} loading={null}>
        {(shown) => (
          <div {...partOf(part.stack)}>
            {shown.length === 0 ? (
              <p>{t("working-hours.who-nobody")}</p>
            ) : (
              <>
                <p>{t("working-hours.who-holders")}</p>
                <ul {...partOf(part.list)}>
                  {shown.map((who) => (
                    <li key={who.id}>{who.name}</li>
                  ))}
                </ul>
              </>
            )}
            <div {...partOf(part.row)}>
              <Link href={paths.roles}>{t("nav.roles")}</Link>
            </div>
          </div>
        )}
      </QueryGate>
    </Card>
  );
}
