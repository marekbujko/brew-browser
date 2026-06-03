#!/usr/bin/env python3
"""
brew-browser — package categorizer.

Fetches Homebrew's published cask + formula data, diffs against prior state,
sends new/changed items to an LLM (OpenAI or Anthropic) for category
assignment, and writes the result to src-tauri/data/categories.json.

Designed to run offline (cron, manual). Never invoked from inside the app.

Auto-detects provider from environment:
- ANTHROPIC_API_KEY set → use claude-haiku-4-5
- OPENAI_API_KEY set    → use gpt-4o-mini
- both set              → ANTHROPIC wins (better at nuanced multi-label)
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import requests
from dotenv import load_dotenv

# ─────────────────────────────────────────────────────────────────────────────
# Paths
# ─────────────────────────────────────────────────────────────────────────────
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
OUTPUT_PATH = REPO_ROOT / "src-tauri" / "data" / "categories.json"
STATE_PATH = SCRIPT_DIR / "state" / "last-tokens.json"
LOG_PATH = SCRIPT_DIR / "state" / "cron.log"
PROMPT_SYSTEM = (SCRIPT_DIR / "prompts" / "system.txt").read_text()

# ─────────────────────────────────────────────────────────────────────────────
# Categories — single source of truth
# ─────────────────────────────────────────────────────────────────────────────
# slug: (label, lucide-icon-name, sf-symbol-name, short-description-for-prompt)
# The icon is chosen ONCE here and emitted to categories.json as both `icon`
# (lucide, for the Tauri web UI) and `iconSF` (SF Symbol, for the native macOS
# UI) — so neither UI re-decides icons in code. Adding a category? Pick both
# names here and they flow to both apps via the data.
CATEGORIES: dict[str, tuple[str, str, str, str]] = {
    "ai":               ("AI & ML",          "Brain",          "brain",                                       "LLM runtimes, model tools, AI-assisted utilities, vector DBs"),
    "browsers":         ("Browsers",         "Globe",          "globe",                                       "Web browsers and browser engines"),
    "cloud-devops":     ("Cloud & DevOps",   "Cloud",          "cloud",                                       "Kubernetes, container runtimes, IaC, cloud CLIs"),
    "communication":    ("Communication",    "MessageSquare",  "message",                                     "Chat, email, video calls, messaging clients"),
    "data":             ("Data",             "Database",       "cylinder.split.1x2",                          "Databases, query tools, ETL, data visualization"),
    "developer-tools":  ("Developer Tools",  "Code",           "chevron.left.forwardslash.chevron.right",     "Compilers, languages, package managers, linters, build tools, IDEs-CLI"),
    "editors":          ("Editors & IDEs",   "FileCode",       "curlybraces",                                 "Code editors and IDEs with GUI (vscode, sublime, zed)"),
    "education":        ("Education",        "GraduationCap",  "graduationcap",                               "Learning, tutoring, courseware"),
    "games":            ("Games & Entertainment", "Gamepad2",  "gamecontroller",                              "Games, game launchers, emulators"),
    "graphics":         ("Graphics & Design","Palette",        "paintpalette",                                "Image editing, vector design, 3D, screenshot, CAD"),
    "music":            ("Music",            "Music",          "music.note",                                  "Music players, streaming clients, DAWs"),
    "office":           ("Office & Docs",    "FileText",       "doc.text",                                    "Word processors, spreadsheets, presentations, PDF"),
    "productivity":     ("Productivity",     "Briefcase",      "briefcase",                                   "Note-taking, task management, calendars, launchers"),
    "security":         ("Security",         "Lock",           "lock",                                        "Password managers, VPNs, encryption, firewalls"),
    "system-utilities": ("System Utilities", "Settings",       "gearshape",                                   "Window managers, system monitors, menu-bar tools, cleaners"),
    "terminal":         ("Terminal",         "Terminal",       "terminal",                                    "Terminal emulators, shells, multiplexers"),
    "video-audio":      ("Video & Audio",    "Video",          "video",                                       "Video editors, codecs, screen recorders, audio converters"),
    "writing":          ("Writing",          "PenTool",        "pencil.tip",                                  "Long-form writing, markdown, dictation, journaling"),
    "uncategorized":    ("Uncategorized",    "HelpCircle",     "questionmark.circle",                         "Genuinely doesn't fit any other category"),
}

DEFAULT_BATCH_SIZE = 50
DEFAULT_MAX_CATEGORIES = 10  # sanity ceiling against pathological output; not a soft cap
HOMEBREW_API = "https://formulae.brew.sh/api"
USER_AGENT = "brew-browser-categorize/0.1 (+https://github.com/msitarzewski/brew-browser)"

# ─────────────────────────────────────────────────────────────────────────────
# Data types
# ─────────────────────────────────────────────────────────────────────────────
@dataclass
class Pkg:
    token: str
    kind: str  # "cask" | "formula"
    desc: str
    homepage: str = ""

    def desc_hash(self) -> str:
        return hashlib.sha256(self.desc.encode("utf-8")).hexdigest()[:16]

    def display_for_prompt(self) -> str:
        # Keep prompt cheap — token: desc, truncate desc.
        d = (self.desc or "").strip().replace("\n", " ")[:200]
        return f"{self.token}: {d}"


# ─────────────────────────────────────────────────────────────────────────────
# Fetch
# ─────────────────────────────────────────────────────────────────────────────
def fetch_all() -> list[Pkg]:
    pkgs: list[Pkg] = []
    for kind, path in (("cask", "cask.json"), ("formula", "formula.json")):
        log(f"  fetching {kind}.json …")
        r = requests.get(f"{HOMEBREW_API}/{path}", headers={"User-Agent": USER_AGENT}, timeout=30)
        r.raise_for_status()
        items = r.json()
        for item in items:
            token = item.get("token") or item.get("name") or ""
            if isinstance(item.get("name"), list):
                # cask 'name' is a list; token is the canonical id
                token = item.get("token", "")
            if not token:
                continue
            desc = (item.get("desc") or "").strip()
            homepage = (item.get("homepage") or "").strip()
            pkgs.append(Pkg(token=token, kind=kind, desc=desc, homepage=homepage))
        log(f"    → {sum(1 for p in pkgs if p.kind == kind)} {kind}s")
    return pkgs


# ─────────────────────────────────────────────────────────────────────────────
# Diff
# ─────────────────────────────────────────────────────────────────────────────
def load_prior_state() -> dict[str, str]:
    """token → desc_hash"""
    if not STATE_PATH.exists():
        return {}
    try:
        return json.loads(STATE_PATH.read_text())
    except Exception:
        log(f"  warning: state file unparseable, treating as fresh run: {STATE_PATH}")
        return {}


def write_state(pkgs: list[Pkg]) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    if STATE_PATH.exists():
        STATE_PATH.replace(STATE_PATH.with_suffix(".json.bak"))
    state = {p.token: p.desc_hash() for p in pkgs}
    STATE_PATH.write_text(json.dumps(state, indent=0, sort_keys=True))


def diff(pkgs: list[Pkg], prior: dict[str, str]) -> tuple[list[Pkg], list[str]]:
    """Return (to_categorize, removed_tokens)."""
    fresh_tokens = {p.token for p in pkgs}
    removed = sorted(set(prior.keys()) - fresh_tokens)
    to_categorize: list[Pkg] = []
    for p in pkgs:
        if p.token not in prior or prior[p.token] != p.desc_hash():
            to_categorize.append(p)
    return to_categorize, removed


# ─────────────────────────────────────────────────────────────────────────────
# LLM providers
# ─────────────────────────────────────────────────────────────────────────────
def build_system_prompt() -> str:
    cats_lines = []
    for slug, (label, _icon, _sf, hint) in CATEGORIES.items():
        cats_lines.append(f"- {slug}: {label} — {hint}")
    max_cats = int(os.environ.get("CATEGORIZE_MAX_CATEGORIES", DEFAULT_MAX_CATEGORIES))
    return PROMPT_SYSTEM.format(
        max_categories=max_cats,
        categories_yaml="\n".join(cats_lines),
    )


def build_user_prompt(batch: list[Pkg]) -> str:
    lines = ["Categorize each of these Homebrew packages:\n"]
    for i, p in enumerate(batch, 1):
        kind = "cask" if p.kind == "cask" else "formula"
        lines.append(f"{i}. [{kind}] {p.display_for_prompt()}")
    return "\n".join(lines)


def call_anthropic(system: str, user: str, model: str) -> str:
    import anthropic
    client = anthropic.Anthropic()
    resp = client.messages.create(
        model=model,
        max_tokens=4096,
        system=system,
        messages=[{"role": "user", "content": user}],
    )
    return "".join(block.text for block in resp.content if hasattr(block, "text"))


def call_openai(system: str, user: str, model: str) -> str:
    import openai
    client = openai.OpenAI()
    resp = client.chat.completions.create(
        model=model,
        messages=[
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        max_tokens=4096,
    )
    return resp.choices[0].message.content or ""


def pick_provider() -> tuple[str, str, callable]:
    """Return (provider_name, model, call_fn)."""
    if os.environ.get("ANTHROPIC_API_KEY"):
        model = os.environ.get("CATEGORIZE_MODEL", "claude-haiku-4-5")
        return ("anthropic", model, call_anthropic)
    if os.environ.get("OPENAI_API_KEY"):
        model = os.environ.get("CATEGORIZE_MODEL", "gpt-4o-mini")
        return ("openai", model, call_openai)
    raise SystemExit(
        "no LLM provider configured.\n"
        "set ANTHROPIC_API_KEY or OPENAI_API_KEY in .env (see .env.example)"
    )


# ─────────────────────────────────────────────────────────────────────────────
# Parsing + validation
# ─────────────────────────────────────────────────────────────────────────────
def parse_llm_response(raw: str, batch: list[Pkg]) -> dict[str, list[str]]:
    """Parse JSON-lines reply into {token: [cats...]}. Tolerant of extra text."""
    result: dict[str, list[str]] = {}
    valid_slugs = set(CATEGORIES.keys())
    for line in raw.splitlines():
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
            tok = obj.get("token")
            cats = obj.get("categories", [])
            if not isinstance(tok, str) or not isinstance(cats, list):
                continue
            # whitelist filter — drop anything the LLM hallucinated
            cats = [c for c in cats if c in valid_slugs]
            if not cats:
                cats = ["uncategorized"]
            # cap to max_categories
            cap = int(os.environ.get("CATEGORIZE_MAX_CATEGORIES", DEFAULT_MAX_CATEGORIES))
            result[tok] = cats[:cap]
        except json.JSONDecodeError:
            continue
    return result


# ─────────────────────────────────────────────────────────────────────────────
# Batching
# ─────────────────────────────────────────────────────────────────────────────
def chunked(seq: list[Pkg], n: int) -> Iterable[list[Pkg]]:
    for i in range(0, len(seq), n):
        yield seq[i : i + n]


# ─────────────────────────────────────────────────────────────────────────────
# Output assembly
# ─────────────────────────────────────────────────────────────────────────────
def load_existing_output() -> dict:
    if not OUTPUT_PATH.exists():
        return {"version": "", "generated_at": "", "model": "",
                "categories": {}, "casks": {}, "formulae": {}}
    try:
        return json.loads(OUTPUT_PATH.read_text())
    except Exception:
        return {"version": "", "generated_at": "", "model": "",
                "categories": {}, "casks": {}, "formulae": {}}


def write_output(
    existing: dict,
    new_categories: dict[str, list[str]],
    pkgs_by_token: dict[str, Pkg],
    removed: list[str],
    model: str,
) -> dict:
    # Start from existing (carry forward unchanged tokens) and apply deltas.
    casks = dict(existing.get("casks", {}))
    formulae = dict(existing.get("formulae", {}))

    for token in removed:
        casks.pop(token, None)
        formulae.pop(token, None)

    for token, cats in new_categories.items():
        p = pkgs_by_token.get(token)
        if not p:
            continue
        target = casks if p.kind == "cask" else formulae
        target[token] = cats

    cat_meta = {
        slug: {"label": label, "icon": icon, "iconSF": sf}
        for slug, (label, icon, sf, _hint) in CATEGORIES.items()
    }

    output = {
        "version": time.strftime("%Y-%m-%d"),
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "model": model,
        "categories": cat_meta,
        "casks": dict(sorted(casks.items())),
        "formulae": dict(sorted(formulae.items())),
    }
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_PATH.write_text(json.dumps(output, indent=2))
    return output


# ─────────────────────────────────────────────────────────────────────────────
# Logging
# ─────────────────────────────────────────────────────────────────────────────
def log(msg: str) -> None:
    ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    line = f"[{ts}] {msg}"
    print(line)
    try:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        with LOG_PATH.open("a") as f:
            f.write(line + "\n")
    except Exception:
        pass


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────
def main() -> int:
    load_dotenv(SCRIPT_DIR / ".env")

    dry_run = os.environ.get("CATEGORIZE_DRY_RUN", "0") == "1"
    limit = int(os.environ.get("CATEGORIZE_LIMIT", "0"))
    batch_size = int(os.environ.get("CATEGORIZE_BATCH_SIZE", DEFAULT_BATCH_SIZE))

    log("=== brew-browser categorize ===")
    log(f"output: {OUTPUT_PATH}")

    log("fetching Homebrew published data…")
    pkgs = fetch_all()
    log(f"  total: {len(pkgs)} packages")

    prior = load_prior_state()
    log(f"prior state: {len(prior)} tokens")

    to_categorize, removed = diff(pkgs, prior)
    log(f"diff: {len(to_categorize)} new/changed, {len(removed)} removed")

    if limit > 0:
        to_categorize = to_categorize[:limit]
        log(f"  (limited to {limit} for this run)")

    if dry_run:
        log("DRY RUN — no LLM calls, no output written")
        for p in to_categorize[:20]:
            log(f"  would categorize: [{p.kind}] {p.token}")
        if len(to_categorize) > 20:
            log(f"  … and {len(to_categorize) - 20} more")
        return 0

    if not to_categorize and not removed:
        log("no work to do — exiting clean")
        return 0

    provider_name, model, call_fn = pick_provider()
    log(f"provider: {provider_name} model={model} batch_size={batch_size}")

    system = build_system_prompt()
    pkgs_by_token = {p.token: p for p in pkgs}
    new_categories: dict[str, list[str]] = {}

    batches = list(chunked(to_categorize, batch_size))
    for i, batch in enumerate(batches, 1):
        log(f"  batch {i}/{len(batches)} — {len(batch)} items")
        user = build_user_prompt(batch)
        try:
            raw = call_fn(system, user, model)
        except Exception as e:
            log(f"  WARN: batch {i} failed: {type(e).__name__}: {e}")
            log(f"  WARN: skipping batch; rerun will retry")
            continue
        parsed = parse_llm_response(raw, batch)
        log(f"    parsed {len(parsed)}/{len(batch)} items")
        new_categories.update(parsed)

    log(f"categorized {len(new_categories)} items total")

    if not new_categories and not removed:
        log("no progress this run — leaving state untouched so next run retries")
        return 1  # non-zero exit so cron can alert

    existing = load_existing_output()
    out = write_output(existing, new_categories, pkgs_by_token, removed, model)
    log(f"wrote {OUTPUT_PATH} — {len(out['casks'])} casks, {len(out['formulae'])} formulae")

    # Only record state for tokens that ended up in the output. Tokens that
    # failed this run stay out of state so the next run picks them up again.
    out_tokens = set(out["casks"].keys()) | set(out["formulae"].keys())
    successful_pkgs = [p for p in pkgs if p.token in out_tokens]
    write_state(successful_pkgs)
    log(f"state updated → {STATE_PATH}  ({len(successful_pkgs)} tokens recorded)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
