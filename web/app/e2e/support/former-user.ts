import { t } from "@bugaboo/messages";

import { escaped } from "./showcase-masks.ts";

// How a removed person is shown (PRIV-04): *Former user* with the random code of the tombstone.

/** The code of the tombstone of a removed person, in place of which `formerUser` reads any code. */
const CODE = "CODE";

/** Where *Former user* stands for a removed person: the words, with any code of the tombstone. */
export const formerUser = new RegExp(
  t("person.former", { label: CODE })
    .split(CODE)
    .map((part) => escaped(part))
    .join("[A-Z0-9]+"),
  "u",
);
