import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface ShowMoreProps {
  /** The words of the button, like *Show more people*: a screen with two of them keeps them apart. */
  readonly label: string;
  /** The next page is on its way. */
  readonly pending: boolean;
  readonly onPress: () => void;
}

/** *Show more* under a list that is read a page at a time: one button, on a row of its own, on every screen. */
export function ShowMore({ label, pending, onPress }: ShowMoreProps) {
  return (
    <div {...partOf(part.row)}>
      <Button isPending={pending} onPress={onPress}>
        {label}
      </Button>
    </div>
  );
}

/** What of a paged query *Show more* needs: the shape of a TanStack infinite query. */
export interface PagedList {
  readonly hasNextPage: boolean;
  readonly isFetchingNextPage: boolean;
  readonly fetchNextPage: () => unknown;
}

/** *Show more* for a paged query: drawn only while there is a next page, and it reads the next page. */
export function ShowMoreOf({ label, list }: { readonly label: string; readonly list: PagedList }) {
  return list.hasNextPage ? (
    <ShowMore
      label={label}
      pending={list.isFetchingNextPage}
      onPress={() => {
        void list.fetchNextPage();
      }}
    />
  ) : null;
}
