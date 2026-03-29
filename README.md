# AriaNg App

A modern, cross-platform desktop download manager built with [Tauri](https://tauri.app/), [aria2](https://aria2.github.io/), and [AriaNg](https://github.com/mayswind/AriaNg).

## Download

Download the latest release for your platform from the [Releases](../../releases) page:

| Platform | Format |
|---|---|
| Windows x64 | `.exe` installer / portable `.zip` |
| macOS x64 | `.dmg` |
| macOS ARM64 (Apple Silicon) | `.dmg` |
| Linux x64 | `.deb` / `.AppImage` |

## Architecture

```
AriaNg App
+-- Tauri (Rust)          # Native window, system tray, process lifecycle
|   +-- aria2 (sidecar)   # Download engine (bundled binary)
|   +-- AriaNg (frontend) # Web-based download management UI
```

- **Tauri** provides the native desktop shell, system tray integration, and manages the aria2 process lifecycle including startup, monitoring, crash recovery, and shutdown.
- **aria2** is the download engine, running as a sidecar process and communicating via JSON-RPC.
- **AriaNg** is the frontend UI, served locally in the WebView and connecting to aria2's RPC interface.

## Build from Source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Platform-specific dependencies (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

### Steps

1. **Clone the repository**

   ```bash
   git clone https://github.com/user/ariang-desktop.git
   cd ariang-desktop
   ```

2. **Download AriaNg frontend**

   Download [AriaNg](https://github.com/mayswind/AriaNg/releases) and extract it into the `frontend/` directory:

   ```bash
   mkdir -p frontend
   # Extract AriaNg release zip into frontend/
   ```

3. **Place aria2c binary**

   Download or build [aria2](https://github.com/aria2/aria2/releases) for your platform and place the binary at:

   ```
   src-tauri/binaries/aria2c-<target-triple>[.exe]
   ```

   Target triples:
   - Windows: `x86_64-pc-windows-msvc`
   - macOS Intel: `x86_64-apple-darwin`
   - macOS ARM: `aarch64-apple-darwin`
   - Linux: `x86_64-unknown-linux-gnu`

4. **Install dependencies and build**

   ```bash
   npm install
   npm run build
   ```

5. **Run in development mode**

   ```bash
   npm run dev
   ```

## Configuration

Configuration is stored in your platform's app data directory:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\com.ariang.desktop\config.json` |
| macOS | `~/Library/Application Support/com.ariang.desktop/config.json` |
| Linux | `~/.config/com.ariang.desktop/config.json` |

Default settings:

| Setting | Default |
|---|---|
| RPC Port | 6800 |
| Max concurrent downloads | 5 |
| Connections per server | 16 |
| Download directory | System Downloads folder |

## Credits

- [aria2](https://aria2.github.io/) - The ultra-fast download utility
- [AriaNg](https://github.com/mayswind/AriaNg) - A modern web frontend for aria2
- [Tauri](https://tauri.app/) - Build cross-platform desktop apps with web technologies

## License

MIT
