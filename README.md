# webxash3d-proxy

A Rust WebRTC-to-UDP proxy that allows browser clients running Xash3D WASM to connect to real CS 1.6 / Half-Life dedicated servers.

## Project Structure

```
webxash3d-proxy/
├── src/                    # Rust proxy server
│   ├── main.rs             # HTTP server, /config endpoint
│   ├── config.rs           # CLI args, env vars
│   ├── signaling.rs        # WebRTC signaling, data channels
│   └── bridge.rs           # UDP <-> WebRTC forwarding
├── client/                 # Web client (TypeScript/Vite)
│   ├── src/
│   │   ├── index.html      # UI, canvas, login form
│   │   ├── main.ts         # Game initialization
│   │   ├── webrtc.ts       # WebRTC connection
│   │   └── valve.zip       # Half-Life base assets (you provide)
│   ├── package.json
│   └── vite.config.ts
├── dist/                   # Built client (after npm run build)
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
```

### 3. Add valve.zip

The build requires `valve.zip` (Half-Life base assets). Place it in `client/src/`:

```bash
# Create valve.zip from your Half-Life installation
cd /path/to/Half-Life
zip -r valve.zip valve/
cp valve.zip /path/to/webxash3d-proxy/client/src/
```

### 4. Build the client

```bash
cd client
npm run build
```

This automatically copies to `dist/`:
- All WASM files from `xash3d-fwgs` package (engine, menu, filesystem, renderers)
- CS 1.6 client files from `cs16-client` package (client.wasm, extras.pk3)
- valve.zip, favicon, logo

### 5. Run the proxy

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

### ReUnion Module

The game server needs **ReUnion** module to accept non-Steam clients (protocol 47/48).

Recommended `reunion.cfg`:
```
EnableQueryLimiter 0
ServerInfoAnswerType 1
FixBuggedQuery 1
```

### Fast Download (Recommended)

If your server has custom content (maps, sounds, models), configure HTTP fast download to avoid slow UDP transfers:

```
// server.cfg
sv_downloadurl "http://your-fastdl-server.com/cstrike/"
sv_allowdownload 1
```

Without this, custom file downloads may timeout in the browser client.

## Docker

```bash
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

## NPM Packages Used

| Package | Description |
|---------|-------------|
| `xash3d-fwgs` | Xash3D engine compiled to WASM (xash.wasm, libmenu.wasm, filesystem_stdio.wasm, renderers) |
| `cs16-client` | CS 1.6 client compiled to WASM (client.wasm, extras.pk3) |

## Troubleshooting

| Issue | Solution |
|-------|----------|
| WebRTC fails | Check firewall, try `--public-ip` flag |
| No server response | Verify server address `ip:port` |
| Instant disconnect | Install ReUnion on game server |
| WASM 404 errors | Run `npm run build` in client/ |
| File download timeout | Configure `sv_downloadurl` on game server |
| "Unsupported Extension Type" warnings | Safe to ignore (WebRTC SCTP extensions) |

## License

MIT
