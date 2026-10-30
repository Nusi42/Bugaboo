import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, SelectField } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { SettingsForm, type ControlRenderer } from "../settings/index.ts";
import { usePersonalForm } from "./use-personal-form.ts";

export interface StartPageProps {
  /** The pages to choose from: the pages quick search knows (and, from WP-08, the projects). */
  readonly pages: readonly { readonly path: string; readonly title: string }[];
}

/** The choice of the page that opens after signing in, from the list of pages. */
function pagePicker(pages: StartPageProps["pages"]): ControlRenderer {
  return (setting, onChange) => {
    const current = String(setting.value);
    // A page that is not in the list any more (or not yet) is still shown, so nothing is lost.
    const choices = pages.some((page) => page.path === current)
      ? pages
      : [{ path: current, title: current }, ...pages];
    return (
      <SelectField
        label={t("nav.start-page")}
        choices={choices.map((page) => ({ value: page.path, label: page.title }))}
        value={current}
        onChange={onChange}
      />
    );
  };
}

/** *My settings › Start page* (CFG-04): the page that opens after signing in. */
export function StartPage({ pages }: StartPageProps) {
  usePageTitle(t("nav.start-page"));
  const form = usePersonalForm([settingKeys.startPage]);
  return (
    <Card heading={t("nav.start-page")} level={1} intro={t("setting.page.start-page-intro")}>
      <SettingsForm
        {...form}
        searchable={false}
        controls={{ [settingKeys.startPage]: pagePicker(pages) }}
      />
    </Card>
  );
}
