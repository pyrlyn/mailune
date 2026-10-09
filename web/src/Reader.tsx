import { t } from "./i18n";
import type { SanitizedHtml } from "./message";

/**
 * The policy inside the reader frame. Mail is untrusted even after the core
 * sanitised it, so the frame loads nothing at all: no script, image, style
 * sheet, font or connection, and no form or `<base href>` can point anywhere.
 */
export const READER_POLICY = "default-src 'none'; base-uri 'none'; form-action 'none'";

/**
 * The frame document. The policy comes first so it applies before any body
 * markup is parsed. `target="_blank"` turns a link click into a popup, which
 * the sandbox refuses, so a link cannot navigate the frame to a remote page.
 */
export function readerDocument(body: SanitizedHtml): string {
  return [
    "<!doctype html><html><head>",
    `<meta http-equiv="Content-Security-Policy" content="${READER_POLICY}">`,
    '<meta charset="utf-8">',
    '<meta name="referrer" content="no-referrer">',
    '<base target="_blank">',
    "</head><body>",
    body,
    "</body></html>",
  ].join("");
}

/** A message body in a frame with every sandbox permission withheld. */
export function Reader({ body }: { body: SanitizedHtml }) {
  return (
    <iframe
      className="message-body"
      title={t("thread.message_body")}
      sandbox=""
      referrerPolicy="no-referrer"
      srcDoc={readerDocument(body)}
    />
  );
}
