import { useId, type DOMAttributes, type KeyboardEventHandler, type ReactNode } from "react";
import { Button, Dialog, Heading } from "react-aria-components";

import { ActionMenu, type ActionMenuTriggerProps } from "./ActionMenu.tsx";
import { Icon } from "./Icon.tsx";
import { iconName } from "./icons.ts";
import { partOf } from "./parts.ts";
import { iconSize, windowState, type WindowState } from "./props.ts";
import { part } from "./tokens.ts";
import { windowEdges, windowSteps, type WindowEdge, type WindowStep } from "./window-size.ts";

/** The words on the buttons of a window, in the person's language. */
export interface WindowLabels {
  readonly minimize: string;
  readonly maximize: string;
  readonly restoreDown: string;
  readonly close: string;
  /** On a phone, in place of the three buttons. */
  readonly back: string;
  /** How to resize with the keyboard, read to screen readers when the focus enters the window. */
  readonly resizeKeys: string;
  /** What the bar of a minimized window says it keeps: *Draft kept*. */
  readonly draftKept: string;
  /** The button and the menu that change the size without dragging (WCAG 2.5.7): *Window size*. */
  readonly sizeMenu: string;
  /** The item of the size menu for each step. */
  readonly steps: Readonly<Record<WindowStep, string>>;
}

/** The pointer handlers of one place to drag. */
export type DragHandlers = Pick<
  DOMAttributes<HTMLElement>,
  "onPointerDown" | "onPointerMove" | "onPointerUp" | "onPointerCancel"
>;

/** How a window is resized: by the pointer at its edges, by the keys inside it and by the size menu. */
export interface WindowResize {
  readonly handlers: (edge: WindowEdge) => DragHandlers;
  readonly onKeyDown: KeyboardEventHandler<HTMLElement>;
  /** Changes the size by a step, for people who cannot drag. */
  readonly step: (step: WindowStep) => void;
}

export interface WindowFrameProps {
  readonly title: string;
  readonly labels: WindowLabels;
  readonly state: WindowState;
  /** The buttons of the form; the main one stands at the right (R6). */
  readonly footer?: ReactNode;
  readonly onMinimize: () => void;
  readonly onToggleMaximize: () => void;
  readonly onClose: () => void;
  readonly resize: WindowResize;
  readonly children: ReactNode;
}

/** The button of the title bar that opens the size menu. */
function SizeButton({ label, onOpen }: ActionMenuTriggerProps) {
  return <Control name={label} icon={iconName.resize} onPress={onOpen} />;
}

/** The size menu: a button of the title bar that opens the four steps. */
function SizeMenu({
  labels,
  onStep,
}: {
  readonly labels: WindowLabels;
  readonly onStep: (step: WindowStep) => void;
}) {
  return (
    <ActionMenu
      label={labels.sizeMenu}
      actions={windowSteps.map((step) => ({
        id: step,
        label: labels.steps[step],
        onAction: () => {
          onStep(step);
        },
      }))}
      trigger={SizeButton}
    />
  );
}

/** Words of the ARIA standard, not text for people. */
const titleSlot = "title";
const closeVariant = "close";

/** A button of the title bar: an icon and its name. */
function Control({
  name,
  icon,
  onPress,
  variant,
}: {
  readonly name: string;
  readonly icon: (typeof iconName)[keyof typeof iconName];
  readonly onPress: () => void;
  readonly variant?: typeof closeVariant;
}) {
  return (
    <Button
      {...partOf(part.windowControl, { variants: [variant] })}
      aria-label={name}
      onPress={onPress}
    >
      <Icon name={icon} size={iconSize.small} />
    </Button>
  );
}

/** The three buttons at the top right, in the order of Windows; on a phone one *Back*. */
function Controls({
  state,
  labels,
  resize,
  onMinimize,
  onToggleMaximize,
  onClose,
}: Pick<
  WindowFrameProps,
  "state" | "labels" | "resize" | "onMinimize" | "onToggleMaximize" | "onClose"
>) {
  if (state === windowState.full) {
    return (
      <Button {...partOf(part.windowControl, { variants: ["back"] })} onPress={onClose}>
        <Icon name={iconName.back} size={iconSize.small} />
        {labels.back}
      </Button>
    );
  }
  const maximized = state === windowState.maximized;
  return (
    <>
      {maximized ? null : <SizeMenu labels={labels} onStep={resize.step} />}
      <Control name={labels.minimize} icon={iconName.minimize} onPress={onMinimize} />
      <Control
        name={maximized ? labels.restoreDown : labels.maximize}
        icon={maximized ? iconName["restore-down"] : iconName.maximize}
        onPress={onToggleMaximize}
      />
      <Control name={labels.close} icon={iconName.close} onPress={onClose} variant={closeVariant} />
    </>
  );
}

/**
 * The inside of a window: the title bar with its buttons, the form that scrolls, the footer with
 * the buttons of the form, and a place to drag at each edge. The title bar and the footer stay in
 * place while the form scrolls (UX §6.3).
 */
export function WindowFrame(props: WindowFrameProps) {
  const { title, labels, state, footer, resize, children } = props;
  const hint = useId();
  const draggable = state === windowState.normal;
  return (
    <Dialog {...partOf(part.window)} {...(draggable ? { "aria-describedby": hint } : {})}>
      <div {...partOf(part.windowContent)} {...(draggable ? { onKeyDown: resize.onKeyDown } : {})}>
        <div {...partOf(part.windowTitleBar)}>
          <Heading slot={titleSlot} level={2} {...partOf(part.windowTitle)}>
            {title}
          </Heading>
          <Controls {...props} />
        </div>
        <div {...partOf(part.windowBody)}>{children}</div>
        {footer === undefined ? null : <div {...partOf(part.windowFooter)}>{footer}</div>}
        {draggable ? (
          <>
            <span id={hint} {...partOf(part.visuallyHidden)}>
              {labels.resizeKeys}
            </span>
            {windowEdges.map((edge) => (
              <div
                key={edge}
                {...partOf(part.windowHandle, { variants: [edge] })}
                {...resize.handlers(edge)}
                aria-hidden
              />
            ))}
          </>
        ) : null}
      </div>
    </Dialog>
  );
}
