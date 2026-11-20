import { t } from "@bugaboo/messages";

import { wholeNumberIn } from "../../shared/number.ts";
import { type SettingControlProps } from "./SettingControl.tsx";
import { SettingBox } from "./SettingBox.tsx";
import { unitWords } from "./texts.ts";
import { useSettingBox } from "./use-setting-box.ts";

/**
 * A text or a number the person types, saved with its own button; a number that counts years has its
 * unit beside the box, in words, so the box holds the number only.
 */
export function TypedControl({ setting, onChange, help, source, noteFor }: SettingControlProps) {
  const bounds = setting.bounds ?? undefined;
  const number = setting.kind === "whole-number";
  const unitHere = number && unitWords(setting, 0) !== undefined;
  const between =
    bounds === undefined
      ? undefined
      : t(unitHere ? "setting.ui.between-amounts" : "setting.ui.between", {
          least: bounds.least,
          most: bounds.most,
        });
  const box = useSettingBox({
    setting,
    help,
    between,
    source,
    initial: String(setting.value),
    read: (text) => {
      if (!number) {
        return { value: text };
      }
      const value = wholeNumberIn(text);
      return value === null ? { problem: between ?? t("setting.ui.whole-number") } : { value };
    },
    onChange,
  });
  const count = wholeNumberIn(box.draft) ?? Number(setting.value);
  const unit = unitWords(setting, count);
  // The note speaks of a number the box accepts; of another it says nothing, and the box says what is wrong.
  const inBounds = bounds === undefined || (count >= bounds.least && count <= bounds.most);
  const note = number && inBounds ? noteFor?.(count) : undefined;
  return (
    <SettingBox
      {...box}
      {...(unit === undefined ? {} : { unit })}
      {...(note === undefined ? {} : { note })}
    />
  );
}
