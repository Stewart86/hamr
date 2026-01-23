# Security Policy

## How Hamr Works

Hamr is a launcher that executes plugins. The core launcher (Rust/GTK4) does not perform privileged operations itself - it spawns plugin processes and runs commands that plugins return.

**Hamr executes what plugins tell it to execute.**

## Built-in Plugins

All built-in plugins are written in Python specifically for transparency and easy auditing. You can inspect exactly what each plugin does by reading the source code in the `plugins/` directory.

Built-in plugins only use standard system tools (`wl-copy`, `xdg-open`, `notify-send`, etc.) and do not make network requests unless explicitly required for functionality (e.g., `dict` plugin for definitions, `flathub` for app search).

## Plugin Trust Model

Hamr uses SHA256 checksums to verify the integrity of built-in plugins. This helps you detect if any plugin files have been tampered with.

### Checksum Verification

Official releases include a `checksums.json` file in the plugins directory containing SHA256 hashes of all built-in plugin files. The daemon verifies these checksums at startup and logs warnings for any modifications.

**Plugin verification statuses:**

| Status | Meaning |
|--------|---------|
| VERIFIED | All files match expected checksums |
| MODIFIED | One or more files have been changed |
| UNKNOWN | Plugin is not in checksums (user-installed) |

### Audit Command

Run `hamr plugins audit` to view the verification status of all installed plugins:

```bash
$ hamr plugins audit

Plugin Audit Report

Checksums: /usr/share/hamr/plugins/checksums.json
Plugins tracked: 15

VERIFIED (12):
  [OK] apps             Applications
  [OK] calculate        Calculator
  ...

MODIFIED (1):
  [!!] shell            Shell Commands
       - handler.py

UNKNOWN (2):
  [??] my-custom        My Custom Plugin
  [??] another          Another Plugin
```

If you see MODIFIED plugins that you did not intentionally change, investigate the modifications before using Hamr. Modified plugins could indicate:
- Legitimate local customizations
- Interrupted update
- Potentially malicious tampering

### Regenerating Checksums

After intentionally modifying built-in plugins, you can regenerate checksums:

```bash
./scripts/generate-plugin-checksums.sh
```

This generates a new `plugins/checksums.json` file. Note that this should only be done for development or if you trust your local modifications.

## Third-Party Plugins

User-installed plugins in `~/.config/hamr/plugins/` run with your user permissions. Before installing third-party plugins:

- Review the source code
- Understand what commands it executes
- Check what data it accesses

Hamr does not sandbox plugins. A malicious plugin can do anything your user account can do.

Third-party plugins always show as UNKNOWN in the audit report since they are not included in the official checksums.

## Reporting Security Issues

If you discover a security vulnerability in Hamr or any built-in plugin, please report it through GitHub's security advisory feature:

1. Go to https://github.com/stewart86/hamr/issues/new/choose
2. Select "Report a security vulnerability"
3. Provide details about the vulnerability

This ensures your report remains private until we can address it. We will coordinate with you on responsible disclosure.

## Disclaimer

Hamr is provided as-is. The maintainers are not responsible for damages caused by plugins, whether built-in or third-party. Users are responsible for reviewing and trusting the plugins they install and execute.
