# Roadmap — folio

## North star

Become the PDF app a specific niche **genuinely prefers and recommends** for
everyday tasks — because it is **free for everyone forever, provably offline,
and dead simple**. 

**Winning is NOT** feature parity with Stirling-PDF or Adobe. We will never have
50 tools, and trying to is how we lose. Winning is being the obvious choice for
people who want simple + free + private and are repelled by license tiers,
telemetry, and cloud sign-ins.

**Positioning line:** *"Free for everyone — including your company. No user
limit, no proprietary core, no telemetry, no cloud. Open source you can verify,
and it physically cannot phone home."*

## Where we are today (v1.0.0 — feature-complete and packaged)

- Rust workspace: `pdf-core` (engine), `pdf-cli` (`pdfx`), `pdf-gui` (Tauri).
- **Every Phase 1 feature is done**, including the two hard ones: true
  redaction (content-stream rewriting) and offline OCR (bundled Tesseract).
- Batch processing, drag-and-drop, progress bars, remembered folders and an
  in-app About panel — the Phase 2 UX list.
- Offline guarantee **enforced** by `cargo-deny` (socket-primitive tripwire) +
  CI, and now a second time by the Store package declaring no network
  capabilities.
- GPL-3.0 licensed. Branded ("folio"). Polished light-theme GUI.
- Shippable artifacts: `.msi`, NSIS installer, and a Microsoft Store `.msix`
  built by `msix/build-msix.ps1`.

---

## Phase 1 — Daily-driver feature floor

*Goal: stop users bouncing for a missing common operation. ~5 features, not 45.*

| Feature | Why | Native dep | Effort |
|---|---|---|---|
| ~~**Compress**~~ ✅ | Done — qpdf lossless repack + image optimization (light/balanced/strong). Safe: never inflates or corrupts. | qpdf (bundled) | done |
| ~~**Password / encrypt**~~ ✅ | Done — Protect (open + owner password, 256/128-bit AES, print/copy/edit permissions) and Remove password, via qpdf. | qpdf | done |
| ~~**Reorder / delete pages**~~ ✅ | Done — visual thumbnail grid (PDFium), drag to reorder + ✕ to delete; inheritance-safe page-tree rewrite (lopdf). CLI `pages --order`. | lopdf + PDFium thumbs | done |
| ~~**OCR (scanned → searchable)**~~ ✅ | Done — bundled Tesseract, rasterize via PDFium, per-page progress, any `tessdata` language. Refuses to wreck a born-digital PDF unless forced. | Tesseract (bundled) | done |
| ~~**Redact (true removal)**~~ ✅ | Done — content-stream rewriting deletes glyphs, images and annotations inside the region, then covers it. Tested by reading the output back. | lopdf + PDFium (region UI) | done |

We are already **ahead** on privacy-specific basics: metadata scrub is done and
treated as a headline (Stirling treats it as a minor tool).

**Phase 1 is complete.** The feature floor is met; the job now is
distribution, not features. Guard the ~12-tool cap from here (see Anti-goals).

## Phase 2 — Trust & polish (what converts the niche)

- [x] **App icon set** — generated reproducibly from geometry by
  `assets/make-icons.py`, wired into `bundle.icon` and the Store tiles.
- [x] **Real installer** — Tauri bundler produces `.msi` and NSIS, with
  PDFium, qpdf and Tesseract bundled via `bundle.resources`.
- [x] **UX:** drag-and-drop, progress for OCR/batch, remembered folders,
  "Open with" file association.
- [x] **Make the trust story visible** — in-app About panel, `PRIVACY.md`, and
  a README section on exactly how to verify the offline claim.
- [x] **Microsoft Store package** — MSIX pipeline under `msix/`. This is the
  cheap answer to code signing: the Store signs the package, so SmartScreen
  stops warning, for a one-time $19 instead of $100-400/yr.
- [ ] **Code signing** for the *direct-download* installers. Now lower
  priority: Store users are already covered.
- [ ] **Cross-platform builds** (macOS `.dmg`, Linux `.AppImage`/`.deb`) — each
  needs its own PDFium/qpdf/tesseract binaries.

## Phase 3 — Distribution (the real war)

*For solo OSS, getting found matters more than any feature.*

- Public **GitHub repo**: clean README, screenshots/GIF, the Stirling-honest
  comparison, the offline guarantee front and center.
- **One-page site** (GitHub Pages): headline, 3 proof bullets, download button.
- **Microsoft Store** — where non-technical Windows users actually look for
  software, and the only route that removes the SmartScreen warning for free.
  See `msix/SUBMISSION.md`.
- **Package managers:** winget + scoop first (Windows), then brew/AUR.
- **Launch posts** where the niche lives: Hacker News (Show HN), r/privacy,
  r/opensource, r/selfhosted, r/degoogle, Lemmy. Lead with values, not features.

## Phase 4 — Sustain & grow

- Triage issues; ship a changelog; respond fast (community goodwill is the
  growth engine for free tools).
- Optional **GitHub Sponsors / "buy me a coffee"** to cover the code-signing
  cert — never a paywall. (Sustainability stance still TBD: passion / donations
  / core-free-forever-with-enterprise-extras.)
- Add features **only** from real user demand, guarding simplicity.

---

## Costs (you can ship the whole thing for $0)

Everything to **build and distribute** is free: Rust, all crates, the bundled
engines (PDFium/qpdf/Tesseract, all permissive), GitHub repo + Pages, and
publishing to winget/scoop/brew/AUR. The only optional paid items remove
security warnings — defer them until there are users (and maybe donations):

| Item | Cost | Skip it? |
|---|---|---|
| Microsoft Store (individual account) | **$19 one-time** | No — this is the one to pay. It signs the package for you |
| Windows code signing | ~$10/mo (Azure Trusted Signing) or ~$100–400/yr | Yes — Store users are covered; direct downloads say "Run anyway" |
| Apple Developer (mac notarization) | $99/yr | Yes — Windows-first needs none |
| Custom domain | ~$12/yr | Yes — free `*.github.io` works |

Nothing is strictly required. **First dollar worth spending: the $19 Partner
Center account** — it buys a signed, warning-free install for the exact
non-technical audience folio is for, which a code-signing certificate would
otherwise cost $100–400/yr to achieve.

## Anti-goals (protecting the moat)

These are as important as the features. Breaking one loses the niche:

- **No telemetry. Ever.** Not even opt-in. The zero-network guarantee is sacred.
- **No accounts, no cloud, no sign-in.**
- **No paywall / no user limit.** Free for everyone, every org, forever.
- **No feature bloat.** Cap the surface at ~12–15 well-chosen tools. "Does 12
  things flawlessly" beats "does 50 things." Saying no is the strategy.
- **No closed/proprietary parts.** Fully GPL, fully auditable.

## Success metrics (honest, modest)

Not "beat Stirling." Realistic signals it's working:
- People in privacy/OSS communities **recommend it unprompted**.
- Steady GitHub stars + package-manager installs (hundreds → thousands).
- Issues filed by real users (means people use it).
- It's the tool **you** reach for. (Already true — that's the floor, not a loss.)

## Risks (named honestly)

- **Discoverability** — the #1 killer of solo OSS. Mitigation: Phase 3.
- **Solo bandwidth** — keep scope tight; the anti-goals double as scope control.
- **Per-platform native bundling** (PDFium/qpdf/Tesseract × Win/mac/Linux) is
  fiddly. Mitigation: nail Windows first, expand once stable.
- **Code-signing cost** (~$100–400/yr). Mitigation: donations, or ship
  unsigned + clear install instructions initially.
