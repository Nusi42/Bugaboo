import { fieldAddress } from "../../src/shared/paths.ts";
import { signedInAdmin } from "./access.ts";
import { createAdmin } from "./accounts.ts";
import type { ApiSession } from "./api.ts";
import { fieldOf } from "./fields.ts";
import { createProject } from "./projects.ts";
import { inOrder } from "./sequence.ts";
import type { Instance } from "./server.ts";
import { choiceNamed } from "./time.ts";

/** The cost centers the pictures and the tests of numbered choices use: a number and a name each, and one with no number. */
export const costCenters = [
  { number: "5200", name: "Sales" },
  { number: "4100", name: "Product development" },
  { number: "0420", name: "Quality assurance" },
  { number: undefined, name: "Overhead" },
] as const;

/** How the cost centers read in a list, by number: what every screen writes for them. */
export const costCenterLabels = [
  "0420 Quality assurance",
  "4100 Product development",
  "5200 Sales",
  "Overhead",
] as const;

/** Adds the cost centers to the starter field *Cost center*, one after the other (each change counts the version up). */
export async function addCostCenters(admin: ApiSession): Promise<void> {
  const field = await fieldOf(admin, "Cost center");
  await inOrder(
    costCenters.map((each) => async () => {
      await choiceNamed(admin, field.id, each.name, each.number);
    }),
  );
}

/** A server with the project WEB and the numbered cost centers: what the pictures of numbered choices start from. */
export async function prepareCostCenters(server: Instance): Promise<void> {
  const admin = await createAdmin(server);
  await createProject(admin, "WEB", "Website");
  await addCostCenters(admin);
  await admin.dispose();
}

/** The address of the page of the field *Cost center*. */
export async function costCenterFieldAddress(server: Instance): Promise<string> {
  const admin = await signedInAdmin(server);
  const field = await fieldOf(admin, "Cost center");
  await admin.dispose();
  return fieldAddress(field.id);
}
