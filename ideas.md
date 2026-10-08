# Ideas

Not approved yet. Nothing moves from here into `roadmap.md` or `plan.md` without the creator's approval.

- **BoltFFI instead of UniFFI.** BoltFFI 0.31 generates Swift, Kotlin, C# and WASM bindings from one tool, and Crux has moved to it. It is about 8 months old. Revisit it after Ph2.
- **usearch for vectors.** Switch to usearch when the in-SQLite KNN (S8) stops meeting its latency budget at the target mailbox size.
- **Shared view-model core.** A Crux-style pure UI core in Rust (`mailune-app` view models as a reducer), so that every shell renders the same state machine.
- **Calendar app.** Grow the scheduling assistant (A22) and Graph calendar access (P25) into a calendar view once JMAP Calendars is published as an RFC.
- **Team features.** Shared inboxes and comments, like Spark and Missive. This needs a server, so it conflicts with the no-server-in-the-path principle unless it is built on JMAP Sharing (RFC 9670).
