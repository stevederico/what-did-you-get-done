---
name: what-did-you-get-done
description: >
  Write a detailed weekly report of what you shipped, or a short "What did you
  get done this week?" social post, from your real git activity. Use when asked
  for a weekly recap, weekly report, week in review, "what did I get done this
  week", or a Friday shipping post.
---

# what-did-you-get-done

Two outputs from one week of real commits: a **report** (detailed, for other people to read) and a **post** (short, for social). Read only. Never posts, pushes, or saves a file unless the user says yes.

## Pick the output

- "report", "recap", "week in review", "what did I get done": write the **report**
- "post", "friday post", "tweet", "share": write the **post**
- Both, or unclear: write the report first, then offer the post built from it as letter options

## Gather

1. `scripts/run.sh gather 7` (path relative to this skill; pass another day count if asked). For each repo it prints commit count, GitHub visibility, tags cut in the window, and the user's commit subjects with chores dropped. It ends with a **Contributions** block: repos, commits, lines added and removed, tags, pull requests
2. `scripts/run.sh tokens 7`: Claude Code tokens burned on this machine (fresh vs cache read, by model). Other tools and machines are not counted, so say "Claude Code on this machine"
3. Group repos that are one product
4. Skim subjects, do not paste them. Turn commits into what a user could see or do
5. Skip repos that are not the user's work and repos with only chore commits

The first run builds a small Rust binary (about 2 seconds, needs `cargo`). It starts from the GitHub CLI (`gh`), so it works without local clones. Local clones are found by scanning common folders and only add line counts. Override with `WDYGD_ROOT`, `WDYGD_AUTHORS`, `WDYGD_SKIP`.

## Report

**Print it in the conversation. Do not save it to a file.** The reader is someone else (a teammate, follower, or investor), so keep it self-contained:

- Describe projects by what they do. Name a project only if it is public or has a public product URL. Private repos become a generic line ("a fleet of iOS apps", "an internal agent workspace"). Never print private repo names, internal todo housekeeping, stalled items, or what was left out
- No "he" or "she", no session or tool chatter
- Short lines, scannable, no em dashes. Numbers are fine here

```
# Week ending YYYY-MM-DD

## Highlights
- 3 to 5 bullets: the headline ships and the week's theme

## Shipped
### <Product or group>
- What changed for users, 3 to 5 lines
- Link if public (product URL or repo)

## In progress
- 2 to 4 lines, generic

## By the numbers
- Commits, repos touched, lines added and removed, version releases, pull requests (opened and merged)
- Tokens: total, fresh tokens (input, output, cache write), cache reads, by model, Claude Code on one machine only

## Next
- 3 to 4 generic picks
```

- Check every claim first: a live URL answers 200 (`curl -A Mozilla/5.0`), a repo is public per the script, a version exists as a tag. Leave out anything unverified
- Never invent a productive week or pad the list
- After printing, offer to save it or turn it into the post

## Post

Build it from the report's Shipped section. Only real user-facing ships. Private repos get the product URL or just the name, never a 404 GitHub link.

```
What Did You Get Done This Week?

<one line: how the week felt, lowercase, honest>

• <name> - <what you did>
• <name> - <what you did>

<bare urls, one per line, same order as the list>

<one closing line>
```

- Names and beats in the list, urls in a block below. Never a long `github.com/...` url on a beat line, since feeds truncate them
- No weekly totals (commits, repos, PRs) in the post. Numbers belong in the report
- No hashtags, no emoji, no em dashes
- Only post or schedule when the user says yes this turn

## Do not

- Count chore commits as ships
- Include private repo names, credentials, paths, or emails
- Post, schedule, or push without a yes this turn
