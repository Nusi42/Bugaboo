import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";
import { useBlocker } from "@tanstack/react-router";

export interface LeaveGuardProps {
  /** Whether something typed is not saved yet; read when the person leaves. */
  readonly isBusy: () => boolean;
}

/**
 * Guards a page with changes that are still being saved: the browser asks its own question before
 * the page is closed or reloaded, and the app asks in a window before it moves to another page.
 * Put it on a page once; it shows nothing until the person tries to leave.
 */
export function LeaveGuard({ isBusy }: LeaveGuardProps) {
  const blocker = useBlocker({
    shouldBlockFn: isBusy,
    enableBeforeUnload: isBusy,
    withResolver: true,
  });
  const asking = blocker.status === "blocked";
  return (
    <AskDialog
      isOpen={asking}
      onOpenChange={(open) => {
        if (!open) {
          blocker.reset?.();
        }
      }}
      title={t("common.leave-unsaved-title")}
      text={t("common.leave-unsaved-text")}
      confirmLabel={t("common.leave-unsaved-leave")}
      cancelLabel={t("common.leave-unsaved-stay")}
      confirmLook={buttonLook.danger}
      onConfirm={() => {
        blocker.proceed?.();
      }}
    />
  );
}
