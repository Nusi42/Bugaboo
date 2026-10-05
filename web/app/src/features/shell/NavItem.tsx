import { cx, Icon, type IconName } from "@bugaboo/ui";
import type { ReactNode } from "react";
import { Button, Link } from "react-aria-components";

import styles from "./NavItem.module.css";

/** Words of the ARIA standard, not text for people. */
const currentPage = "page";

interface ItemProps {
  readonly icon: IconName;
  /** What the item says; with icons only it is still there for screen readers. */
  readonly label: string;
  /** The sidebar shows icons only. */
  readonly narrow?: boolean;
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
  onNavigate,
}: NavLinkProps) {
  return (
    <Link
      className={cx(styles.item)}
      href={href}
      {...(narrow ? { title: label } : {})}
      {...(current ? { "aria-current": currentPage } : {})}
      {...(onNavigate === undefined ? {} : { onPress: onNavigate })}
    >
      <Icon name={icon} />
      <span className={narrow ? styles.hidden : styles.label}>{label}</span>
    </Link>
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
    <Button
      className={cx(styles.item)}
      onPress={onPress}
      {...(isExpanded === undefined ? {} : { "aria-expanded": isExpanded })}
      aria-busy={isPending}
      {...(narrow ? { title: label } : {})}
    >
      <Icon name={icon} />
      <span className={narrow ? styles.hidden : styles.label}>{label}</span>
      {narrow ? null : hint}
    </Button>
  );
}
