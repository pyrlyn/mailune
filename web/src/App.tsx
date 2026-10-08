import { InboxIcon } from "./InboxIcon";
import { translate } from "./i18n";

export function App() {
  return (
    <main>
      <InboxIcon />
      <h1>{translate("en", "title")}</h1>
    </main>
  );
}
