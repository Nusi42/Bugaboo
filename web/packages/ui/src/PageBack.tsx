import { Link } from "./Link.tsx";
import { useLayout } from "./layout.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/**
 * The link at the foot of a page that goes back up (*All people*, *All roles*): on a computer and a tablet it is
 * there; on a phone the bar of the screen has Back already, so the page does not say it twice.
 */
export function PageBack({ href, label }: { readonly href: string; readonly label: string }) {
  return useLayout() === "phone" ? null : (
    <div {...partOf(part.row)}>
      <Link href={href}>{label}</Link>
    </div>
  );
}
