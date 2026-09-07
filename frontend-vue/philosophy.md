# infoinserver — UI Design Philosophy

> This document is the design constitution for the infoinserver frontend.
> Every UI decision must be evaluated against the principles here.
> "What will make this stand out to a user?"

---

## 1. Identity Statement

**infoinserver is a pro-grade server management dashboard for engineers who live in the terminal.**

It is NOT:
- A generic SaaS product dashboard
- A Bootstrap admin template
- An AI-generated CRUD interface

It IS:
- A focused tool built by someone who runs servers
- An interface that respects the user's technical literacy
- A dashboard that feels like it belongs next to your terminal, not your CRM

**Tone:** Precise. Technical. Confident. Dark-first. No decorative fluff — only purposeful detail.

---

## 2. The Anti-Slop Checklist

Before shipping any UI, verify it does NOT contain:

- [ ] `Inter` or `Arial` as the primary display font
- [ ] Purple gradients as a decorative motif
- [ ] All cards with the same `shadow-sm` regardless of context
- [ ] `prompt()` or `confirm()` calls breaking the design system
- [ ] Identical loading spinners across different contexts
- [ ] Hardcoded hex values that should be design tokens (e.g. `bg-[#0f111a]`)
- [ ] Duplicate button class aliases (`btn-outline` ≡ `btn-secondary` is a red flag)
- [ ] Same transition on every element regardless of purpose
- [ ] Zero page-level entrance animation — abrupt view changes
- [ ] Mixed-language UI strings (pick English OR Indonesian, not both)
- [ ] `border-blue-500` instead of `border-brand-500` for active states
- [ ] Icon-only buttons with no accessible `title` or `aria-label`
- [ ] "wall of content" list renders with no staggered reveal

---

## 3. Font System — Geist Mono

**Primary font: Geist Mono** (imported from CDN)

Rationale: A server dashboard deals in IDs, ports, PIDs, paths, hostnames, commands, and timestamps. Monospace IS the language of this domain. Geist Mono is modern, readable at small sizes, and distinctive without being quirky.

### Type Scale

| Role              | Size          | Weight      | Letter Spacing | Usage                              |
|-------------------|---------------|-------------|----------------|------------------------------------|
| Display           | `text-2xl`    | `font-bold` | `-0.03em`      | Page headings, count stats         |
| Title             | `text-lg`     | `font-bold` | `-0.02em`      | Card titles, server names          |
| Subtitle          | `text-base`   | `font-medium`| `-0.01em`     | Section headers                    |
| Body              | `text-sm`     | `font-normal`| `0em`         | Table cells, descriptions          |
| Label             | `text-xs`     | `font-semibold`| `0.05em`    | Column headers (UPPERCASE), badges |
| Caption           | `text-[11px]` | `font-normal`| `0.01em`      | Timestamps, metadata               |
| Micro             | `text-[10px]` | `font-medium`| `0.02em`      | Port numbers, UIDs, hex values     |

### Rules
- Column headers (`table-th`): UPPERCASE + `tracking-[0.05em]` — communicates "category, not content"
- Heading sizes: tight letter-spacing pulls letters together, feels confident and dense
- Badge text: open letter-spacing signals "label, not data"
- All numeric data (ports, PIDs, CPU%, memory, timestamps): displayed in Geist Mono naturally — no special wrapping needed since the base font is already mono
- Avoid mixing `font-bold` and `font-semibold` arbitrarily — use the scale above

---

## 4. Color System

### Primary Accent: Cyan

**Brand cyan replaces generic blue.**

```
--color-brand-50:  #ecfeff   (very light cyan)
--color-brand-100: #cffafe
--color-brand-200: #a5f3fc
--color-brand-300: #67e8f9
--color-brand-400: #22d3ee
--color-brand-500: #06b6d4   ← primary accent (Tailwind cyan-500)
--color-brand-600: #0891b2   ← active, focus states
--color-brand-700: #0e7490
```

Rationale: Cyan reads as "digital", "networked", "alive". It contrasts sharply in dark mode without feeling aggressive. It differentiates infoinserver from 90% of SaaS dashboards that default to blue.

### Semantic Colors (preserved from existing system)

| Domain            | Color   | Token         | Meaning                          |
|-------------------|---------|---------------|----------------------------------|
| Primary accent    | Cyan    | `brand-*`     | Interactive, selected, links     |
| Success / running | Green   | `green-*`     | Active services, healthy status  |
| Warning / pending | Amber   | `amber-*`     | Updates needed, moderate risk    |
| Error / danger    | Red     | `red-*`       | Failed, destructive, banned      |
| Memory / RAM      | Purple  | `purple-*`    | RAM metrics only (DO NOT reuse)  |
| Storage / disk    | Amber   | `amber-*`     | Disk metrics (shared with warn)  |
| Network RX        | Emerald | `emerald-*`   | Download, inbound                |
| Network TX        | Red     | `red-*`       | Upload, outbound                 |
| Cloudflare / DNS  | Orange  | `orange-*`    | Cloudflare-specific accent only  |

### Terminal Color Palette

Used inside any log/terminal area (`bg-terminal`, xterm, log panels):

```
--color-terminal-bg:      #0d1117   (darker than slate-950, GitHub dark-inspired)
--color-terminal-surface: #161b22
--color-terminal-text:    #e6edf3
--color-terminal-muted:   #8b949e
--color-terminal-green:   #3fb950   (success lines, bash commands)
--color-terminal-cyan:    #79c0ff   (service names, info)
--color-terminal-amber:   #d29922   (warning lines)
--color-terminal-red:     #f85149   (error lines, ban events)
--color-terminal-purple:  #bc8cff   (special highlight)
```

### Page Background

NOT flat slate. Use a very subtle atmospheric layer:

- **Dark mode:** `bg-slate-950` base + `radial-gradient(ellipse at top, rgba(6,182,212,0.04) 0%, transparent 60%)` — a barely-visible cyan glow at the top of the viewport. Not decorative noise, just depth.
- **Light mode:** `bg-slate-100` base + `radial-gradient(ellipse at top, rgba(6,182,212,0.06) 0%, transparent 50%)` — same but lighter.

This creates the sensation that the interface has a light source, not a flat ceiling.

---

## 5. Shadow Hierarchy

Shadows communicate depth and interactivity. Every element exists at a specific "altitude":

| Level | Token            | Value                                       | Usage                              |
|-------|------------------|---------------------------------------------|------------------------------------|
| 0     | `shadow-none`    | none                                        | Inline elements, table rows        |
| 1     | `shadow-card`    | `0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06)` | Resting cards (.card)  |
| 2     | `shadow-card-hover` | `0 4px 16px rgba(0,0,0,0.12), 0 2px 6px rgba(0,0,0,0.08)` | Hovered interactive cards |
| 3     | `shadow-dropdown`| `0 8px 24px rgba(0,0,0,0.18)`              | Dropdowns, tooltips, popovers      |
| 4     | `shadow-modal`   | `0 24px 64px rgba(0,0,0,0.32)`             | Modals, overlays                   |

### Rules
- Resting state is always lower altitude than interaction state
- Modals must use the highest shadow — they float above everything
- Never give two adjacent sibling elements the same shadow if one should feel "above" the other
- Dark mode: increase shadow opacity slightly (dark surfaces absorb less light visually)

---

## 6. Atmospheric Backgrounds

Flat `bg-slate-800` cards are boring. Atmospheric backgrounds create the illusion of a live, dimensional interface.

### Techniques (in order of subtlety — use sparingly)

1. **Radial glow origin** — `background: radial-gradient(ellipse at top left, rgba(6,182,212,0.06) 0%, transparent 60%)` on the page shell. One per page, maximum.

2. **Surface tints** — Domain-colored card backgrounds with very low opacity:
   - CPU panel: `bg-cyan-500/5 dark:bg-cyan-500/8`
   - Memory panel: `bg-purple-500/5 dark:bg-purple-500/8`
   - Disk panel: `bg-amber-500/5 dark:bg-amber-500/8`
   - Uptime panel: `bg-green-500/5 dark:bg-green-500/8`

3. **Subtle inner border glow** — Cards that represent "live" data get a hairline inner border:
   `box-shadow: inset 0 0 0 1px rgba(6,182,212,0.1)` — barely visible cyan rim.

4. **Terminal panels** — Use `--color-terminal-bg` consistently. Never `bg-black`, `bg-[#0f111a]`, or other one-off values.

5. **Login page** — Already distinctive. Keep the layered system (bg.jpg + blur + radial gradients + dot grid). Add a subtle card entrance animation.

### Rules
- Maximum ONE gradient per view (the page-level ambient glow doesn't count)
- No decorative gradients on buttons
- Gradient direction should always feel like "light from above" — top-to-bottom or top-left-to-bottom-right

---

## 7. Motion — Quality Over Quantity

**Animation must justify its existence. If you cannot articulate WHY something moves, it should not move.**

### Approved Animations

| Animation               | Purpose                                        | Implementation                                   |
|-------------------------|------------------------------------------------|--------------------------------------------------|
| Card entrance           | "Content just arrived"                         | `fade-in + slide-up-4` on mount, `duration-300` |
| Staggered list reveal   | Prevents "wall of content" effect              | `animation-delay: calc(var(--i) * 50ms)`         |
| Sonar pulse (live dots) | "This is a live, real-time indicator"          | `animate-ping` — keep as-is, already great       |
| Button press            | Tactile feedback                               | `active:scale-[0.97]` — keep as-is              |
| Icon button press       | Tactile feedback (compact)                     | `active:scale-90` — keep as-is                  |
| Hover lift (cards)      | "This is interactive / clickable"              | `hover:shadow-card-hover transition-shadow`      |
| Progress bar fill       | "Value changed"                                | `transition-all duration-300`                    |
| Chevron rotation        | "This controls expand/collapse"                | `rotate-180 transition-transform duration-150`   |
| Loading spinner         | "Working on it"                                | `animate-spin` on `<Loader2>` — standardize      |
| View transition         | "Navigating between tabs"                      | Vue `<Transition>` with `fade` or `slide-fade`  |

### Prohibited Animations

- Bounce, elastic, or spring effects — not fitting for a pro tool
- Looping decorative animations (except sonar pulse on live indicators)
- Simultaneous animations on multiple unrelated elements
- Transitions longer than `400ms` for any UI element
- `animate-bounce` — never, under any circumstances

### Standardize Loading States

Always use `<Loader2 class="w-4 h-4 animate-spin" />` from lucide-vue-next.
Never use the manual CSS spinner (`animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600`).

---

## 8. Component Standards

### Cards

```
.card {
  background: bg-white dark:bg-slate-800/90
  border: border border-slate-200 dark:border-slate-700
  border-radius: rounded-xl
  padding: p-6
  shadow: shadow-card  ← use new token, not shadow-sm
  transition: transition-shadow duration-200
}

.card-interactive {
  extends: .card
  cursor: cursor-pointer
  hover: hover:shadow-card-hover
}
```

Cards that display live data get a cyan hairline inner glow:
```
box-shadow: inset 0 0 0 1px rgba(6,182,212,0.08)
```

### Buttons

5 semantic variants — no aliases:
- `btn-primary` — cyan bg, white text (replaces blue)
- `btn-secondary` — slate neutral (REMOVE `btn-outline` alias)
- `btn-danger` — red bg (REMOVE `btn-destructive` alias)
- `btn-warning` — amber bg
- `btn-success` — green bg

All buttons: `active:scale-[0.97] transition-all duration-150`

### Tables

`.table-th` — UPPERCASE, `tracking-[0.05em]`, `text-[11px]`, `font-semibold`
`.table-td` — `text-sm`, appropriate padding

### Modals

All modals follow this structure:
```
<Teleport to="body">
  <!-- Overlay -->
  <div class="fixed inset-0 backdrop-blur-sm bg-slate-900/50 z-[100]">
    <!-- Modal -->
    <div class="rounded-xl shadow-modal overflow-hidden">
      <!-- Header: always dark (bg-slate-900 dark:bg-slate-900) -->
      <!-- Body: bg-white dark:bg-slate-800 -->
      <!-- Footer (if needed): bg-slate-50 dark:bg-slate-800/50 border-t -->
    </div>
  </div>
</Teleport>
```

**Never use `prompt()` or `confirm()`** — always build a proper modal.

### Status Badges

```
.badge-success  → bg-green-100/10 text-green-700 dark:text-green-400 border border-green-200/20
.badge-warning  → bg-amber-100/10 text-amber-700 dark:text-amber-400 border border-amber-200/20
.badge-danger   → bg-red-100/10 text-red-700 dark:text-red-400 border border-red-200/20
.badge-info     → bg-cyan-100/10 text-cyan-700 dark:text-cyan-400 border border-cyan-200/20
.badge-neutral  → bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300
```

---

## 9. Design Tokens (CSS Variables)

All one-off values must become tokens. No exceptions.

```css
/* Terminal */
--color-terminal-bg:      #0d1117;
--color-terminal-surface: #161b22;
--color-terminal-text:    #e6edf3;
--color-terminal-muted:   #8b949e;
--color-terminal-green:   #3fb950;
--color-terminal-cyan:    #79c0ff;
--color-terminal-amber:   #d29922;
--color-terminal-red:     #f85149;

/* Shadows */
--shadow-card:       0 1px 3px rgba(0,0,0,0.08), 0 1px 2px rgba(0,0,0,0.06);
--shadow-card-hover: 0 4px 16px rgba(0,0,0,0.12), 0 2px 6px rgba(0,0,0,0.08);
--shadow-dropdown:   0 8px 24px rgba(0,0,0,0.18);
--shadow-modal:      0 24px 64px rgba(0,0,0,0.32);

/* Ambient glow (page background) */
--gradient-ambient-dark:  radial-gradient(ellipse at top, rgba(6,182,212,0.04) 0%, transparent 60%);
--gradient-ambient-light: radial-gradient(ellipse at top, rgba(6,182,212,0.06) 0%, transparent 50%);
```

---

## 10. Elements to Preserve (Do Not Redesign)

These elements are already distinctive and should be kept, potentially enhanced:

| Element                          | Why It's Good                                                   |
|----------------------------------|-----------------------------------------------------------------|
| Login layered background         | Most polished visual in the app — bg.jpg + blur + radial + dots |
| Live ping latency indicator      | Real-time color-coded latency (emerald/amber/orange/red) badge  |
| Sonar `animate-ping` on live dots| Immediately communicates "live" — containers, Fail2Ban, logs    |
| Five-color cron field system     | Genuinely intuitive — each cron position has a semantic color   |
| Distro icon system               | Per-OS color backgrounds + SimpleIcons SVG — personal and smart |
| FilesView emoji file type icons  | `📁🐍🟨💚🦀` — charming, functional, human                      |
| Dashboard 4-color resource panels| green/blue/purple/amber resource identity system               |
| GitHub presence avatar stack     | Multi-user awareness feature unique to this app                 |
| Fail2Ban terminal log coloring   | Red=Ban, Emerald=Unban, Blue=Found — semantically correct       |
| Cloudflare setup wizard stepper  | Per-step accent colors, completion states                       |

---

## 11. Language

**All UI strings must be in English.** No exceptions.

Indonesian strings found in the codebase that must be updated:
- `UsersView`: "Konfirmasi", "Hapus User", "Apakah Anda yakin...", "Shell/Binary"
- `LogsView`: "Log ini dicatat secara otomatis oleh background scheduler..."
- Any other `confirm()` / `alert()` / `prompt()` message text

---

## 12. Implementation Order

When applying this philosophy to existing views, follow this sequence:

1. **Foundation first** — `main.css` tokens and font before touching any view
2. **Shell second** — `App.vue` and `ServerLayout.vue` set the frame for everything
3. **Most-used views** — Login, Home, Dashboard see the most traffic
4. **Feature views** — Services, Files, Containers, Users, Ports, etc.
5. **Utility views** — Updates, Syslogs, Cron, Cloudflare, Logs, Settings last

Each view change is self-contained. A view should not depend on another view being updated first.

---

*Last updated: 2026*
*Applies to: frontend-vue/src — all views, components, and stylesheets*
