# Input: raw git log with fields separated by \x1f and records by \x1e.
# Format string: %h%x1f%cs%x1f%s%x1f%b%x1e

split("\u001e") |
map(ltrimstr("\n") | select(contains("\u001f"))) |
map(
  split("\u001f") |
  {
    hash:    .[0],
    date:    .[1],
    subject: .[2],
    body: (
      (.[3] // "") |
      split("\n") |
      map(ltrimstr("* ") | select(length > 0)) |
      join("\n")
    )
  }
) |
map(. + {
  type: ((.subject | capture("^(?<t>feat|fix|refactor|perf|style|docs|chore|ci|test|build|revert)[(!:]") | .t) // "other"),
  desc: ((.subject | capture("^[a-z]+(?:[^)]+\\))?!?: (?<d>.+)") | .d) // .subject)
}) |
map(select(
  .type != "chore" and
  .type != "ci" and
  .type != "test" and
  .type != "build" and
  .type != "revert" and
  (.subject | contains("[skip ci]") | not) and
  (.subject | startswith("Merge ") | not)
)) |
{
  version: $version,
  date: $date,
  previous: $prev,
  has_added: (map(select(.type == "feat")) | length > 0),
  has_fixed: (map(select(.type == "fix")) | length > 0),
  has_changed: (map(select(.type != "feat" and .type != "fix")) | length > 0),
  commits: .
}
