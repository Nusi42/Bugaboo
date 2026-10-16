import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  type SaveState,
  SaveStatus,
  SearchField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { type Setting } from "../../shared/my-settings.ts";
import { SettingRow, type ControlRenderer, type SettingActions } from "./SettingRow.tsx";
import { hasNoGenericControl, wordsOf } from "./texts.ts";
import { saveLabels } from "../../shared/save-labels.ts";

export interface SettingsFormProps {
  /** The settings of the topic, as the server answered. */
  readonly settings: readonly Setting[];
  readonly actions: SettingActions;
  /** *Saving…*, *Saved* or *Not saved yet*. */
  readonly saveState: SaveState;
  /** The plain message when the server refused a change. */
  readonly problem: string | null;
  /** Whether the form has a search box; a page of a few settings does not need one. */
  readonly searchable?: boolean;
  /** Controls a page draws for some settings itself, by key (a theme with its sample). */
  readonly controls?: Readonly<Record<string, ControlRenderer>>;
}

/** Whether a setting is drawn: by the generic controls, or by one the page brings. */
function isDrawn(setting: Setting, controls: SettingsFormProps["controls"]): boolean {
  const ownControl = controls !== undefined && Object.hasOwn(controls, setting.key);
  return ownControl || !hasNoGenericControl(setting);
}

/** Whether every word the person typed is in the name or the explanation of the setting. */
function matches(setting: Setting, query: string): boolean {
  const text = `${wordsOf(setting.name_key)} ${wordsOf(setting.help_key)}`.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/u)
    .filter((word) => word !== "")
    .every((word) => text.includes(word));
}

/**
 * The generic settings form (ADR-0018): a topic drawn from what the registry knows about each
 * setting, with a search box that filters by name and explanation. It knows no setting by name,
 * so a setting that is added to the registry appears here with its explanation and nothing else
 * to write.
 */
export function SettingsForm({
  settings,
  actions,
  saveState,
  problem,
  searchable = true,
  controls,
}: SettingsFormProps) {
  const [query, setQuery] = useState("");
  const shown = settings.filter((setting) => isDrawn(setting, controls) && matches(setting, query));
  return (
    <div {...partOf(part.stack)}>
      {searchable ? (
        <SearchField
          label={t("setting.ui.search")}
          clearLabel={t("search.clear")}
          value={query}
          onChange={setQuery}
        />
      ) : null}
      <SaveStatus state={saveState} labels={saveLabels()} />
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
      {shown.length === 0 ? (
        <p {...partOf(part.muted)}>{t("setting.ui.nothing")}</p>
      ) : (
        <ul {...partOf(part.list, { variants: ["loose"] })}>
          {shown.map((setting) => (
            <SettingRow
              key={setting.key}
              setting={setting}
              actions={actions}
              control={controls?.[setting.key]}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
