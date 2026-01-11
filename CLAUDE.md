# CLAUDE.md - AI Assistant Context

## Project Summary

**webxash3d-proxy** is a complete solution for running CS 1.6 / Half-Life in a browser, connecting to real game servers. It consists of:

1. **Rust proxy server** - WebRTC-to-UDP bridge
2. **TypeScript web client** - Xash3D WASM wrapper with WebRTC

## Project Structure

```
webxash3d-proxy/
├── src/                        # Rust proxy server
│   ├── main.rs                 # HTTP server, /config endpoint
│   ├── config.rs               # CLI args, env vars
│   ├── signaling.rs            # WebRTC signaling, data channels
│   └── bridge.rs               # UDP ↔ WebRTC forwarding
├── client/                     # Web client (TypeScript)
│   ├── src/
│   │   ├── index.html          # UI, canvas, login form
│   │   ├── main.ts             # Game initialization, config loading
│   │   └── webrtc.ts           # WebRTC connection, Xash3D wrapper
│   ├── package.json            # npm dependencies
│   ├── vite.config.ts          # Vite build config
│   └── tsconfig.json
├── dist/                       # Built client output
├── scripts/
│   └── setup-client.sh         # Build helper script
├── Cargo.toml
├── Dockerfile
└── README.md
```

## Architecture

```
┌─────────────────────────┐     ┌─────────────────────────┐     ┌─────────────────┐
│  Browser                │     │  Proxy (Rust)           │     │  Game Server    │
│  ├─ index.html          │     │                         │     │  (HLDS)         │
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
- Static file serving via `--static-dir`
- Client configuration via `/config` endpoint

### config.rs
- CLI args: `--server`, `--port`, `--static-dir`, `--game-dir`, etc.
- Environment variable support
- Configuration struct with validation

### signaling.rs
- WebRTC peer connection setup
- Creates two data channels: `write` (server→client), `read` (client→server)
- SDP offer/answer exchange
- ICE candidate trickle
- Waits for both channels to open before starting bridge

### bridge.rs
- Per-client UDP socket to game server
- Bidirectional packet forwarding
- Handles channel close/error for cleanup

## Web Client (client/)

### index.html
- Game canvas
- Login form (username, touch controls)
- Loading animation
- Social links

### main.ts
- Fetches `/config` for server configuration
- Loads game assets (valve.zip, WASM modules)
- Initializes Xash3D with WebRTC networking
- Handles username and console commands

### webrtc.ts
- `Xash3DWebRTC` class extending `Xash3D`
- WebSocket connection to `/websocket`
- WebRTC peer connection handling
- Data channel message routing
- Integrates with Xash3D's network layer

## Build Commands

```bash
# Rust proxy
cargo build --release

# Web client
cd client && npm install && npm run build

# Or use helper script
./scripts/setup-client.sh
```

## Key APIs

### GET /config
```json
{
  "arguments": ["-windowed"],
  "console": [],
  "game_dir": "cstrike",
  "libraries": {
    "client": "/cstrike/cl_dlls/client.wasm",
    "server": "/cstrike/dlls/server.wasm",
    "extras": "/cstrike/extras.pk3",
    "menu": "/mainui.wasm",
    "filesystem": "/filesystem_stdio.wasm"
  }
}
```

### WebSocket Signaling
```json
{"event": "offer", "data": {"type": "offer", "sdp": "..."}}
{"event": "answer", "data": {"type": "answer", "sdp": "..."}}
{"event": "candidate", "data": {"candidate": "..."}}
```

## Required Game Assets

After building client, add to `dist/`:
- `valve.zip` - Half-Life base assets
- `*.wasm` - From `client/node_modules/xash3d-fwgs/`
- `cstrike/` - Game directory with client.wasm, extras.pk3

## Common Tasks

### Add new CLI option
1. Add field to `Config` struct in `config.rs`
2. Add `#[arg(...)]` attribute
3. Use in `main.rs` or pass to handlers

### Modify client UI
1. Edit `client/src/index.html`
2. Run `npm run build` in `client/`

### Change WebRTC settings
- Data channel options: `signaling.rs:57-61`
- ICE servers: `signaling.rs:379-383`
- STUN/TURN: Add to `RTCConfiguration`

## Debugging

### Proxy
```bash
cargo run -- --server 127.0.0.1:27015 -v
```

### Client
- Browser DevTools Console
- Network tab for WebSocket messages
- `webrtc://` internals in Chrome

## Related Files in webxash3d-fwgs

Original project at `/Users/noname-m2/projects/webxash3d-fwgs`:
- `docker/cs-web-server/src/server/sfu.go` - Go WebRTC server
- `docker/cs-web-server/src/client/` - Original client source
- Uses embedded Xash3D engine via CGO
