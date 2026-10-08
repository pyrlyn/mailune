# text-sanitize

Header and plain-text sanitising is still in flight. `mailune-mime` does not depend on a sanitiser, and this tree does not add a second one.

Checked 2026-10-08:

- `https://crates.io/api/v1/crates/text-sanitize` returned 404.
- The shared `packages/crates` workspace has no `text-sanitize` crate.

When that crate lands, MIME header and plain-text values should go through it. Until then the parse path leaves the step out.
