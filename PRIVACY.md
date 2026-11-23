# Orbit Privacy

Orbit is local-first. Your source code, projects, terminal, AI conversations,
and usage history stay on your machine.

**Orbit collects no telemetry.** There is no analytics client, no installation
or session identifier, no usage reporting, and no third-party tracking SDK in
the app. Nothing about how you use Orbit is sent anywhere.

## What leaves your machine

Orbit talks to the network only for things you asked for, plus the images in
the Markdown you open (below):

* **Update checks** — the app fetches its own signed release feed to see whether
  a newer version exists, and to download it if you accept.
* **GitHub metadata** — author avatars and repository data shown in the Git and
  GitHub panels.
* **New issues and pull requests** — while the app is open, every 5 minutes
  and when its window comes forward, Orbit asks GitHub through your signed-in
  `gh` for the current repository's open issues and pull requests (when each
  was opened and by whom), to mark the sidebar's Git button when any are new
  since you last looked. Without `gh`, a sign-in or a GitHub remote nothing is
  asked. Turn it off in Settings → Privacy → GitHub.
* **Starring Orbit** — the first time Settings → About opens in a launch, Orbit
  asks GitHub through your signed-in `gh` whether you have starred its
  repository; it stars or unstars only when you click the button.
* **Model providers** — prompts, context, and tool output are sent to whichever
  provider your `pi` agent is configured to use, exactly as the `pi` CLI does on
  its own. That traffic is governed by the provider's policy, not Orbit's.

* **Images in Markdown** — an image in a GitHub issue or pull request body,
  or in an agent's reply, loads inline as soon as it is on screen. Its host
  sees an ordinary image request from your IP address, as a browser showing the
  same page would; a tracking pixel in that Markdown would see it too. Images
  load over HTTPS only (a plain `http://` image is not fetched), with a 5 s
  connect and 15 s total timeout, at most 5 redirects and an 8 MiB response cap.
  Local `file://` and `data:` images are never loaded. An image that cannot
  load shows its alt text and an **Open** action that hands the HTTPS URL to
  your browser. A host that resolves to a private, loopback, or link-local
  address is refused, so an image in that Markdown cannot reach your own
  machine or your LAN. Exported PDFs still leave remote images out.

GitHub avatar downloads use the same HTTPS client and limits.

None of these carry a telemetry identifier, and Orbit adds no tracking data to
them.

## What Orbit stores locally

Orbit keeps its own settings and state under `~/.orbit-pi/`, and reads session
data that `pi` writes. This never leaves your machine except through the
provider calls above:

* workspace list, marks, and sort preferences;
* explicit workspace project-trust decisions (`project-trust.json`), including
  canonical local folders and remote account+directory identities;
* theme, appearance, and interface preferences, including your favorite
  and recently used models;
* window, terminal, and layout state;
* session-retention policy;
* the live event log of each delegated subagent (`subagent-logs/`, readable
  only by you), so its tab can show what it did; logs older than a week are
  removed when the next delegation starts.

There is no `analytics.json`, no pseudonymous identifier, and no queue of
events.

## Privacy controls

**Settings → Privacy** holds the session-retention control. It lets you
archive or delete old `pi` sessions on a schedule you choose, entirely on your
machine. Orbit never uploads anything from that store.

## Source of truth

The absence of telemetry is enforced structurally: there is no
`crates/orbit-analytics` crate and no `crate::analytics` module. If a
telemetry path is ever reconsidered, this document must change first.
