# AI-generated tests

Everything in this folder was written by Claude (an AI assistant), not by a
human maintainer. The human-written tests are `tests/test_projects.rs` and the
`test_projects/` folder; nothing outside `tests/ai_generated/` was touched.

These tests come from a code review done on 2026-10-26. Each one encodes the
behaviour the language *should* have, so many of them fail on the current
`main`. A failing test here is a bug report, not a broken test. The `#N`
numbers in the comments match the findings in the review document.

Run only these tests with:

```bash
cargo test --test ai_generated
```

Layout:

- `main.rs`: the test target and shared helpers. These helpers run each case on
  its own thread with a timeout, so a panic or hang fails only that case.
- `wrong_results.rs`: programs that run but compute the wrong value.
- `crashes.rs`: inputs that panic, hang, or abort instead of returning an error.
