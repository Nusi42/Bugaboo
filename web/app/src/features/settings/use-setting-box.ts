import { t } from "@bugaboo/messages";
import { useState } from "react";

import { type Setting, type SettingValue } from "../../shared/my-settings.ts";
import { wordsOf } from "./texts.ts";

/** What a box makes of what was typed: the value to send, or the sentence that says what is wrong. */
export type SettingRead = { readonly value: SettingValue } | { readonly problem: string };

export interface SettingBoxSetup {
  readonly setting: Setting;
  /** The one-line explanation of the setting. */
  readonly help: string | undefined;
  /** What the box accepts, in the words of its unit; also the sentence of a number outside the bounds. */
  readonly between: string | undefined;
  /** More for the hint, such as the number to go back to. */
  readonly extra?: string | undefined;
  /** Where the value comes from, above the Save button. */
  readonly source: string | undefined;
  /** The value as it is first written in the box. */
  readonly initial: string;
  /** Reads what was typed; each kind of box brings its own reader. */
  readonly read: (text: string) => SettingRead;
  readonly onChange: (value: SettingValue) => void;
}

/** What a typed-in box shows and does: the words and the state that every kind of box shares. */
export interface SettingBoxState {
  readonly label: string;
  readonly description: string | undefined;
  readonly problem: string | undefined;
  readonly draft: string;
  readonly onDraft: (next: string) => void;
  readonly source: string | undefined;
  readonly onSubmit: () => void;
}

/**
 * The state and the words every typed-in box shares: the draft, the problem, the hint and the check
 * of the bounds. A kind of box brings only its reader and its words.
 */
export function useSettingBox({
  setting,
  help,
  between,
  extra,
  source,
  initial,
  read,
  onChange,
}: SettingBoxSetup): SettingBoxState {
  const [draft, setDraft] = useState(initial);
  const [problem, setProblem] = useState<string>();
  const bounds = setting.bounds ?? undefined;
  // The hint is the explanation of the setting and, from the registry, the bounds it accepts: said once, never typed
  // into the explanation.
  const description =
    [help, between, extra].filter((piece) => piece !== undefined).join(" ") || undefined;
  const onSubmit = (): void => {
    const made = read(draft);
    if ("problem" in made) {
      setProblem(made.problem);
      return;
    }
    // A number outside the bounds is not sent: the server would only refuse it.
    if (
      bounds !== undefined &&
      typeof made.value === "number" &&
      (made.value < bounds.least || made.value > bounds.most)
    ) {
      setProblem(between ?? t("setting.ui.whole-number"));
      return;
    }
    setProblem(undefined);
    onChange(made.value);
  };
  return {
    label: wordsOf(setting.name_key),
    description,
    problem,
    draft,
    onDraft: (next) => {
      setDraft(next);
      setProblem(undefined);
    },
    source,
    onSubmit,
  };
}
