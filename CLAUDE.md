# CLAUDE.md - AI Assistant Context

## Project Summary

**webxash3d-proxy** is a Rust WebRTC-to-UDP proxy that allows browser clients to play CS 1.6 / Half-Life on real game servers. It consists of:

1. **Rust proxy server** - WebRTC signaling + UDP bridge
2. **TypeScript web client** - Xash3D WASM wrapper with WebRTC networking

## Project Structure

```
webxash3d-proxy/
├── src/                        # Rust proxy server
│   ├── main.rs                 # HTTP server, /config endpoint, static files
│   ├── config.rs               # CLI args (clap), env vars
│   ├── signaling.rs            # WebRTC peer connection, data channels, ICE
│   └── bridge.rs               # UDP <-> WebRTC packet forwarding
├── client/                     # Web client (TypeScript)
│   ├── src/
│   │   ├── index.html          # UI, canvas, login form
│   │   ├── main.ts             # Game init, config loading, asset loading
│   │   ├── webrtc.ts           # Xash3DWebRTC class, WebSocket signaling
│   │   ├── valve.zip           # Half-Life base assets (user provides)
│   │   ├── favicon.png
│   │   └── logo.png
│   ├── package.json            # Dependencies: xash3d-fwgs, cs16-client
│   ├── vite.config.ts          # Build config, static file copying
│   └── tsconfig.json
├── dist/                       # Built client output
├── Cargo.toml
└── Dockerfile
```

## Architecture

```
┌─────────────────────────┐     ┌─────────────────────────┐     ┌─────────────────┐
│  Browser                │     │  Proxy (Rust)           │     │  Game Server    │
│  ├─ index.html          │     │                         │     │  (HLDS+ReUnion) │
│  ├─ main.ts             │     │  ┌─────────────────┐    │     │                 │
│  └─ webrtc.ts           │     │  │ Signaling       │    │     │                 │
│      │                  │     │  │ (WebSocket)     │    │     │                 │
│      │ WebSocket ───────┼────►│  └─────────────────┘    │     │                 │
│      │                  │     │                         │     │                 │
│      │ WebRTC           │     │  ┌─────────────────┐    │     │                 │
│      ├─ write channel ◄─┼─────┼──┤ Bridge          ├────┼────►│  UDP :27015     │
│      └─ read channel ───┼─────┼──┤ (per client)    │    │     │                 │
│                         │     │  └─────────────────┘    │     │                 │
│  Xash3D WASM Engine     │     │                         │     │                 │
└─────────────────────────┘     └─────────────────────────┘     └─────────────────┘
```

## Rust Proxy (src/)

### main.rs
- Axum HTTP server on port 27016
- Routes: `/ws`, `/websocket`, `/config`, `/health`
- Static file serving via `--static-dir` with `ServeDir`
- `/config` endpoint returns client configuration with library paths and files_map

### config.rs
- CLI args via clap: `--server`, `--port`, `--static-dir`, `--game-dir`, etc.
- Environment variable support for all options
- `get_console_commands()` helper for parsing comma-separated commands

### signaling.rs
- Creates RTCPeerConnection with STUN server (stun.l.google.com)
- Creates two data channels: `write` (proxy→browser), `read` (browser→proxy)
- Data channels configured as ordered for reliable file downloads
- SDP offer/answer exchange over WebSocket
- ICE candidate trickle
- Starts Bridge when both channels open

### bridge.rs
- Per-client UDP socket connected to game server
- `forward_udp_to_webrtc()`: UDP recv → write channel send
- `setup_webrtc_to_udp()`: read channel on_message → UDP send
- Handles channel close/error for cleanup

## Web Client (client/)

### Dependencies (package.json)
- `xash3d-fwgs`: Xash3D engine WASM (xash.wasm, libmenu.wasm, filesystem_stdio.wasm, renderers)
- `cs16-client`: CS 1.6 client WASM (client_emscripten_wasm32.wasm, extras.pk3)

### vite.config.ts
- Copies WASM files from node_modules to dist automatically
- Copies cstrike/ directory from cs16-client
- Copies valve.zip, favicon.png, logo.png from src/
- Output to `../../dist` (project root)

### main.ts
- Fetches `/config` for server configuration
- Loads valve.zip (jszip) and extras.pk3
- Initializes Xash3DWebRTC with library paths from config
- Uses `files_map` to translate .so requests to .wasm URLs
- Executes `connect 127.0.0.1:8080` (virtual address for WebRTC)

### webrtc.ts
- `Xash3DWebRTC` class extends `Xash3D` from xash3d-fwgs
- WebSocket connection to `/websocket` for signaling
- Handles WebRTC offer/answer/candidate messages
- `write` channel: receives server packets, enqueues to net.incoming
- `read` channel: sendto() sends packets to server
- Packets use virtual address 127.0.0.1:8080

## Build Commands

```bash
# Rust proxy
cargo build --release

# Web client (requires valve.zip in client/src/)
cd client && npm install && npm run build
```

## Key APIs

### GET /config
Returns configuration for the web client:
```json
{
  "arguments": ["-windowed", "-game", "cstrike"],
  "console": [],
  "game_dir": "cstrike",
  "libraries": {
    "client": "/cstrike/cl_dlls/client_emscripten_wasm32.wasm",
    "server": "/cstrike/dlls/cs_emscripten_wasm32.wasm",
    "extras": "/cstrike/extras.pk3",
    "menu": "/cstrike/cl_dlls/menu_emscripten_wasm32.wasm",
    "filesystem": "/filesystem_stdio.wasm"
  },
  "dynamic_libraries": ["dlls/cs_emscripten_wasm32.so", "/rwdir/filesystem_stdio.wasm"],
  "files_map": {
    "dlls/cs_emscripten_wasm32.so": "/cstrike/dlls/cs_emscripten_wasm32.wasm",
    "dlls/hl_emscripten_wasm32.so": "/cstrike/dlls/cs_emscripten_wasm32.wasm",
    "/rwdir/filesystem_stdio.wasm": "/filesystem_stdio.wasm"
  }
}
```

### WebSocket Signaling
```json
{"event": "offer", "data": {"type": "offer", "sdp": "..."}}
{"event": "answer", "data": {"type": "answer", "sdp": "..."}}
{"event": "candidate", "data": {"candidate": "...", "sdpMid": "...", "sdpMLineIndex": ...}}
```

## Required Assets

After `npm run build`, the only manual requirement is `valve.zip` in `client/src/`:
- Contains Half-Life base assets (valve/ folder with .wad, .pak files)
- Must be created from a Half-Life installation

Everything else is automatic:
- WASM files copied from xash3d-fwgs package
- CS 1.6 files copied from cs16-client package

## Common Tasks

### Add new CLI option
1. Add field to `Config` struct in `config.rs`
2. Add `#[arg(...)]` attribute with clap options
3. Use in `main.rs` or pass to handlers

### Modify client UI
1. Edit `client/src/index.html`
2. Run `npm run build` in `client/`

### Change WebRTC settings
- Data channel options: `signaling.rs` RTCDataChannelInit
- ICE servers: `signaling.rs` RTCConfiguration
- NAT traversal: Use `--public-ip` flag

### Change game directory
- Use `--game-dir valve` for Half-Life
- Use `--game-dir cstrike` for CS 1.6 (default)

## Server Requirements

### ReUnion Module
Game server needs ReUnion to accept non-Steam clients (protocol 47/48).

### Fast Download
For servers with custom content, configure sv_downloadurl:
```
sv_downloadurl "http://fastdl.example.com/cstrike/"
sv_allowdownload 1
```
Without this, custom file downloads may timeout.

## Debugging

### Proxy
```bash
cargo run -- --server 127.0.0.1:27015 -v --static-dir ./dist
```

### Client
- Browser DevTools Console for JS errors
- Network tab for WebSocket messages
- `chrome://webrtc-internals` for WebRTC debugging

### Common Issues
- "Unsupported Extension Type" warnings: Safe to ignore (WebRTC SCTP)
- File download timeout: Configure sv_downloadurl on game server
- Connection fails: Check ReUnion is installed on game server
