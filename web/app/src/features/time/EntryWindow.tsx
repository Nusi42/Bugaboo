import { Window } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";

export interface EntryWindowProps {
  readonly title: string;
  readonly isOpen: boolean;
  /** Called with `false` when the window closes (*Close*, *Back* or Esc). */
  readonly onOpenChange: (isOpen: boolean) => void;
  /** The buttons; the main one stands at the right (R6). */
  readonly footer: ReactNode;
  readonly children: ReactNode;
}

/**
 * The window every form of a time entry opens in (UX §4.9, §6.3): *Stop timer*, *Add time* and, in
 * the timesheet, the details of a row. They share one size, which the person gives it once.
 */
export function EntryWindow({ title, isOpen, onOpenChange, footer, children }: EntryWindowProps) {
  const windowProps = useWindowProps(windowForms.timeEntry);
  return (
    <Window
      {...windowProps}
      title={title}
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      footer={footer}
    >
      {children}
    </Window>
  );
}
