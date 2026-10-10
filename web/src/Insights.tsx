import type { ThreadAi } from "./ai";
import { t } from "./i18n";

/** The summary and reply chips above a message. Choosing a chip only starts a draft. */
export function Insights({ ai, onReply }: { ai: ThreadAi; onReply: (text: string) => void }) {
  return (
    <aside className="insights">
      <section aria-label={t("thread.summary")}>
        <h3>{t("thread.summary")}</h3>
        <p>{ai.summary.text}</p>
      </section>
      <section aria-label={t("thread.suggested_replies")}>
        <ul className="chips">
          {ai.replies.map((reply) => (
            <li key={reply}>
              <button type="button" className="chip" onClick={() => onReply(reply)}>
                {reply}
              </button>
            </li>
          ))}
        </ul>
      </section>
    </aside>
  );
}
