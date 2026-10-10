// Ajv validators over the contract schema the generated types come from.

import { Ajv2020 } from "ajv/dist/2020.js";
import { contractSchemas } from "../scripts/contract.ts";

export function validator(root: "Event" | "Submission") {
  const ajv = new Ajv2020({ strict: false });
  // schemars marks Rust's u32 with this format; Ajv ignores unknown ones.
  ajv.addFormat("uint32", {
    type: "number",
    validate: (value: number) => Number.isInteger(value) && value >= 0 && value <= 0xffff_ffff,
  });
  return ajv.compile(contractSchemas()[root]);
}
