<p align="center">
  <img src="assets/logo.png" alt="PWDManager" width="400">
</p>

<h1 align="center">PWDManager</h1>

A lightweight desktop password manager for Windows, built with [Dioxus](https://dioxuslabs.com/) and Rust. Credentials
are stored in a local SQLCipher-encrypted SQLite database. No data is ever sent to external servers.

Built on the Dioxus 0.7 framework, PWDManager compiles to a single native binary with a minimal footprint. The UI runs
on a system WebView2 instance -- no bundled Chromium, no Electron overhead. The result is a compact, fast application
that starts in under a second and uses very little memory.

## Why this project

Most password managers rely on cloud infrastructure or SaaS subscriptions. Even when they advertise end-to-end
encryption, their business model still depends on the user trusting a third-party server. PWDManager takes a different
approach: the database lives on the local filesystem, the encryption key never leaves the machine via Windows Credential
Manager, and there is no sync server, no cloud account, no telemetry.

## Features

**Password management**

- Store credentials with name, username, URL, password, and notes
- Password strength scoring on a 0-100 numeric scale, mapped to levels: WEAK, MEDIUM, STRONG, EPIC, GOD
- Built-in blacklist for common password detection
- Random password generator with configurable length, character set, and excluded symbols
- Diceware passphrase generator in English, Italian, and French, with optional numbers and special characters

**Security**

- Database encrypted at rest with SQLCipher (AES-256)
- Encryption key derived via Argon2id, stored in Windows Credential Manager
- User passwords hashed with Argon2 (zeroize enabled)
- Individual credentials encrypted per-field with AES-256-GCM (unique nonce per field)
- Recovery key generated as a 6-word Diceware passphrase, derivable via Argon2id with a dedicated salt
- Configurable auto-logout: 10 minutes, 1 hour, or 5 hours of inactivity

See [docs/security.md](docs/security.md) for a full breakdown of the security architecture.

**Import / Export**

- Export to JSON, CSV, and XML
- Import from all three formats with automatic deduplication by (URL, password) pair
- Skips passwords already present in the database during import
- Compatible with Chrome's CSV export -- export from `chrome://password-manager/settings` and import directly into
  PWDManager

**Desktop integration**

- System tray icon with visual state (different icon when authenticated vs logged out)
- Tray menu actions: Open, Logout, Quit
- Closing the window hides the application without terminating the process
- Auto-start on Windows boot via HKCU registry key, with Task Manager disabled-state detection

**Interface**

- Light and dark theme
- UI built on DaisyUI 5 and Tailwind CSS
- Dashboard with vault composition statistics (count per strength level)
- Paginated password list with strength filtering and sorting

**Updates**

- Automatic update check on startup
- Download with progress bar and minisign signature verification before installation
- Custom NSIS installer

## Prerequisites

- [Rust](https://rustup.rs/) (edition 2024)
- [Node.js](https://nodejs.org/) (for Tailwind CSS build)
- Windows 10/11 (x86_64)
- Visual Studio Build Tools with the C++ workload

## Build

```bash
# Clone the repository with submodules
git clone --recurse-submodules https://github.com/LucioPg/PWDManager.git

# Development build with hot reload
dx serve --desktop

# Release build
dx build --desktop --release
```

The release build produces an NSIS installer in the `dist/` directory.

## Workspace structure

The project is a Cargo workspace with six crates:

| Crate           | Purpose                                                          |
|-----------------|------------------------------------------------------------------|
| `PWDManager`    | Main application (UI, routing, business logic)                   |
| `gui_launcher`  | Desktop launcher with window config, icon embedding, and logging |
| `custom_errors` | Typed errors (DBError, AuthError, CryptoError)                   |
| `pwd-types`     | Core types: PasswordScore, StoredPassword, UserAuth              |
| `pwd-strength`  | Password strength evaluation with blacklist support              |
| `pwd-crypto`    | Argon2 hashing and AES-256-GCM encryption                        |

`custom_errors`, `pwd-types`, `pwd-strength`, and `pwd-crypto` are external Git dependencies. `pwd-dioxus` provides
shared UI components.

## Application data

- **Database**: `%LOCALAPPDATA%/PWDManager/pwdmanager.db` (SQLCipher encrypted)
- **Salt**: `%LOCALAPPDATA%/PWDManager/pwdmanager.db.salt` (16 bytes, Argon2id)
- **DB key**: Windows Credential Manager, service `PWDManager`
- **Log**: `%LOCALAPPDATA%/PWDManager/pwdmanager.log`
- **WebView2 data**: `%LOCALAPPDATA%/PWDManager/`

## Technical documentation

- [docs/security.md](docs/security.md) -- security architecture, encryption layers, key management
- [docs/howto_sqlitetemplate.md](docs/howto_sqlitetemplate.md) -- sqlx-template guide for automatic CRUD generation
- [docs/nsis-custom-template.md](docs/nsis-custom-template.md) -- custom NSIS template for the installer

## License and Commercial Use

This project is licensed under the **Prosperity Public License 3.0.0**.

### What does this mean for you?

- **Personal and Non-Profit Use:** You are free to use, study, and modify this software at no cost for personal,
  educational, or research purposes.
- **Commercial Use:** If you are a company or a professional using this software for profit-making activities, you are
  granted a **30-day trial period**.

### How to Obtain a Commercial License

To continue using the software for commercial purposes after the 30-day trial, you must purchase a dedicated commercial
license.

To request a quote or activate your license, please contact:
**ldcproductions@proton.me**

*Please use the subject line: "Commercial License Request - PWDManager"*

---
*Note: This software is built using the Dioxus framework (MIT/Apache 2.0). All third-party open-source components remain
subject to their respective licenses.*
