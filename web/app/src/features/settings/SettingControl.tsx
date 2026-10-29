import { t } from "@bugaboo/messages";
import {
  ChoiceGroup,
  Form,
  SelectField,
  SubmitButton,
  Switch,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { type Setting, type SettingValue } from "../../shared/my-settings.ts";
import { wholeNumberIn } from "../../shared/number.ts";
import { hasNoGenericControl, wordsOf } from "./texts.ts";

/** What a control needs: the setting, and what to do with a new value. */
export interface SettingControlProps {
  readonly setting: Setting;
  /** The one-line explanation of a box that is typed in: it stands with the box, above its Save button. */
  readonly help?: string | undefined;
  /** The new value, to be shown at once and sent. */
  readonly onChange: (value: SettingValue) => void;
}

/** The time zones of this browser, for the person to choose from; the first is the device's own. */
function timeZoneChoices(
  current: string,
): readonly { readonly value: string; readonly label: string }[] {
  const names = new Set<string>([
    ...Intl.supportedValuesOf("timeZone"),
    "UTC",
    ...(current === "device" ? [] : [current]),
  ]);
  const device = new Intl.DateTimeFormat().resolvedOptions().timeZone;
  return [
    { value: "device", label: t("setting.ui.device-zone", { zone: device }) },
    ...[...names].toSorted().map((name) => ({ value: name, label: name })),
  ];
}

/** A text or a number the person types, saved with its own button. */
function TypedControl({ setting, onChange, help }: SettingControlProps) {
  const [draft, setDraft] = useState(String(setting.value));
  const [problem, setProblem] = useState<string>();
  const bounds = setting.bounds ?? undefined;
  const number = setting.kind === "whole-number";
  const between =
    bounds === undefined
      ? undefined
      : t("setting.ui.between", { least: bounds.least, most: bounds.most });
  const submit = (): void => {
    if (!number) {
      onChange(draft);
      return;
    }
    // A number that is none, or outside the bounds, is not sent: the server would only refuse it.
    const value = wholeNumberIn(draft, bounds);
    if (value === null) {
      setProblem(between ?? t("setting.ui.whole-number"));
      return;
    }
    setProblem(undefined);
    onChange(value);
  };
  return (
    <Form onSubmit={submit}>
      <TextField
        label={wordsOf(setting.name_key)}
        {...(help === undefined ? {} : { description: help })}
        errorMessage={problem}
        value={draft}
        onChange={(next) => {
          setDraft(next);
          setProblem(undefined);
        }}
      />
      <div {...partOf(part.row)}>
        <SubmitButton>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/**
 * The control that fits what a setting holds: a switch for yes or no, a few buttons or a list
 * for a fixed list of choices, a list of time zones, a box for a number or a text. The keys of
 * shortcuts and the start page have pages of their own and are not drawn here.
 */
export function SettingControl({ setting, onChange, help }: SettingControlProps) {
  const value = setting.value;
  if (setting.kind === "yes-no" && typeof value === "boolean") {
    return <Switch label={wordsOf(setting.name_key)} isSelected={value} onChange={onChange} />;
  }
  if (setting.format === "choice" && typeof value === "string") {
    const choices = setting.choices.map((choice) => ({
      value: choice.value,
      label: wordsOf(choice.label_key),
    }));
    return choices.length <= 4 ? (
      <ChoiceGroup
        legend={wordsOf(setting.name_key)}
        choices={choices}
        value={value}
        onChange={onChange}
        stacked={choices.length > 2}
      />
    ) : (
      <SelectField
        label={wordsOf(setting.name_key)}
        choices={choices}
        value={value}
        onChange={onChange}
      />
    );
  }
  if (setting.format === "time-zone" && typeof value === "string") {
    return (
      <SelectField
        label={wordsOf(setting.name_key)}
        choices={timeZoneChoices(value)}
        value={value}
        onChange={onChange}
      />
    );
  }
  if (hasNoGenericControl(setting)) {
    return null;
  }
  // Keyed by the stored value, so the box shows the new value after a reset.
  return <TypedControl key={String(value)} setting={setting} onChange={onChange} help={help} />;
}
