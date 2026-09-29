# what-did-you-get-done

An agent skill that turns a week of your real git activity into a shareable report, or a short "What did you get done this week?" post.

It starts from your GitHub account, counts what you shipped, checks that links are live, and writes it up. Nothing is posted or saved unless you say so.

## What you get

- **Report:** highlights, shipped work by product, in progress, and a numbers block (commits, pull requests, reviews, releases, lines, tokens burned), printed in the chat
- **Post:** a short list of beats with the links underneath, ready to paste

## Install

```bash
npx skills add stevederico/what-did-you-get-done
```

Then ask your agent: "what did I get done this week?" or "write my weekly report".

## Requirements

- [`gh`](https://cli.github.com), logged in once with `gh auth login`. This is the main data source, so no local clones are needed
- `git`
- Rust with `cargo` ([rustup.rs](https://rustup.rs)). The first run builds the binary in about 2 seconds. It has zero crates and only uses the standard library
- Optional: Claude Code, for the token count

Without `gh`, it falls back to scanning local git repos with your `git config` identity.

## Configuration

Environment variables, all optional:

| Variable | Default | Meaning |
| --- | --- | --- |
| `WDYGD_ROOT` | current repo, then `~/Projects`, `~/code`, `~/dev`, `~/src`, `~/work`, `~/repos`, `~/Developer`, `~/git` | Folder of local clones (only adds line counts when `gh` is used) |
| `WDYGD_AUTHORS` | your `git config` name and email | Local fallback only: authors to count |
| `WDYGD_SKIP` | none | Comma-separated repos (`owner/name` or folder name) to ignore |

## Run it by hand

```bash
scripts/run.sh gather 7   # commits, releases, pull requests, lines for the last 7 days
scripts/run.sh tokens 7   # Claude Code tokens for the last 7 days
```

## Notes

- Private repos appear in the raw output. The skill tells the agent to describe them generically in the report
- Lockfiles, build output, and media are excluded from line counts
- Tokens count Claude Code on this machine only. Other tools and machines are not counted

## License

MIT
