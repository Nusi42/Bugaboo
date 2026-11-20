import { QueryGate } from "../shell/index.ts";
import { useOrganizationSettings } from "./organization.ts";
import { SettingsForm, type SettingsFormProps } from "./SettingsForm.tsx";

export interface OrganizationTopicProps {
  /** The settings of this page: the beginnings of their keys, separated by commas. */
  readonly topics: string;
  /** Whether the form has a search box; a page of a few settings does not need one. */
  readonly searchable?: boolean;
  /** The explanations a page words itself for some settings (see the settings form). */
  readonly helps?: SettingsFormProps["helps"];
  /** The controls a page draws itself for some settings (a theme with its samples). */
  readonly controls?: SettingsFormProps["controls"];
  /** Hears that a change has been answered by the server, so a page reads again what follows from it. */
  readonly onSettled?: () => void;
}

/** The organization's settings of one page, drawn by the generic form, with the lock for administrators. */
export function OrganizationTopic({
  topics,
  searchable,
  helps,
  controls,
  onSettled,
}: OrganizationTopicProps) {
  const { settings, failed, edits } = useOrganizationSettings(topics);
  return (
    <QueryGate error={failed} data={settings} loading={null}>
      {(shown) => (
        <SettingsForm
          settings={shown.settings}
          {...(searchable === undefined ? {} : { searchable })}
          {...(helps === undefined ? {} : { helps })}
          {...(controls === undefined ? {} : { controls })}
          saveState={edits.saveState}
          problem={edits.problem}
          actions={{
            onChange: (key, value) => {
              edits.change({ kind: "change", key, value }, onSettled);
            },
            onReset: (key) => {
              edits.change({ kind: "reset", key }, onSettled);
            },
            onLock: (key, locked) => {
              edits.change({ kind: "lock", key, locked });
            },
          }}
        />
      )}
    </QueryGate>
  );
}
