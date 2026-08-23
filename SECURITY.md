# Security Policy

## Supported versions

Security fixes are provided for the latest published TypeLess release.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

---

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub Security Advisories:

1. Open the repository's **Security** tab.
2. Select **Advisories** and **Report a vulnerability**.
3. Include affected versions, reproduction steps, impact, and any suggested mitigation.

Do not disclose an unpatched vulnerability in a public issue or discussion. Please do not include private audio recordings, tokens, personal paths, or sensitive data in a report.

Reports concerning command or option injection in speech pipelines, path traversal in local models, sidecar supply chain integrity, unsafe audio buffer parsing, keystroke injection safety, or Tauri CSP bypasses are especially helpful.

---

## Scope notes

TypeLess is a 100% offline, local desktop application for Windows:
- **No Telemetry or Analytics:** TypeLess does not collect, store, or transmit your voice data, keystrokes, or transcripts.
- **No Hosted Accounts or Cloud Database:** All AI processing is performed entirely on your local CPU / hardware via `whisper.cpp`.
- **System Permissions:** Low-level OS access is limited to global hotkey detection (`GetAsyncKeyState`) and simulated Unicode keyboard input (`SendInput`).
- Third-party model providers, upstream `whisper.cpp` changes, and Windows platform policy updates are outside the project's direct control.
