# Logo Attribution

This directory ships ASCII-art logos used by the COSMIC ORDER
screensaver. COSMIC ORDER itself is GPL-3.0-only (see
[LICENSE](../../../LICENSE)); the *content* of individual `.txt` files
in this directory carries the licensing / trademark status documented
below.

COSMIC ORDER is **not affiliated with or endorsed by** System76. The
"COSMIC" name and brand are trademarks of System76, Inc.; the
ASCII renderings here reference that mark to identify the desktop
environment this software extends, not to imply official affiliation.

If you are a brand owner and would like a logo removed or the
attribution adjusted, open an issue at
<https://github.com/jfreed-dev/cosmic-order/issues>.

---

## COSMIC (System76)

| File | Status |
|------|--------|
| `cosmic-icon.txt` | ASCII art derived from the COSMIC Desktop brand iconography. |
| `cosmic-name.txt` | "COSMIC" rendered in the [figlet](http://www.figlet.org/) "ANSI Shadow" font. Typefaces are not copyrightable in the United States. |
| `cosmic-name-with-icon.txt` | Composition of the two above. |

The "COSMIC" name and the COSMIC Desktop branding are trademarks of
**System76, Inc.** (<https://system76.com/cosmic>). These ASCII
renderings are provided for **nominative use** — identification of the
desktop environment that COSMIC ORDER extends — and do not imply
endorsement, sponsorship, or affiliation. The ASCII files themselves
(as creative works) are GPL-3.0-only along with the rest of the repo,
but that license cannot grant any rights in the underlying trademark.

If you do not run COSMIC Desktop, consider not using these logos.

---

## Removed third-party trademarks

ASCII renderings of the **Framework Computer** cog and the **Pop!_OS /
System76** name were removed from this directory in v0.15.0 (commit
`31c53f3`, 2026-05-10). They were dropped as a clearer-risk-removal
move; see [../../docs/LICENSING.md](../../../docs/LICENSING.md) §
"ASCII Logo Status" for the audit trail.

If you want similar logos in your own setup, drop a UTF-8 text file
into `~/.local/share/cosmic-order/screensaver/` and point the
screensaver's `LOGO_FILE` config there.

---

## A note on figlet wordmarks

`*-name.txt` files are figlet output of plain ASCII strings. The "ANSI
Shadow" figlet font is freely redistributable. Typefaces are not
copyrightable in the United States, so the rendered ASCII art is not
a copyright-protected work. Trademark status of the underlying *word*
is independent and addressed in each project's section above.

---

## User-supplied logos

Anything you drop into `~/.local/share/cosmic-order/screensaver/` (or
point the screensaver `LOGO_FILE` at directly) is yours; this
attribution file does not apply to logos you provide.
