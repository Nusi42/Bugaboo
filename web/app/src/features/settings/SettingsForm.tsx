import { t } from "@bugaboo/messages";
import {
  Alert,
  containsEveryWord,
  alertTone,
  type SaveState,
  SaveStatus,
  SearchField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { type Setting } from "../../shared/my-settings.ts";
import { isLockedBelow, type SettingActions } from "./setting-actions.ts";
import { SettingRow, type ControlRenderer } from "./SettingRow.tsx";
import { hasNoGenericControl, isBox, wordsOf } from "./texts.ts";
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
  /** The explanation a page words itself for some settings, by key: one sentence for the state the setting is in. */
  readonly helps?: Readonly<Record<string, (setting: Setting) => string>>;
}

/** Whether the page draws the control of a setting itself (a theme with its sample). */
function hasOwnControl(setting: Setting, controls: SettingsFormProps["controls"]): boolean {
  return controls !== undefined && Object.hasOwn(controls, setting.key);
}

/** Whether a setting is drawn: by the generic controls, or by one the page brings. */
function isDrawn(setting: Setting, controls: SettingsFormProps["controls"]): boolean {
  return hasOwnControl(setting, controls) || !hasNoGenericControl(setting);
}

/** How a change of a setting is saved: at once, with its own Save button, or not at all (locked above this level). */
function howSaved(
  setting: Setting,
  actions: SettingActions,
  controls: SettingsFormProps["controls"],
): "at-once" | "with-save" | "never" {
  if (isLockedBelow(setting, actions)) {
    return "never";
  }
  return !hasOwnControl(setting, controls) && isBox(setting) ? "with-save" : "at-once";
}

/** Whether every word the person typed is in the name or the explanation of the setting. */
function matches(setting: Setting, query: string): boolean {
  return containsEveryWord(`${wordsOf(setting.name_key)} ${wordsOf(setting.help_key)}`, query);
}

/**
 * The setting changed last, whose box says *Saving…* and *Saved* (UX §6.4: where the change
 * happened, in a line that is there anyway), and the actions that note it.
 */
function useLastChanged(actions: SettingActions) {
  const [last, setLast] = useState<string | null>(null);
  const noted: SettingActions = {
    onChange: (key, value) => {
      setLast(key);
      actions.onChange(key, value);
    },
    onReset: (key) => {
      setLast(key);
      actions.onReset(key);
    },
    ...(actions.onLock === undefined
      ? {}
      : {
          onLock: (key: string, locked: boolean) => {
            setLast(key);
            actions.onLock?.(key, locked);
          },
        }),
  };
  return { last, noted };
}

/**
 * *Saving…* and *Saved* of a setting: every box has its status from the start, silent but for the
 * setting changed last, so a screen reader hears the first change too.
 */
function savedOf(state: SaveState) {
  return <SaveStatus state={state} labels={saveLabels()} keepsRoom />;
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
  helps,
}: SettingsFormProps) {
  const [query, setQuery] = useState("");
  const { last, noted } = useLastChanged(actions);
  const drawn = settings.filter((setting) => isDrawn(setting, controls));
  const shown = drawn.filter((setting) => matches(setting, query));
  // Where a switch stands beside a box with its own Save, the page says which is saved when (R7).
  const ways = new Set(drawn.map((setting) => howSaved(setting, actions, controls)));
  const twoWays = ways.has("at-once") && ways.has("with-save");
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
      {twoWays ? <p {...partOf(part.muted)}>{t("setting.ui.saved-at-once")}</p> : null}
      {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
      {shown.length === 0 ? (
        <p {...partOf(part.muted)}>{t("setting.ui.nothing")}</p>
      ) : (
        <ul {...partOf(part.list, { variants: ["loose"] })}>
          {shown.map((setting) => (
            <SettingRow
              key={setting.key}
              setting={setting}
              actions={noted}
              control={controls?.[setting.key]}
              help={helps?.[setting.key]?.(setting)}
              saved={savedOf(setting.key === last ? saveState : "idle")}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
