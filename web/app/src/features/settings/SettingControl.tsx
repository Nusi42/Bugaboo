import { timeZones } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { ChoiceGroup, SelectField, Switch } from "@bugaboo/ui";

import { type Setting, type SettingValue } from "../../shared/my-settings.ts";
import { deviceTimeZone } from "../../shared/region.ts";
import { LengthControl } from "./LengthControl.tsx";
import { TypedControl } from "./TypedControl.tsx";
import { controlKindOf, wordsOf } from "./texts.ts";

/** What a control needs: the setting, and what to do with a new value. */
export interface SettingControlProps {
  readonly setting: Setting;
  /** The one-line explanation of a box that is typed in: it stands with the box, above its Save button. */
  readonly help?: string | undefined;
  /** Where the value comes from, for a box that is typed in: it stands under the hint of the box, above its Save button. */
  readonly source?: string | undefined;
  /** The new value, to be shown at once and sent. */
  readonly onChange: (value: SettingValue) => void;
}

/** The time zones the server knows (M2-F72), for the person to choose from; the first is the device's own. */
function timeZoneChoices(): readonly { readonly value: string; readonly label: string }[] {
  return [
    { value: "device", label: t("setting.ui.device-zone", { zone: deviceTimeZone() }) },
    ...timeZones.map((name) => ({ value: name, label: name })),
  ];
}

/**
 * The control that fits what a setting holds: a switch for yes or no, a few buttons or a list
 * for a fixed list of choices, a list of time zones, a box for a number or a text. The keys of
 * shortcuts and the start page have pages of their own and are not drawn here.
 */
export function SettingControl({ setting, onChange, help, source }: SettingControlProps) {
  const value = setting.value;
  const kind = controlKindOf(setting);
  if (kind === "switch" && typeof value === "boolean") {
    return <Switch label={wordsOf(setting.name_key)} isSelected={value} onChange={onChange} />;
  }
  if ((kind === "choices" || kind === "list") && typeof value === "string") {
    const choices = setting.choices.map((choice) => ({
      value: choice.value,
      label: wordsOf(choice.label_key),
    }));
    return kind === "choices" ? (
      <ChoiceGroup
        legend={wordsOf(setting.name_key)}
        choices={choices}
        value={value}
        onChange={onChange}
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
  if (kind === "time-zone" && typeof value === "string") {
    return (
      <SelectField
        label={wordsOf(setting.name_key)}
        choices={timeZoneChoices()}
        value={value}
        onChange={onChange}
      />
    );
  }
  if (kind === "none") {
    return null;
  }
  if (kind === "length") {
    // Keyed by the stored value, so the box shows the new value after a reset.
    return <LengthControl key={String(value)} {...{ setting, onChange, help, source }} />;
  }
  // Keyed by the stored value, so the box shows the new value after a reset.
  return <TypedControl key={String(value)} {...{ setting, onChange, help, source }} />;
}
