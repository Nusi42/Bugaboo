import { QueryGate } from "../shell/index.ts";
import { useOrganizationSettings } from "./organization.ts";
import { SettingsForm } from "./SettingsForm.tsx";

export interface OrganizationTopicProps {
  /** The settings of this page: the beginnings of their keys, separated by commas. */
  readonly topics: string;
}

/** The organization's settings of one page, drawn by the generic form, with the lock for administrators. */
export function OrganizationTopic({ topics }: OrganizationTopicProps) {
  const { settings, failed, edits } = useOrganizationSettings(topics);
  return (
    <QueryGate error={failed} data={settings} loading={null}>
      {(shown) => (
        <SettingsForm
          settings={shown.settings}
          saveState={edits.saveState}
          problem={edits.problem}
          actions={{
            onChange: (key, value) => {
              edits.change({ kind: "change", key, value });
            },
            onReset: (key) => {
              edits.change({ kind: "reset", key });
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
