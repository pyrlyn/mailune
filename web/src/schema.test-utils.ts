// Ajv validators over the contract schema the generated types come from.

import { Ajv2020 } from "ajv/dist/2020.js";
import { contractSchemas } from "../scripts/contract.ts";

export function validator(root: "Event" | "Submission") {
  return new Ajv2020({ strict: false }).compile(contractSchemas()[root]);
}
