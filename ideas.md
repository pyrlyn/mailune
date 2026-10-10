# Ideas

Not approved yet. Nothing moves from here into `roadmap.md` or `plan.md` without the creator's approval.

The previous five moved into `plan.md` as B10, S14, B11, P33, and P34.

- **List page under its budget.** At 100k messages, `Store::thread_page` takes 220 ms against the 50 ms budget (T6 bench). The time is in its query: threads filtered by `eq_any` over a memberships join. A mailbox-to-threads index, or a denormalised latest-per-mailbox table, would let the page read 50 rows instead of scanning the inbox.
- **Warm the vector cache after open.** The first search after open loads and decrypts every vector (1.31 s at 100k); every later one takes 71 ms. `mailune-app` could call `nearest` with any query on a background task after open, so the first user search is already warm.
