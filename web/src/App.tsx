import { InboxIcon } from "./icons";
import { t } from "./i18n";

export function App() {
  return (
    <h1>
      <InboxIcon />
      {t("app.mail")}
    </h1>
  );
}
