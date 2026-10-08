# Web mode

v1 is the self-hosted `mailune-server`. The browser talks to the native core over JSON-RPC on a WebSocket. The core still owns protocols, storage, and sync. The server is a surface in front of that core, the same way the other shells are.

WASM that speaks JMAP directly from the browser comes later. It is not part of v1.

This is the project decision: the native core behind JSON-RPC over WebSocket first, and the WASM JMAP-direct mode after that.
