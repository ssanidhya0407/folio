# folio — Privacy Policy

_Last updated: 13 September 2026_

## The short version

folio does not collect, transmit, store or share any information about you or
your documents. It cannot: the application contains no networking code.

## What folio collects

Nothing.

- No personal information.
- No account — there is nothing to sign in to.
- No analytics, telemetry, crash reporting or usage statistics, not even
  anonymous or opt-in.
- No advertising, and no advertising identifiers.
- No cookies or web tracking.

## What happens to your documents

Your files are opened, processed and saved by folio on your own computer.
They are never uploaded anywhere, because folio has no ability to upload
anything. Output files go only where you tell folio to put them; the originals
are never modified in place.

Temporary files created while processing (for example, page images produced
during OCR) are written to your system temporary folder and deleted when the
operation finishes.

## What folio stores on your computer

A single small settings file, holding the folders you last opened and saved
from, so the file dialogs start somewhere useful:

```
%APPDATA%\app.folio.desktop\settings.json
```

It contains two folder paths and nothing else. Deleting it resets that
behaviour. Uninstalling folio removes it.

## Network access

folio makes no network connections of any kind — not for updates, not for
licence checks, not for fonts, not for error reporting. This is enforced, not
merely promised:

- The project's `deny.toml` bans every HTTP, TLS, socket and async-networking
  library, and continuous integration runs `cargo deny check bans` on every
  change. If networking code ever entered the dependency tree, the build would
  fail.
- The Microsoft Store package declares **no network capabilities**. A packaged
  Windows application cannot use a capability it has not declared, so this is
  enforced by the operating system regardless of what the code does.

You can confirm both: the source is public, and folio runs normally with the
machine disconnected from the network.

## Third-party components

folio bundles three open-source engines, all of which run locally and none of
which makes network requests:

| Component | Purpose | Licence |
|---|---|---|
| PDFium | Rendering pages to images | BSD-3-Clause |
| qpdf | Compression and encryption | Apache-2.0 |
| Tesseract | Optical character recognition | Apache-2.0 |

No third party receives any data from folio.

## Children

folio is safe for all ages. It collects nothing, shows no advertising and
contains no user-generated or online content.

## Changes to this policy

If this policy ever changes it will be updated here, and the change will appear
in the project's public commit history.

## Contact

Questions about this policy can be raised as an issue on the project's public
issue tracker: <https://github.com/ssanidhya0407/folio/issues>
