# webxash3d-proxy

A Rust WebRTC-to-UDP proxy that allows browser clients running Xash3D WASM to connect to real CS 1.6 / Half-Life dedicated servers.

## Project Structure

```
webxash3d-proxy/
├── src/                    # Rust proxy server
│   ├── main.rs
│   ├── config.rs
│   ├── signaling.rs
│   └── bridge.rs
├── client/                 # Web client (TypeScript/Vite)
│   ├── src/
│   │   ├── index.html
│   │   ├── main.ts
│   │   └── webrtc.ts
│   ├── package.json
│   └── vite.config.ts
├── dist/                   # Built client (after npm run build)
├── scripts/
│   └── setup-client.sh
├── Cargo.toml
└── Dockerfile
```

## How It Works

```
Browser (Xash3D WASM)              Proxy                    Real CS 1.6 Server
        │                            │                              │
   WebSocket ◄──────────────────────►│  (signaling only)            │
   WebRTC Data Channels ◄───────────►│◄────────── UDP ─────────────►│
     • write (server → client)       │                              │
     • read  (client → server)       │                              │
```

## Quick Start

### 1. Build the Rust proxy

```bash
cargo build --release
```

### 2. Build the web client

```bash
cd client
npm install
npm run build
cd ..
```

Or use the helper script:
```bash
./scripts/setup-client.sh
```

### 3. Add game assets to `dist/`

After building, add these files to the `dist/` directory:

```
dist/
├── index.html          # (built by vite)
├── assets/             # (built by vite)
├── valve.zip           # Half-Life base assets
├── mainui.wasm         # From client/node_modules/xash3d-fwgs/
├── filesystem_stdio.wasm
└── cstrike/
    ├── cl_dlls/
    │   └── client.wasm
    └── extras.pk3
```

Copy WASM files:
```bash
cp client/node_modules/xash3d-fwgs/*.wasm dist/
```

### 4. Run the proxy

```bash
./target/release/webxash3d-proxy \
    --server 192.168.1.100:27015 \
    --static-dir ./dist \
    --game-dir cstrike
```

Open `http://localhost:27016` in your browser.

## CLI Options

```
Usage: webxash3d-proxy [OPTIONS] --server <SERVER>

Options:
  -s, --server <SERVER>              Game server address (e.g., 192.168.1.100:27015)
  -p, --port <PORT>                  Listen port [default: 27016]
      --host <HOST>                  Bind address [default: 0.0.0.0]
      --public-ip <PUBLIC_IP>        Public IP for ICE candidates (NAT traversal)
  -v, --verbose                      Enable debug logging
      --static-dir <STATIC_DIR>      Directory to serve static files from
      --game-dir <GAME_DIR>          Game directory name [default: cstrike]
      --console-commands <COMMANDS>  Console commands (comma-separated)
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GAME_SERVER` | Game server address |
| `LISTEN_PORT` | Listen port (default: 27016) |
| `LISTEN_HOST` | Bind address (default: 0.0.0.0) |
| `PUBLIC_IP` | Public IP for ICE candidates |
| `STATIC_DIR` | Static files directory |
| `GAME_DIR` | Game directory (default: cstrike) |
| `CONSOLE_COMMANDS` | Comma-separated console commands |

## Server Requirements

The game server needs **ReUnion** module to accept non-Steam clients (protocol 47/48).

Recommended `reunion.cfg`:
```
EnableQueryLimiter 0
ServerInfoAnswerType 1
FixBuggedQuery 1
```

## Docker

```dockerfile
# Build everything
docker build -t webxash3d-proxy .

# Run
docker run -p 27016:27016 \
    -e GAME_SERVER=192.168.1.100:27015 \
    webxash3d-proxy
```

## Development

### Proxy development
```bash
cargo run -- --server 127.0.0.1:27015 -v --static-dir ./dist
```

### Client development
```bash
cd client
npm run dev
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| WebRTC fails | Check firewall, try `--public-ip` flag |
| No server response | Verify server address `ip:port` |
| Instant disconnect | Install ReUnion on game server |
| WASM 404 errors | Check file paths match `/config` response |

## License

MIT
