import { ada, type Person } from "./accounts.ts";

export { createPeopleWithRoles, roleName } from "./access.ts";
export { idInPeople } from "./privacy.ts";

/** A person created after the others of the tests whose name sorts before every one of them. */
export const aaron: Person = {
  name: "Aaron Archer",
  email: "aaron@example.test",
  password: ada.password,
};
