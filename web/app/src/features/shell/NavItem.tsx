import { Icon, type IconName, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Button, Link, Tooltip, TooltipTrigger, VisuallyHidden } from "react-aria-components";

/** Words of the ARIA standard and of React Aria Components, not text for people. */
const currentPage = "page";
const tooltipSide = { end: "end" } as const;

interface ItemProps {
  readonly icon: IconName;
  /** What the item says; with icons only it is still there for screen readers. */
  readonly label: string;
  /** The sidebar shows icons only. */
  readonly narrow?: boolean;
  /** A number after the words, such as the notifications not read yet. Nothing for none. */
  readonly count?: string | undefined;
}

/** The icon and the words; with icons only the words are for screen readers and the tooltip. */
function Content({ icon, label, narrow, count, hint }: ItemProps & { readonly hint?: ReactNode }) {
  return (
    <>
      <Icon name={icon} />
      {narrow === true ? (
        <VisuallyHidden>{label}</VisuallyHidden>
      ) : (
        <span {...partOf(part.navItemLabel)}>{label}</span>
      )}
      {count === undefined ? null : <span {...partOf(part.navItemCount)}>{count}</span>}
      {narrow === true ? null : hint}
    </>
  );
}

/**
 * With icons only the item says what it is in a tooltip that shows when the pointer is over it and
 * also when it has the keyboard focus. Otherwise the item is what it is.
 */
function WithTooltip({
  narrow,
  label,
  children,
}: {
  readonly narrow: boolean;
  readonly label: string;
  readonly children: ReactNode;
}) {
  if (!narrow) {
    return children;
  }
  return (
    <TooltipTrigger delay={0} closeDelay={0}>
      {children}
      <Tooltip placement={tooltipSide.end} {...partOf(part.navItemTooltip)}>
        {label}
      </Tooltip>
    </TooltipTrigger>
  );
}

export interface NavLinkProps extends ItemProps {
  readonly href: string;
  /** The page of this link is open. */
  readonly current?: boolean;
  /** Called after the link was used, for a menu that closes. */
  readonly onNavigate?: () => void;
}

/** A link of the menu: an icon and its words. */
export function NavLink({
  icon,
  label,
  narrow = false,
  href,
  current = false,
  count,
  onNavigate,
}: NavLinkProps) {
  return (
    <WithTooltip narrow={narrow} label={label}>
      <Link
        {...partOf(part.navItem)}
        href={href}
        {...(current ? { "aria-current": currentPage } : {})}
        {...(onNavigate === undefined ? {} : { onPress: onNavigate })}
      >
        <Content icon={icon} label={label} narrow={narrow} count={count} />
      </Link>
    </WithTooltip>
  );
}

export interface NavButtonProps extends ItemProps {
  readonly onPress: () => void;
  /** Whether what the button opens is open. */
  readonly isExpanded?: boolean;
  readonly isPending?: boolean;
  /** Something after the words, such as the keys of a shortcut. Not shown when narrow. */
  readonly hint?: ReactNode;
}

/** A button of the menu that looks like its links. */
export function NavButton({
  icon,
  label,
  narrow = false,
  onPress,
  isExpanded,
  isPending = false,
  hint,
}: NavButtonProps) {
  return (
    <WithTooltip narrow={narrow} label={label}>
      <Button
        {...partOf(part.navItem, { states: [isPending && "busy"] })}
        onPress={onPress}
        {...(isExpanded === undefined ? {} : { "aria-expanded": isExpanded })}
        aria-busy={isPending}
      >
        <Content icon={icon} label={label} narrow={narrow} hint={hint} />
      </Button>
    </WithTooltip>
  );
}
