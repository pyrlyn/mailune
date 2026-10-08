# Self-host

The container entrypoint is the `mailune-server` binary. It serves the JSON-RPC methods already defined by `mailune-rpc` on a WebSocket. Passkeys are not part of this server.

The process reads `MAILUNE_TOKEN` and refuses an empty value. It does not print the token. It listens on `127.0.0.1` inside the container and writes the bound address to stderr.

## Image

From the repository root:

```
docker build -t mailune-server .
docker run --rm -e MAILUNE_TOKEN=replace-me mailune-server
```

## Without the image

```
MAILUNE_TOKEN=replace-me cargo run --locked -p mailune-server
```
