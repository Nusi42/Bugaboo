import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OwnThemes, themeKinds } from "../theme-editor/index.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";
import { useThemeControl } from "./theme-control.tsx";

/** The settings that belong to *Themes* (UX §5.1): the organization's defaults of how Bugaboo looks. */
const TOPICS = "appearance.";

/** The themes the organization's defaults offer besides the built-in ones: its themes for everyone. */
const ORGANIZATION_KINDS = [themeKinds.forEveryone] as const;

/**
 * *Organization settings › Look › Themes* (THM-02, THM-04, CFG-06): the organization's defaults,
 * which an administrator can lock, and under them the list *Themes for everyone* with *Create a
 * theme* and *Upload* (UX §5.6).
 */
export function ThemesPage() {
  usePageTitle(t("nav.themes"));
  const themeControl = useThemeControl(ORGANIZATION_KINDS);
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.themes")} level={1} intro={t("setting.page.themes-intro")}>
        <OrganizationTopic
          topics={TOPICS}
          controls={{
            [settingKeys.appearanceLightTheme]: themeControl,
            [settingKeys.appearanceDarkTheme]: themeControl,
          }}
        />
      </Card>
      <OwnThemes kind={themeKinds.forEveryone} />
    </div>
  );
}
