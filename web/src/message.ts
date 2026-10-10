// Message bodies reach the web only after mailune-mime's sanitiser. The
// brand keeps a plain string, such as a raw body or model output, from being
// handed to the reader by accident.

import hostileSnapshot from "../../crates/mailune-mime/src/snapshots/mailune_mime__html__tests__web_reader_hostile.snap?raw";

declare const sanitized: unique symbol;

/** HTML that mailune-mime's `sanitize_html` returned. */
export type SanitizedHtml = string & { readonly [sanitized]: true };

/**
 * The value of an insta snapshot written by mailune-mime. Only the core's
 * own output may be branded, and today that output arrives as this fixture.
 */
export function fromCoreSnapshot(snapshot: string): SanitizedHtml {
  return snapshot.replace(/^---\n[\s\S]*?\n---\n/, "").trimEnd() as SanitizedHtml;
}

/** A hostile message after the core sanitised it: scripts, trackers and forms are gone. */
export const fixtureBody = fromCoreSnapshot(hostileSnapshot);
