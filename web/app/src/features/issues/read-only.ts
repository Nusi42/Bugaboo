import { t } from "@bugaboo/messages";

/** What is said under a field the person can see and not change (with the lock). */
export function readOnlyReason(name: string): string {
  return t("issues.read-only", { field: name });
}
