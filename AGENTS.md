# tanaid

## Pull request descriptions

- Use one `## Description` heading.
- Open with a short prose paragraph that gives context and motivation. Say where things stood before, linking the earlier PR (`Previously in #30, ...`), what this PR changes, and why. First person is fine.
- In the same paragraph, name the scope and known gaps (`limited to single key events; lacks mouse events`) so reviewers don't have to find them.
- Follow with a bullet list of changes, ordered by impact: architecture first, then features, then renames and samples.
- Start each top-level bullet with a present-tense verb (`Adds`, `Replaces`, `Renames`), and keep it to the change itself.
- Put details in nested sub-bullets: what the change means for callers, migration notes (`You should no longer pollster::block_on an eval`), and parts that aren't integrated yet.
- Describe API changes as what replaces what, with migration notes, rather than labeling them "breaking".
- Mention new test utilities when they change how tests are written.
- Leave out small fixes that ride along with the main change, routine validation, command lists, and commit history.
- Avoid inverted passive phrasing (`versions use`, `checks are performed`), drama, and obvious assurances.
- Don't add a generated-by footer.
- End with an issue-closing reference when one applies.
