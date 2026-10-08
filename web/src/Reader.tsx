// The message body is untrusted. The iframe has no sandbox flags, and the
// document's CSP refuses remote images and scripts. Markup in the body is
// escaped so it cannot become an element.

export function readerSrcDoc(body: string): string {
  return `<!DOCTYPE html><html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src 'none'; style-src 'unsafe-inline'"></head><body><p>${escapeHtml(body)}</p></body></html>`;
}

export function Reader({ body }: { body: string }) {
  return <iframe className="message-body" title="Message body" sandbox="" srcDoc={readerSrcDoc(body)} />;
}

function escapeHtml(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
