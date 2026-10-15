import { Icon, iconName, SelectField, partOf, part, type SelectChoice } from "@bugaboo/ui";
import { useRef, type ReactNode } from "react";

import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { readOnlyReason } from "./read-only.ts";

export interface FactProps {
  /** The name of the field, in words. */
  readonly name: string;
  /** The value, or what changes it. */
  readonly children: ReactNode;
  /** Why the person cannot change it, when they cannot: shown with a lock. */
  readonly reason?: string | undefined;
  /** The ID of the box, so the bar of a phone can take the person to it. */
  readonly id?: string | undefined;
}

/**
 * One field of the facts bar or the details: its name, its value or the way to change it, and,
 * when it cannot be changed by this person, a lock and the reason (UX §4.4).
 */
export function Fact({ name, children, reason, id }: FactProps) {
  return (
    <div id={id} {...partOf(part.issueFact)}>
      <span {...partOf(part.issueFactName)}>{name}</span>
      <div {...partOf(part.issueFactValue)}>{children}</div>
      {reason === undefined ? null : (
        <p {...partOf(part.issueFactReason)}>
          <Icon name={iconName.lock} />
          <span>{reason}</span>
        </p>
      )}
    </div>
  );
}

export interface ChoiceFactProps<Value extends string> {
  /** The key that moves the focus to the list of words, if the field has one (`s` for the status). */
  readonly shortcut?: "change-status";
  readonly name: string;
  /** The ID of the box, so the bar of a phone can take the person to it. */
  readonly id?: string | undefined;
  readonly choices: readonly SelectChoice<Value>[];
  readonly value: Value;
  /** Whether the person may change it in this issue. */
  readonly canChange: boolean;
  readonly onPick: (value: Value) => void;
}

/** A field with a fixed list of words (*Status*, *Priority*): a list to pick from, or the word and why it is fixed. */
export function ChoiceFact<Value extends string>({
  shortcut,
  name,
  id,
  choices,
  value,
  canChange,
  onPick,
}: ChoiceFactProps<Value>) {
  const box = useRef<HTMLDivElement>(null);
  useShortcuts(
    shortcut === undefined || !canChange
      ? {}
      : {
          [shortcut]: () => {
            box.current?.querySelector("select")?.focus();
          },
        },
  );
  if (!canChange) {
    const current = choices.find((choice) => choice.value === value);
    return (
      <Fact id={id} name={name} reason={readOnlyReason(name)}>
        {current?.label ?? value}
      </Fact>
    );
  }
  return (
    <div ref={box} id={id} {...partOf(part.issueFact)}>
      <SelectField label={name} choices={choices} value={value} onChange={onPick} />
    </div>
  );
}
