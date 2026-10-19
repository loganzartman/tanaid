# tanaid

## Pull request descriptions

- Use one `## Description` heading. Write for quick skimming: one thought per sentence, no semicolons or `, so` clause chains, and no low-level qualifiers.
- Open with 2 short paragraphs:
  - First, state where things stood and what problem that caused (separate sentences for setup and effect), linking relevant PRs (`#30`).
  - Second, state how this PR fixes it at a high level in third person (`This PR ...`).
- Follow with a tight bullet list of changes, ordered by impact:
  - Start each top-level bullet with a present-tense verb (`Adds`, `Replaces`, `Drains`) and state only the headline change (plus default, if any).
  - Limit sub-bullets to 1–3 single-sentence details: behavior boundaries, caller-visible changes, and notable tradeoffs. Omit unchanged invariants and details that repeat the intro.
- Leave out ride-along fixes, validation steps, commit history, passive phrasing, and generated-by footers.
- End with `closes #N` when an issue applies.
