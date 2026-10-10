// What the on-device AI hands the web client, from fixtures until the RPC
// carries it. The mailune-ai result types derive no serde or JSON Schema
// yet, so these mirror them by hand and should be generated once they do:
//   summary  — `SummaryCache` text for a `SummaryKind` (summary.rs)
//   replies  — three suggestions, as A13 specifies
//   assist   — the `assist` string for a `ComposeAction` (compose.rs)
// Model output is untrusted text: it is rendered as text, never as HTML.

export type SummaryKind = "short" | "detailed" | "action_items";

export type Tone = "formal" | "friendly" | "direct";

export type ComposeAction = "draft" | "rewrite" | "shorten" | "proofread" | { tone: Tone };

export interface ThreadSummary {
  kind: SummaryKind;
  text: string;
}

export type ReplySuggestions = readonly [string, string, string];

export interface ThreadAi {
  summary: ThreadSummary;
  replies: ReplySuggestions;
}

export interface AssistResult {
  action: ComposeAction;
  text: string;
}

export const fixtureAi: Readonly<Record<string, ThreadAi>> = {
  build: {
    summary: { kind: "short", text: "Ada says the nightly build is ready and asks for a review." },
    replies: ["Thanks, I'll review it today.", "Looks good to me.", "Can it wait until Monday?"],
  },
  thursday: {
    summary: { kind: "short", text: "Grace wants to move the review to Thursday afternoon." },
    replies: ["Thursday works.", "Could we do Friday instead?", "Which time on Thursday?"],
  },
};

export const fixtureAssist: AssistResult = {
  action: "shorten",
  text: "The build is ready. Could you review it by Thursday?",
};
