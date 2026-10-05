import type { ReactNode } from "react";

import styles from "./Alert.module.css";
import { cx } from "./cx.ts";

/** Words of the ARIA standard, not text for people. */
const roles = { alert: "alert", status: "status" } as const;

export interface AlertProps {
  readonly children: ReactNode;
  /** Something went wrong. It is announced to screen readers at once. */
  readonly danger?: boolean;
  /** Something worked. */
  readonly success?: boolean;
  /** Something to be careful about. */
  readonly warning?: boolean;
}

/**
 * A message to the person. Problems are announced to screen readers at once; good news, warnings
 * and hints wait for a pause. Without a flag it is a hint.
 */
export function Alert({ children, danger = false, success = false, warning = false }: AlertProps) {
  return (
    <div
      className={cx(
        styles.alert,
        danger && styles.danger,
        success && styles.success,
        warning && styles.warning,
        !danger && !success && !warning && styles.info,
      )}
      role={danger ? roles.alert : roles.status}
    >
      {children}
    </div>
  );
}
