// Local stand-in for a mailbox. Later tasks read this instead of a server.

export interface ThreadFixture {
  id: string;
  folder: string;
  from: string;
  subject: string;
  snippet: string;
  body: string;
}

export const threads: ThreadFixture[] = [
  {
    id: "build",
    folder: "Inbox",
    from: "Ada",
    subject: "Build notes",
    snippet: "The build is ready.",
    body: "The build is ready. No further action.",
  },
  {
    id: "thursday",
    folder: "Inbox",
    from: "Grace",
    subject: "Thursday",
    snippet: "Can we meet Thursday?",
    body: "Can we meet Thursday afternoon?",
  },
];

export interface ThreadAi {
  summary: string;
  replies: string[];
}

export const threadAi: Record<string, ThreadAi> = {
  build: {
    summary: "The build is ready.",
    replies: ["Thanks", "On it", "Later"],
  },
  thursday: {
    summary: "Grace wants to meet Thursday.",
    replies: ["Yes", "Afternoon", "Next week"],
  },
};

export const assistResult = "Shortened: See you Thursday.";

export function chosenAi(id: string): ThreadAi {
  const found = threadAi[id];
  if (!found) {
    throw new Error("thread has no fixture summary");
  }
  return found;
}

export function chosenThread(list: ThreadFixture[], id: string): ThreadFixture {
  const found = list.find((thread) => thread.id === id);
  if (!found) {
    throw new Error("thread is not in the fixture");
  }
  return found;
}
