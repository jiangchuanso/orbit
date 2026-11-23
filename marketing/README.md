# Orbit — marketing site

The landing page for Orbit, the native GPUI workbench for the pi coding agent.
Built with Next.js (App Router), Tailwind CSS v4, and Motion.

## Develop

```bash
npm install
npm run dev      # http://localhost:3000
```

## Build

```bash
npm run build
npm run start
```

## Structure

```
src/app/layout.tsx          fonts (Inter + Geist Mono) + metadata
src/app/globals.css         reference palette + shell/band/dash utilities
src/app/page.tsx            section order
src/app/changelog/page.tsx  release feed (repo CHANGELOG.md + GitHub releases)
src/app/changelog/[version]/  per-release page + its Open Graph image
src/components/             nav, footer, ui primitives, screenshot frame
src/components/download-button.tsx  OS-aware download CTA + options menu
src/components/sections/    hero, stage, steps, workbench, features,
                            guard, review, gallery, closer
src/components/changelog/   release entries, inline markdown, skeleton
src/components/ui/          shadcn primitives (button, card, badge,
                            skeleton, empty)
src/lib/changelog.ts        changelog + release parsing and formatting
src/lib/downloads.ts        release assets → per-platform download lists
public/screens/             real app screenshots (window-only)
```

The copy describes the **app** — its features (chat, tools, sessions, find,
providers, plugins, models, usage, appearance) and the steps you take in it
(connect a provider → start a task → watch it work → review → commit), not the
repository's engineering internals.

Screenshots in `public/screens/` are the real app, captured window-only
(rounded corners and shadow, no desktop or menu bar). They are `next/image`-
optimized at runtime; the hero shot is `priority`, the rest lazy-load.

### Refreshing screenshots

`scripts/capture-shot.swift` captures the frontmost Orbit window straight
into `public/screens/`. It resizes the window to 1470×922 pt and keeps the
shadow, so every full shot comes out at 3164×2068 like the rest and
`tour.tsx` needs no size change. Run it from your own terminal, which needs
**Screen Recording** permission (and **Accessibility** for the resize):

```bash
swift scripts/capture-shot.swift --list            # which Orbit window it sees
swift scripts/capture-shot.swift new-task          # dark  → new-task.png
swift scripts/capture-shot.swift new-task --light  # light → new-task-light.png
```

Set the theme in Orbit (Settings → Appearance) before each run; the script
does not change it. Stage demo content first: the sidebar's session titles
end up on the public site.

| Shot | Screen in Orbit |
|---|---|
| `new-task` | ⌘N, the new-task page with a workspace picked |
| `review` | a session with the Review pane open on a diff |
| `models` | Settings → Models |
| `skills` | Settings → Skills, one skill selected |
| `mcp` | Settings → MCP |
| `plugins` | Settings → Plugins |
| `usage` | the Usage page |
| `appearance` | Settings → Appearance |
| `session`, `git` | a live session; the Git page (SEO metadata and the OG image) |
| `sessions-detail` | crop: the sidebar, 250×575 pt from the window's top-left |
| `agent-detail` | crop: 600×450 pt of a transcript; `--crop X,Y` takes its top-left in screen points from the ⌘⇧4 readout |

The visual system mirrors the warm.run landing: a 1280px `shell` with hairline
rules down each edge, dashed separators between `band`s, a dark slate page
(`#141518`) with recessed well bands, `#1c1d21` cards, a single blue micro-accent
(`#5b9dff`), and Inter at light weights (250/300/350) with Geist Mono for labels.

Dark is the default. Every colour resolves through CSS variables defined in
`:root` (dark) and `.light`, so a nav toggle can flip the whole site; the
choice is stored in `localStorage` and applied before paint by an inline
script in the root layout.
