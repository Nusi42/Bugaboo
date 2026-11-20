import { t } from "@bugaboo/messages";

import { wholeNumberIn } from "../../shared/number.ts";
import { useSettled } from "../../shared/use-later.ts";
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
  const typed = wholeNumberIn(box.draft);
  const unit = unitWords(setting, typed ?? Number(setting.value));
  // The note speaks of a changed number the box accepts, once the person stops typing; of the stored number or one the
  // box does not take it says nothing, and the box says what is wrong.
  const inBounds =
    typed !== null && (bounds === undefined || (typed >= bounds.least && typed <= bounds.most));
  const changed = inBounds && typed !== Number(setting.value) ? typed : undefined;
  const settled = useSettled(changed);
  const note = number ? noteFor?.(changed === undefined ? undefined : settled) : undefined;
  return (
    <SettingBox
      {...box}
      {...(unit === undefined ? {} : { unit })}
      {...(note === undefined ? {} : { note })}
    />
  );
}
