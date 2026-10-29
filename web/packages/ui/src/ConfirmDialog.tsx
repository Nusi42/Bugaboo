import { useId, useState, type ReactNode } from "react";
import { Dialog, DialogTrigger, Heading, Modal, ModalOverlay } from "react-aria-components";

import { Button } from "./Button.tsx";
import { useLayout } from "./layout.ts";
import { partOf } from "./parts.ts";
import { buttonLook, type ButtonLook } from "./props.ts";
import { part } from "./tokens.ts";

/** Words of React Aria Components and of the ARIA standard, not text for people. */
const dialogRole = "alertdialog";
const titleSlot = "title";

/**
 * A question is a short task, so it is a dialog, and on a phone it fills the screen (UX §6). It is
 * drawn with the one overlay part of the shell (the layer that menus and sheets use), in its
 * variant `question`, or `full`.
 */
function useQuestionVariant(): "question" | "full" {
  return useLayout() === "phone" ? "full" : "question";
}

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
  /** `danger` draws the confirming button in the color of warnings: the question is about a loss. */
  readonly confirmLook?: ButtonLook;
  readonly onConfirm: () => void;
  /** Something to fill in before confirming, for example a password. */
  readonly children?: ReactNode;
  /** The confirming button waits until what is filled in is complete. */
  readonly confirmDisabled?: boolean;
  /** Why the button waits, in words under what is filled in; shown only while it waits. */
  readonly confirmHint?: string;
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
  const variant = useQuestionVariant();
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
      <ModalOverlay {...partOf(part.overlay, { variants: [variant] })} isDismissable>
        <Modal {...partOf(part.overlayModal, { variants: [variant] })}>
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

export interface AskDialogProps extends Omit<ConfirmDialogProps, "triggerLabel" | "triggerName"> {
  /** Whether the question is showing. */
  readonly isOpen: boolean;
  /** Called with `false` when the question closes, whichever way. */
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * The same question as [`ConfirmDialog`], opened by the program and not by a button: for a question
 * that comes only when the server says a change takes something from the person. The focus moves
 * into it and goes back to where it was when it closes.
 */
export function AskDialog({ isOpen, onOpenChange, ...question }: AskDialogProps) {
  const variant = useQuestionVariant();
  return (
    <ModalOverlay
      {...partOf(part.overlay, { variants: [variant] })}
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable
    >
      <Modal {...partOf(part.overlayModal, { variants: [variant] })}>
        <Question
          {...question}
          close={() => {
            onOpenChange(false);
          }}
        />
      </Modal>
    </ModalOverlay>
  );
}

/** The inside of the dialog: the question, what to fill in, and the two answers. */
function Question({
  title,
  text,
  confirmLabel,
  cancelLabel,
  confirmLook = buttonLook.primary,
  onConfirm,
  children,
  confirmDisabled = false,
  confirmHint,
  close,
}: Omit<ConfirmDialogProps, "triggerLabel" | "triggerName"> & { readonly close: () => void }) {
  // The note that says why the main button waits is the description of that button.
  const hintId = useId();
  return (
    <Dialog {...partOf(part.confirmDialog)} role={dialogRole}>
      <Heading slot={titleSlot} {...partOf(part.confirmTitle)}>
        {title}
      </Heading>
      <div {...partOf(part.confirmBody)}>
        <p {...partOf(part.text)}>{text}</p>
        {children}
      </div>
      {confirmDisabled && confirmHint !== undefined ? (
        <p {...partOf(part.muted)} id={hintId}>
          {confirmHint}
        </p>
      ) : null}
      <div {...partOf(part.confirmActions)}>
        <Button look={buttonLook.quiet} onPress={close}>
          {cancelLabel}
        </Button>
        <Button
          look={confirmLook}
          isDisabled={confirmDisabled}
          {...(confirmDisabled && confirmHint !== undefined ? { "aria-describedby": hintId } : {})}
          onPress={() => {
            // The answer first: closing tells whoever listens that the question went away, and
            // that must not be taken for a "no".
            onConfirm();
            close();
          }}
        >
          {confirmLabel}
        </Button>
      </div>
    </Dialog>
  );
}
