import { type SettingActions } from "../settings/index.ts";
import { changeMySettings } from "../../shared/change-my-settings.ts";
import { isSettingKey, type Setting } from "../../shared/my-settings.ts";
import { useMySettings, useSaveProgress } from "../../shared/use-my-settings.ts";

/** What a page of *My settings* needs to draw some of the person's settings and change them. */
export interface PersonalForm {
  /** The settings whose keys start with one of the beginnings, as the server answered. */
  readonly settings: readonly Setting[];
  readonly actions: SettingActions;
  readonly saveState: "idle" | "saving" | "saved" | "not-saved";
  readonly problem: string | null;
}

/** The person's own settings whose keys start with one of `topics`, and what to do with them. */
export function usePersonalForm(topics: readonly string[]): PersonalForm {
  const all = useMySettings();
  const progress = useSaveProgress();
  return {
    settings: (all?.settings ?? []).filter((setting) =>
      topics.some((start) => setting.key.startsWith(start)),
    ),
    saveState: progress.state,
    problem: progress.problem,
    actions: {
      onChange: (key, value) => {
        if (isSettingKey(key)) {
          void changeMySettings([{ key, value }]);
        }
      },
      onReset: (key) => {
        if (isSettingKey(key)) {
          void changeMySettings([{ key, value: null }]);
        }
      },
    },
  };
}
