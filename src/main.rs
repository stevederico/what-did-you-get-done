//! wdygd: one week of shipped work, from the GitHub CLI and local git.
//!
//!   wdygd gather [days]   commits, releases, pull requests, lines (default 7 days)
//!   wdygd tokens [days]   Claude Code tokens burned on this machine
//!
//! Read only. Never pushes or edits anything. std only, no crates.
//!
//! Environment (all optional):
//!   WDYGD_ROOT     folder to scan for local clones (default: the current repo, then common folders)
//!   WDYGD_AUTHORS  local fallback only: comma-separated author names or emails
//!   WDYGD_SKIP     comma-separated repo names (owner/name or folder) to ignore
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ROOTS: [&str; 10] = [
    "Projects", "projects", "code", "Code", "dev", "src", "work", "repos", "Developer", "git",
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("gather");
    let days: u64 = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(7);
    match cmd {
        "gather" => gather(days),
        "tokens" => tokens(days),
        _ => {
            eprintln!("usage: wdygd gather [days] | wdygd tokens [days]");
            std::process::exit(2);
        }
    }
}

// ---------- helpers ----------

fn run(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => String::new(),
    }
}

fn git(repo: &str, args: &[&str]) -> String {
    let mut all = vec!["-C", repo];
    all.extend_from_slice(args);
    run("git", &all)
}

fn home() -> PathBuf {
    PathBuf::from(env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_default())
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Unix seconds to `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days algorithm, no time crate).
fn iso(secs: u64) -> String {
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    let s = secs % 86400;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, s / 3600, (s % 3600) / 60, s % 60)
}

fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn fmt_tokens(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else {
        format!("{}K", (n as f64 / 1e3).round() as u64)
    }
}

fn list_env(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn is_chore(subject: &str) -> bool {
    let s = subject.to_lowercase();
    const WORDS: [&str; 10] = [
        "lockfile", "package-lock", "bump dep", "dependabot", "changelog", "typo", "merge branch",
        "merge pull", "prettier", "gitignore",
    ];
    if WORDS.iter().any(|w| s.contains(w)) || s.contains("readme only") || s.contains("lint") {
        return true;
    }
    s == "wip" || (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// Drop a leading version like `1.2.3 ` or `v1.2.3 `.
fn strip_version(s: &str) -> String {
    let t = s.strip_prefix('v').unwrap_or(s);
    let end = t.find(' ').unwrap_or(0);
    if end > 0 && t[..end].split('.').count() == 3 && t[..end].split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        return t[end + 1..].to_string();
    }
    s.to_string()
}

fn clean_subjects(subjects: &[String]) -> Vec<String> {
    subjects.iter().filter(|s| !s.is_empty() && !is_chore(s)).map(|s| strip_version(s)).collect()
}

fn skip_path(p: &str) -> bool {
    const PARTS: [&str; 8] = [
        "package-lock.json", "bun.lock", "yarn.lock", "Cargo.lock", "pnpm-lock.yaml", "node_modules", "/dist/", ".min.",
    ];
    const EXT: [&str; 6] = [".png", ".jpg", ".mp4", ".gif", ".webp", ".onnx"];
    PARTS.iter().any(|x| p.contains(x)) || p.starts_with("dist/") || EXT.iter().any(|x| p.ends_with(x))
}

fn cut(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// ---------- local repos ----------

fn remote_slug(repo: &str) -> String {
    let url = git(repo, &["remote", "get-url", "origin"]);
    let url = url.trim();
    if let Some(i) = url.find("github.com") {
        let rest = url[i + "github.com".len()..].trim_start_matches([':', '/']);
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 2 {
            return format!("{}/{}", parts[0], parts[1]).to_lowercase();
        }
    }
    String::new()
}

fn sub_dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p).map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).collect()).unwrap_or_default();
    v.sort();
    v.into_iter().filter(|p| p.is_dir()).collect()
}

fn find_local_repos() -> Vec<(String, String)> {
    let roots: Vec<PathBuf> = match env::var("WDYGD_ROOT") {
        Ok(r) if !r.is_empty() => vec![PathBuf::from(r.replacen('~', &home().to_string_lossy(), 1))],
        _ => ROOTS.iter().map(|r| home().join(r)).collect(),
    };
    let mut paths: Vec<PathBuf> = vec![];
    let top = git(&env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(), &["rev-parse", "--show-toplevel"]);
    if !top.trim().is_empty() {
        paths.push(PathBuf::from(top.trim()));
    }
    for root in roots {
        for p in sub_dirs(&root) {
            if p.join(".git").exists() {
                paths.push(p);
            } else if !p.file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false) {
                for q in sub_dirs(&p) {
                    if q.join(".git").exists() {
                        paths.push(q);
                    }
                }
            }
        }
    }
    let mut seen = HashSet::new();
    let mut out = vec![];
    for p in paths {
        let path = p.to_string_lossy().into_owned();
        let slug = remote_slug(&path);
        if !slug.is_empty() && seen.insert(slug.clone()) {
            out.push((slug, path));
        }
    }
    out.sort();
    out
}

fn local_numstat(path: &str, author_args: &[String], since: &str) -> (u64, u64) {
    let mut args: Vec<&str> = vec!["log", since, "--no-merges"];
    args.extend(author_args.iter().map(String::as_str));
    args.extend(["--numstat", "--format=tformat:@@"]);
    let out = git(path, &args);
    let (mut add, mut rem) = (0u64, 0u64);
    for line in out.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() == 3 {
            if let (Ok(a), Ok(r)) = (p[0].parse::<u64>(), p[1].parse::<u64>()) {
                if !skip_path(p[2]) {
                    add += a;
                    rem += r;
                }
            }
        }
    }
    (add, rem)
}

// ---------- gather ----------

const QUERY: &str = "query($login:String!,$from:DateTime!,$to:DateTime!){user(login:$login){contributionsCollection(from:$from,to:$to){\
totalCommitContributions totalPullRequestContributions totalPullRequestReviewContributions totalIssueContributions restrictedContributionsCount \
commitContributionsByRepository(maxRepositories:100){repository{nameWithOwner isPrivate isFork description} contributions{totalCount}} \
pullRequestContributions(first:100){nodes{pullRequest{merged}}}}}}";

// Flatten to tab separated lines so no JSON parser is needed.
const JQ: &str = r#".data.user.contributionsCollection | ("TOTALS\t\(.totalCommitContributions)\t\(.totalPullRequestContributions)\t\(.totalPullRequestReviewContributions)\t\(.totalIssueContributions)\t\(.restrictedContributionsCount)"), (.commitContributionsByRepository[] | "REPO\t\(.repository.nameWithOwner)\t\(.repository.isPrivate)\t\(.repository.isFork)\t\(.contributions.totalCount)\t\((.repository.description // "") | gsub("[\t\n\r]"; " "))"), (.pullRequestContributions.nodes[] | "PR\t\(.pullRequest.merged)")"#;

struct Repo {
    nwo: String,
    private: bool,
    fork: bool,
    commits: u64,
    desc: String,
}

fn gather(days: u64) {
    let now = now_secs();
    let from_iso = iso(now.saturating_sub(days * 86400));
    let to_iso = iso(now);
    let since = format!("--since={} days ago", days);
    let skip: HashSet<String> = list_env("WDYGD_SKIP").into_iter().map(|s| s.to_lowercase()).collect();

    let login = run("gh", &["api", "user", "-q", ".login"]).trim().to_string();
    if !login.is_empty() && gh_mode(&login, days, &from_iso, &to_iso, &since, &skip) {
        return;
    }
    local_mode(days, &since, &skip);
}

fn gh_mode(login: &str, days: u64, from_iso: &str, to_iso: &str, since: &str, skip: &HashSet<String>) -> bool {
    let out = run(
        "gh",
        &["api", "graphql", "-f", &format!("query={}", QUERY), "-F", &format!("login={}", login), "-F", &format!("from={}", from_iso), "-F", &format!("to={}", to_iso), "--jq", JQ],
    );
    if !out.starts_with("TOTALS") {
        return false;
    }
    let (mut totals, mut repos, mut prs_merged, mut prs_seen) = (vec![0u64; 5], vec![], 0u64, 0u64);
    for line in out.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "TOTALS" if f.len() >= 6 => totals = f[1..6].iter().map(|x| x.parse().unwrap_or(0)).collect(),
            "REPO" if f.len() >= 6 => repos.push(Repo {
                nwo: f[1].to_string(),
                private: f[2] == "true",
                fork: f[3] == "true",
                commits: f[4].parse().unwrap_or(0),
                desc: f[5].to_string(),
            }),
            "PR" => {
                prs_seen += 1;
                if f.get(1) == Some(&"true") {
                    prs_merged += 1;
                }
            }
            _ => {}
        }
    }
    let _ = prs_seen;
    repos.retain(|r| !skip.contains(&r.nwo.to_lowercase()));
    repos.sort_by(|a, b| b.commits.cmp(&a.commits));
    let local: HashMap<String, String> = find_local_repos().into_iter().collect();

    println!("# Week on GitHub ({} days, {} repos, login {})\n", days, repos.len(), login);
    println!("Judge what is a real user-facing ship. Chore-only repos are listed last.\n");
    let (mut chore_only, mut shown, mut add_total, mut rem_total, mut releases_total) = (vec![], 0, 0u64, 0u64, 0u64);
    for r in &repos {
        let subs: Vec<String> = run(
            "gh",
            &["api", &format!("repos/{}/commits?author={}&since={}&per_page=100", r.nwo, login, from_iso), "--jq", r#".[].commit.message | split("\n")[0]"#],
        )
        .lines()
        .map(|s| s.trim().to_string())
        .collect();
        let real = clean_subjects(&subs);
        let rels_raw = run(
            "gh",
            &["api", &format!("repos/{}/releases?per_page=30", r.nwo), "--jq", &format!(r#"[.[] | select(.published_at >= "{}") | .tag_name] | join(",")"#, from_iso)],
        );
        let rels: Vec<&str> = rels_raw.trim().split(',').filter(|s| !s.is_empty()).collect();
        releases_total += rels.len() as u64;
        if let Some(path) = local.get(&r.nwo.to_lowercase()) {
            let email = git(path, &["config", "user.email"]).trim().to_string();
            let mut authors = vec![format!("--author={}", login)];
            if !email.is_empty() {
                authors.push(format!("--author={}", email));
            }
            let (a, b) = local_numstat(path, &authors, since);
            add_total += a;
            rem_total += b;
        }
        if real.is_empty() {
            chore_only.push(r.nwo.clone());
            continue;
        }
        if shown >= 30 {
            continue;
        }
        shown += 1;
        let span = match rels.len() {
            0 => "no release this week".to_string(),
            1 => rels[0].to_string(),
            n => format!("{} to {}", rels[n - 1], rels[0]),
        };
        println!("## {}  ({} commits, {}{}, releases: {})", r.nwo, r.commits, if r.private { "private" } else { "public" }, if r.fork { ", fork" } else { "" }, span);
        if !r.desc.is_empty() {
            println!("_{}_", cut(&r.desc, 120));
        }
        for s in real.iter().take(8) {
            println!("- {}", cut(s, 150));
        }
        if real.len() > 8 {
            println!("- ... {} more", real.len() - 8);
        }
        println!();
    }
    if !chore_only.is_empty() {
        println!("## Chore-only (skip): {}\n", chore_only.join(", "));
    }
    println!("# Contributions\n");
    println!("- Repos with commits: {}", repos.len());
    if totals[4] > 0 {
        println!("- Commits: {} (plus {} in private repos not listed above)", totals[0], totals[4]);
    } else {
        println!("- Commits: {}", totals[0]);
    }
    println!("- Pull requests opened: {} (merged {})", totals[1], prs_merged);
    println!("- Pull request reviews: {}", totals[2]);
    println!("- Issues opened: {}", totals[3]);
    println!("- Releases published: {}", releases_total);
    if add_total > 0 || rem_total > 0 {
        println!("- Lines: +{} / -{} (local clones only; lockfiles, build output and media excluded)", commas(add_total), commas(rem_total));
    } else {
        println!("- Lines: not counted (no local clones found; set WDYGD_ROOT to a folder of clones)");
    }
    true
}

fn local_mode(days: u64, since: &str, skip: &HashSet<String>) {
    let mut names = list_env("WDYGD_AUTHORS");
    if names.is_empty() {
        for k in ["user.name", "user.email"] {
            let v = run("git", &["config", "--global", k]).trim().to_string();
            if !v.is_empty() {
                names.push(v);
            }
        }
    }
    if names.is_empty() {
        eprintln!("gh is not available and no git identity found. Run `gh auth login` or set git config user.name/user.email.");
        std::process::exit(1);
    }
    let author_args: Vec<String> = names.iter().map(|a| format!("--author={}", a)).collect();
    let local = find_local_repos();
    if local.is_empty() {
        eprintln!("No local repos found. Run `gh auth login`, or set WDYGD_ROOT to a folder of clones.");
        std::process::exit(1);
    }
    let (mut rows, mut commits, mut add, mut rem) = (vec![], 0u64, 0u64, 0u64);
    for (slug, path) in &local {
        let base = Path::new(path).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if skip.contains(slug) || skip.contains(&base) {
            continue;
        }
        let mut args: Vec<&str> = vec!["log", since, "--no-merges"];
        args.extend(author_args.iter().map(String::as_str));
        args.push("--format=%s");
        let subs: Vec<String> = git(path, &args).lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if subs.is_empty() {
            continue;
        }
        commits += subs.len() as u64;
        let (a, b) = local_numstat(path, &author_args, since);
        add += a;
        rem += b;
        rows.push((slug.clone(), subs.len(), clean_subjects(&subs)));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    println!("# Week in local git ({} days, {} repos, no GitHub login)\n", days, rows.len());
    for (slug, n, real) in rows.iter().filter(|r| !r.2.is_empty()).take(30) {
        println!("## {}  ({} commits)", slug, n);
        for s in real.iter().take(8) {
            println!("- {}", cut(s, 150));
        }
        println!();
    }
    println!("# Contributions\n");
    println!("- Repos touched: {}\n- Commits: {}\n- Lines: +{} / -{} (lockfiles, build output and media excluded)", rows.len(), commits, commas(add), commas(rem));
}

// ---------- tokens ----------

fn find_str<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\":\"", key);
    let i = line.find(&pat)? + pat.len();
    let j = line[i..].find('"')?;
    Some(&line[i..i + j])
}

fn find_num(text: &str, key: &str) -> u64 {
    let pat = format!("\"{}\":", key);
    match text.find(&pat) {
        Some(i) => text[i + pat.len()..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0),
        None => 0,
    }
}

fn walk_jsonl(dir: &Path, cutoff: SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            walk_jsonl(&p, cutoff, out);
        } else if p.extension().map(|x| x == "jsonl").unwrap_or(false) {
            if e.metadata().and_then(|m| m.modified()).map(|m| m >= cutoff).unwrap_or(false) {
                out.push(p);
            }
        }
    }
}

#[derive(Default, Clone)]
struct Usage {
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
    calls: u64,
}

impl Usage {
    fn add(&mut self, i: u64, o: u64, cw: u64, cr: u64) {
        self.input += i;
        self.output += o;
        self.cache_write += cw;
        self.cache_read += cr;
        self.calls += 1;
    }
    fn all(&self) -> u64 {
        self.input + self.output + self.cache_write + self.cache_read
    }
    fn line(&self) -> String {
        let fresh = self.input + self.output + self.cache_write;
        format!(
            "{} total | {} fresh (input {}, output {}, cache write {}) | {} cache read | {} model calls",
            fmt_tokens(self.all()), fmt_tokens(fresh), fmt_tokens(self.input), fmt_tokens(self.output), fmt_tokens(self.cache_write), fmt_tokens(self.cache_read), self.calls
        )
    }
}

fn tokens(days: u64) {
    let cutoff_secs = now_secs().saturating_sub(days * 86400);
    let cutoff = UNIX_EPOCH + Duration::from_secs(cutoff_secs);
    let cutoff_iso = iso(cutoff_secs);
    let mut files = vec![];
    walk_jsonl(&home().join(".claude").join("projects"), cutoff, &mut files);

    // Streaming repeats one message id, so keep the largest usage per (message id, request id).
    let mut best: HashMap<String, (u64, u64, u64, u64, String)> = HashMap::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else { continue };
        for line in text.lines() {
            let Some(u) = line.find("\"usage\":{") else { continue };
            let ts = find_str(line, "timestamp").unwrap_or("");
            if ts.is_empty() || ts < cutoff_iso.as_str() {
                continue;
            }
            let usage = &line[u..];
            let (i, o, cw, cr) = (find_num(usage, "input_tokens"), find_num(usage, "output_tokens"), find_num(usage, "cache_creation_input_tokens"), find_num(usage, "cache_read_input_tokens"));
            let model = find_str(line, "model").unwrap_or("unknown").to_string();
            let key = match find_str(line, "id").filter(|id| id.starts_with("msg_")) {
                Some(id) => format!("{}|{}", id, find_str(line, "requestId").unwrap_or("")),
                None => format!("{}|{}", f.display(), ts),
            };
            if best.get(&key).map(|b| o >= b.1).unwrap_or(true) {
                best.insert(key, (i, o, cw, cr, model));
            }
        }
    }
    let mut total = Usage::default();
    let mut by_model: HashMap<String, Usage> = HashMap::new();
    for (i, o, cw, cr, model) in best.into_values() {
        total.add(i, o, cw, cr);
        by_model.entry(model).or_default().add(i, o, cw, cr);
    }
    println!("# Claude Code tokens ({} days, this machine)\n", days);
    if total.calls == 0 {
        println!("No Claude Code transcripts found in ~/.claude/projects for this window.");
        return;
    }
    println!("All models: {}", total.line());
    let mut models: Vec<(String, Usage)> = by_model.into_iter().collect();
    models.sort_by(|a, b| b.1.all().cmp(&a.1.all()));
    for (m, u) in models.iter().filter(|(_, u)| u.calls >= 20) {
        println!("- {}: {}", m, u.line());
    }
}
