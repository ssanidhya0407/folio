# Publishing folio to the Microsoft Store

Everything here is done once. Steps 1–3 are the only ones that need you
personally — the rest is scripted.

---

## 0. What the Store gets you

The Store signs the package with its own certificate, which is the whole
reason to bother: **it removes the SmartScreen warning without buying a code
signing certificate.** That warning is the single biggest thing standing
between a non-technical user and actually running folio, and this fixes it for
$19 instead of $100–400 a year.

Keep shipping the `.msi`/NSIS build from GitHub Releases too — the OSS audience
will not install from the Store, and winget/scoop need a plain installer.

---

## 1. Create a Partner Center account

<https://partner.microsoft.com/dashboard/registration>

- **Individual** account: one-time **$19**. This is the only money folio needs.
- A Company account ($99) is only worth it if you want the publisher to be a
  legal entity. Individual is fine and faster to verify.
- Verification takes anywhere from a few hours to a few days.

## 2. Reserve the app name — done

The name **`Folio : Local PDF Studio`** is reserved, and every file in this
repo already uses it:

- `msix/AppxManifest.xml` — `<DisplayName>` and the tile `DisplayName`
- `msix/store-listing.md` — the listing copy

The executable and window are still called `folio`. That is deliberate and
allowed: the Store listing name and the application name do not have to match,
and the short name keeps the taskbar tidy.

## 3. Copy your identity values

Partner Center → your app → **Product management → Product identity**. Copy the
three values into a new file `msix/identity.json` (it is gitignored — these are
account details, not project config):

```json
{
  "name": "12345Publisher.FolioLocalPDFStudio",
  "publisher": "CN=ABCDEF12-3456-7890-ABCD-EF1234567890",
  "publisherDisplayName": "Your Publisher Name",
  "version": "1.0.0.0"
}
```

> These three are **not** the app name, and cannot be guessed — Partner Center
> generates them per account. `name` is usually your publisher prefix plus a
> squashed version of the reserved name; `publisher` is a GUID. Copy them
> exactly, including case.

- `name` is **Package/Identity/Name**
- `publisher` is **Package/Identity/Publisher** (it starts with `CN=`)
- `publisherDisplayName` is **Package/Properties/PublisherDisplayName**
- `version` must have four parts, and the Store requires the **last part to be
  `0`**. Bump the third part for each submission: `1.0.1.0`, `1.0.2.0`, …

## 4. Build the package

```powershell
# once per machine: the native engines folio ships with
python assets\fetch-runtime.py --lang eng

# the OCR engine needs an elevated prompt (its installer demands admin)
python assets\fetch-runtime.py --skip pdfium qpdf tessdata --tesseract

# then, any time
powershell -ExecutionPolicy Bypass -File msix\build-msix.ps1
```

Output: `target\msix\folio-1.0.0.0.msix`.

**Upload that file unsigned.** Microsoft signs it during ingestion, and a
package you signed yourself will be rejected.

### Testing it locally first

```powershell
powershell -ExecutionPolicy Bypass -File msix\build-msix.ps1 -SelfSign
```

This makes a second, self-signed copy you can install. Trust the test
certificate once (elevated PowerShell), then double-click the `.msix`. Remember
to uninstall the sideloaded copy before installing the Store one.

## 5. Fill in the Store listing

Copy is written and ready in [`store-listing.md`](store-listing.md). Paste it
into Partner Center → **Store listings**.

### Screenshots — the one asset that needs you

The Store requires **at least one** screenshot; 4–8 is much better for
conversion. They cannot be generated from the repo, so run the app and capture:

| # | Shot | Why it converts |
|---|---|---|
| 1 | Main window with a PDF open and the tool list visible | Shows the whole product at a glance |
| 2 | The **Redact** tool with boxes drawn on a page | The feature nothing free does properly |
| 3 | The **Compress** result card showing the size drop | Concrete, believable benefit |
| 4 | The **About** panel with the offline claims | The differentiator, in folio's own words |
| 5 | **Batch** report with a list of processed files | Signals "real tool", not a toy |

Requirements: **PNG**, **1366×768 or 1920×1080**, no transparency, max 50.
Capture with `Win`+`Shift`+`S` or Snipping Tool, and resize the window to a
16:9 shape first.

## 6. Properties and ratings

| Field | Value |
|---|---|
| Category | **Productivity** |
| Subcategory | Personal finance / Other → **Other** |
| Pricing | **Free** |
| Markets | All |
| Device families | Windows 10 desktop 10.0.17763.0 and later |
| Privacy policy URL | **Required** — see below |
| Support contact | Your GitHub Issues URL or an email |

### Privacy policy URL

Partner Center requires a reachable URL even though folio collects nothing.
[`PRIVACY.md`](../PRIVACY.md) at the repo root is written for this. Publish it
via GitHub Pages (Settings → Pages → deploy from `main`) and use the resulting
`https://<you>.github.io/folio/privacy` URL.

### Age rating questionnaire

Answer **No** to everything. folio has no ads, no user-generated content, no
data collection, no in-app purchases, no network access at all. Expect a rating
of 3+ / Everyone.

### Product declarations

- ✅ This app has been tested to meet accessibility guidelines — only tick this
  if you have actually checked keyboard navigation and contrast.
- ❌ Does not depend on any non-Microsoft driver or service.
- ❌ Collects no personal information.

## 6a. Two things that catch Tauri apps specifically

### WebView2

folio's window is a WebView2 control, so the WebView2 Evergreen Runtime has to
be present. The `.msi`/NSIS installers handle this by running Microsoft's
bootstrapper (`webviewInstallMode` in `tauri.conf.json`), but **a Store package
may not run an installer**, so the MSIX relies on the runtime already being
there.

In practice that is fine: it ships with Windows 11, and on Windows 10 it
arrives through Windows Update and with Microsoft 365. It is the same bet every
Tauri and Electron-adjacent Store app makes. If you ever get support reports of
a blank window on old Windows 10 builds, the fix is to switch to a
**fixed-version WebView2**: download the fixed-version runtime, drop it into
`src-tauri/runtime/`, and set `webviewInstallMode` to `fixedRuntime`. It adds
roughly 180 MB to the package, which is why it is not the default.

### The certification kit checks every binary, not just yours

folio's package contains third-party executables (`qpdf.exe`,
`tesseract.exe`) and their DLLs. The Windows App Certification Kit inspects all
of them for `/DYNAMICBASE` and `/NXCOMPAT`. The official upstream builds that
`fetch-runtime.py` downloads pass; a self-compiled replacement might not. If
certification fails on a binary analyzer check, that is where to look.

You can run the same checks locally before submitting — install the Windows SDK
component "Windows App Certification Kit", then point it at the built package.

## 7. Submit

Certification usually takes a few hours to three days. A failed certification
report tells you exactly which policy tripped; the two that catch desktop apps
most often are:

- **10.1.1 — distinct value**: folio is fine here (it is a full application,
  not a wrapper around a website).
- **10.2 — security**: full-trust desktop apps pass, but the package must not
  declare capabilities it does not use. folio declares only `runFullTrust`.

---

## Later submissions

1. Bump the version in `Cargo.toml`, `src-tauri/tauri.conf.json` and
   `msix/identity.json` (keep the fourth part `0`).
2. Re-run `build-msix.ps1`.
3. Upload the new `.msix` as a new submission.

## Licensing note

folio is GPL-3.0. The Store's Standard Application License Terms explicitly
defer to a product's own license when one is supplied, so shipping GPL software
is allowed and common. Point the listing's licence field at the repository's
`LICENSE` and make the source link prominent — that is what satisfies GPL §3's
"corresponding source" obligation for users who install from the Store.
