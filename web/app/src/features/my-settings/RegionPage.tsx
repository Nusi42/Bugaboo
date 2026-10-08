import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime, formatNumber } from "../../shared/format.ts";
import { useRegion } from "../../shared/region.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { SettingsForm } from "../settings/index.ts";
import { usePersonalForm } from "./use-personal-form.ts";

/** An example of a number, which shows how a decimal and a thousand are written. */
const EXAMPLE_NUMBER = 1234.5;

/** What dates, times and numbers look like with the person's choices, as they change them. */
function Example() {
  useRegion();
  const [now] = useState(() => new Date().toISOString());
  return (
    <p {...partOf(part.regionPageExample)}>
      <span>{t("setting.ui.example-moment", { moment: formatDateTime(now) })}</span>
      <span>{t("setting.ui.example-number", { number: formatNumber(EXAMPLE_NUMBER) })}</span>
    </p>
  );
}

/**
 * *My settings › Region* (CFG-04): time zone, date format, number format and first day of the
 * week, kept with the person on the server. Times and numbers on every page follow them.
 */
export function RegionPage() {
  usePageTitle(t("nav.region"));
  const form = usePersonalForm(["region."]);
  return (
    <Card heading={t("nav.region")} level={1}>
      <p>{t("setting.page.region-intro")}</p>
      <Example />
      <SettingsForm {...form} searchable={false} />
    </Card>
  );
}
