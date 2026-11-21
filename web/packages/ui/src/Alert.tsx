import type { ReactNode } from "react";

import { live } from "./live.ts";
import { partOf } from "./parts.ts";
import { alertTone, type AlertTone } from "./props.ts";
import { part } from "./tokens.ts";

export interface AlertProps {
  readonly children: ReactNode;
  /**
   * What the message is: a problem (announced to screen readers at once), good news, something
   * to be careful about, or a hint (the default).
   */
  readonly tone?: AlertTone;
}

/**
 * A message to the person. Problems are announced to screen readers at once; good news, warnings
 * and hints wait for a pause. Without a tone it is a hint.
 */
export function Alert({ children, tone = alertTone.info }: AlertProps) {
  return (
    <div
      {...partOf(part.alert, { variants: [tone] })}
      role={tone === alertTone.danger ? live.alert : live.status}
    >
      {children}
    </div>
  );
}
