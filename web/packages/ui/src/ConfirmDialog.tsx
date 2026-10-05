import { useState, type ReactNode } from "react";
import { Dialog, DialogTrigger, Heading, Modal, ModalOverlay } from "react-aria-components";

import { Button } from "./Button.tsx";
import styles from "./ConfirmDialog.module.css";
import { cx } from "./cx.ts";
import { useLayout } from "./layout.ts";

/** Words of React Aria Components and of the ARIA standard, not text for people. */
const dialogRole = "alertdialog";
const titleSlot = "title";

export interface ConfirmDialogProps {
  /** The text of the button that opens the question. */
  readonly triggerLabel: string;
  /** The name for screen readers when the same button appears many times in a list. */
  readonly triggerName?: string;
  /** The question, as the heading of the dialog. */
  readonly title: string;
  /** What will happen, in a sentence. */
  readonly text: string;
  readonly confirmLabel: string;
  readonly cancelLabel: string;
  /** The confirming button is drawn in the color of warnings: the question is about a loss. */
  readonly danger?: boolean;
  readonly onConfirm: () => void;
  /** Something to fill in before confirming, for example a password. */
  readonly children?: ReactNode;
  /** The confirming button waits until what is filled in is complete. */
  readonly confirmDisabled?: boolean;
  /** Called when the question closes, whichever way, so what was filled in can be cleared. */
  readonly onClosed?: () => void;
}

/**
 * Asks "are you sure?" before something that cannot be taken back. The dialog keeps the focus
 * inside, closes with Escape, and gives the focus back to the button that opened it.
 */
export function ConfirmDialog(props: ConfirmDialogProps) {
  const { triggerLabel, triggerName, onClosed } = props;
  const [isOpen, setOpen] = useState(false);
  // A short task is a dialog, and on a phone the dialog fills the screen (UX §6).
  const full = useLayout() === "phone";
  const changeOpen = (open: boolean): void => {
    setOpen(open);
    if (!open) {
      onClosed?.();
    }
  };
  return (
    <DialogTrigger isOpen={isOpen} onOpenChange={changeOpen}>
      <Button {...(triggerName === undefined ? {} : { "aria-label": triggerName })}>
        {triggerLabel}
      </Button>
      <ModalOverlay className={cx(styles.overlay, full && styles.full)} isDismissable>
        <Modal className={cx(styles.modal, full && styles.full)}>
          <Question
            {...props}
            close={() => {
              changeOpen(false);
            }}
          />
        </Modal>
      </ModalOverlay>
    </DialogTrigger>
  );
}

/** The inside of the dialog: the question, what to fill in, and the two answers. */
function Question({
  title,
  text,
  confirmLabel,
  cancelLabel,
  danger = false,
  onConfirm,
  children,
  confirmDisabled = false,
  close,
}: ConfirmDialogProps & { readonly close: () => void }) {
  return (
    <Dialog className={cx(styles.dialog)} role={dialogRole}>
      <Heading slot={titleSlot} className={cx(styles.title)}>
        {title}
      </Heading>
      <p className={styles.text}>{text}</p>
      {children}
      <div className={styles.actions}>
        <Button onPress={close}>{cancelLabel}</Button>
        <Button
          danger={danger}
          isDisabled={confirmDisabled}
          onPress={() => {
            close();
            onConfirm();
          }}
        >
          {confirmLabel}
        </Button>
      </div>
    </Dialog>
  );
}
