/**
 * Lucide icon-name → Svelte component binding for the category icons.
 *
 * The icon CHOICE is data-driven: it lives once in
 * `tools/categorize/categorize.py` (the `CATEGORIES` taxonomy) and is emitted
 * to `src-tauri/data/categories.json` as `icon` (this Lucide name, for the web
 * UI) and `iconSF` (an SF Symbol name, consumed directly by the native macOS
 * UI). Neither UI re-decides icons in code.
 *
 * This map is NOT a second source of truth — it's purely the bundler binding
 * from the data-provided Lucide name to its tree-shakeable Svelte component.
 * Lucide-svelte has no render-by-name without bundling all ~1600 icons, so the
 * 19 components we use must be statically imported here. (Native needs no such
 * map: SF Symbols render from a plain string, so it reads `iconSF` directly.)
 *
 * Adding a category in `categorize.py`? Pick its Lucide + SF names there; then
 * add the one matching Lucide import below. Unknown names fall back to
 * `HelpCircle` so a missing import never crashes — but it WILL look out of
 * place, so keep this in sync with the taxonomy.
 */

import type { Component } from "svelte";

import Brain from "@lucide/svelte/icons/brain";
import Briefcase from "@lucide/svelte/icons/briefcase";
import Cloud from "@lucide/svelte/icons/cloud";
import Code from "@lucide/svelte/icons/code";
import Database from "@lucide/svelte/icons/database";
import FileCode from "@lucide/svelte/icons/file-code";
import FileText from "@lucide/svelte/icons/file-text";
import Gamepad2 from "@lucide/svelte/icons/gamepad-2";
import Globe from "@lucide/svelte/icons/globe";
import GraduationCap from "@lucide/svelte/icons/graduation-cap";
import HelpCircle from "@lucide/svelte/icons/help-circle";
import Lock from "@lucide/svelte/icons/lock";
import MessageSquare from "@lucide/svelte/icons/message-square";
import Music from "@lucide/svelte/icons/music";
import Palette from "@lucide/svelte/icons/palette";
import PenTool from "@lucide/svelte/icons/pen-tool";
import Settings from "@lucide/svelte/icons/settings";
import Terminal from "@lucide/svelte/icons/terminal";
import Video from "@lucide/svelte/icons/video";

const ICONS: Record<string, Component> = {
  Brain,
  Briefcase,
  Cloud,
  Code,
  Database,
  FileCode,
  FileText,
  Gamepad2,
  Globe,
  GraduationCap,
  HelpCircle,
  Lock,
  MessageSquare,
  Music,
  Palette,
  PenTool,
  Settings,
  Terminal,
  Video,
};

/**
 * Resolve a Lucide icon by PascalCase name. Falls back to `HelpCircle` for
 * any unknown name — see module docstring for why that fallback exists.
 */
export function resolveCategoryIcon(name: string): Component {
  return ICONS[name] ?? HelpCircle;
}
