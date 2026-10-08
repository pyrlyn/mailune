import { useState } from "react";
import { InboxIcon } from "./InboxIcon";
import { chosenThread, threads as fixtureThreads, type ThreadFixture } from "./fixture";
import { translate } from "./i18n";
import "./shell.css";

export function Shell() {
  const [selectedId, setSelectedId] = useState(fixtureThreads[0].id);
  return (
    <ShellView threads={fixtureThreads} selectedId={selectedId} onSelect={setSelectedId} />
  );
}

export function ShellView({
  threads,
  selectedId,
  onSelect,
}: {
  threads: ThreadFixture[];
  selectedId: string;
  onSelect: (id: string) => void;
}) {
  const selected = chosenThread(threads, selectedId);
  const folders = [...new Set(threads.map((thread) => thread.folder))];
  return (
    <div className="shell">
      <section className="pane" data-pane="folders" aria-label="Folders">
        <h1>
          <InboxIcon />
          {translate("en", "title")}
        </h1>
        <ul>
          {folders.map((folder) => (
            <li key={folder}>{folder === "Inbox" ? translate("en", "inbox") : folder}</li>
          ))}
        </ul>
      </section>
      <section className="pane" data-pane="list" aria-label="Threads">
        <ul>
          {threads.map((thread) => (
            <li key={thread.id}>
              <button
                type="button"
                aria-pressed={thread.id === selected.id}
                onClick={() => onSelect(thread.id)}
              >
                <span>{thread.from}</span>
                <span>{thread.subject}</span>
                <span>{thread.snippet}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>
      <section className="pane" data-pane="reading" aria-label="Reading">
        <h2>{selected.subject}</h2>
        <p>{selected.from}</p>
        <p>{selected.snippet}</p>
      </section>
    </div>
  );
}
