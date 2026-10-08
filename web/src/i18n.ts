// Two catalogs, loaded by language code. A missing key uses the English
// string. There is no extraction or compile step.

const english = {
  inbox: "Inbox",
  title: "Mailune",
} as const;

type MessageKey = keyof typeof english;

const german: Partial<Record<MessageKey, string>> = {
  inbox: "Posteingang",
};

const catalogs: Record<string, Partial<Record<MessageKey, string>>> = {
  en: english,
  de: german,
};

export function translate(code: string, key: MessageKey): string {
  const localized = catalogs[code]?.[key];
  if (localized !== undefined && localized !== "") {
    return localized;
  }
  return english[key];
}
