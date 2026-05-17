# Bittime CLI — TODO List

## 🚀 High Priority (Reliability & Performance)
- [x] **Time Offset Caching**: Cache server time offset on first request to avoid extra RTT on every signed call.
- [ ] **WS Reconnection**: Implement automatic exponential backoff reconnection for WebSocket streams.
- [ ] **History Persistence**: Save REPL shell history to `~/.config/bittime/history`.
- [x] Implement proactive rate limiting (token bucket)
- [x] Unified retry logic with exponential backoff
- [x] Refactor output architecture for String capture
- [x] Implement MCP server foundation
    - [x] Tool registry (automatic command mapping)
    - [x] JSON Schema generation from clap
    - [x] JSON-RPC over stdio
- [ ] WebSocket Resilience
    - [ ] Auto-reconnection with exponential backoff
    - [ ] Message queueing during disconnects
- [ ] Update E2E tests for new output format
- [ ] Final integration test with AI agent

## 🤖 AI & Agent Support
- [ ] **Error Catalog**: Generate a machine-readable `error-catalog.json` with retry strategies.
- [ ] **Tool Catalog**: Generate a `tool-catalog.json` for agent parameter schema discovery.

## ✨ New Features (Feature Parity)
- [ ] **Paper Trading**: Add a `--paper` mode with a local SQLite/JSON engine for simulated trading.
- [ ] **Price Alerts**: Implement a real-time price alert system (`bittime alert add --above 100000`).
- [ ] **Advanced Trading**: Add support for `batch-orders` and `cancel-all`.
- [ ] **TUI Dashboard**: Build a real-time Terminal UI for monitoring markets and portfolio.

## 📦 Distribution & DX
- [ ] **NPM Package**: Add JS wrapper and publishing scripts for `npm install -g bittime-cli`.
- [ ] **Docker Image**: Create a multi-arch Dockerfile for containerized usage.
- [ ] **GitHub Actions**: Set up CI for cross-platform builds (Linux, macOS, Windows).
- [ ] **Manga/Entertainment**: (Optional) integrate the PubKit modules mentioned in previous sessions if relevant to this ecosystem.

## 📝 Documentation
- [ ] **API Reference**: Generate more detailed command examples for all 41 commands.
- [ ] **Video Demo**: Create an SVG/GIF terminal recording of the shell in action.
