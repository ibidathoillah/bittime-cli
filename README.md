# bittime-cli

The unofficial, fast, and feature-rich command-line interface for [Bittime](https://bittime.com) — a cryptocurrency exchange.

Track markets, execute trades, manage your portfolio, and stream real-time data — all from your terminal.

- 📊 **Comprehensive Market Data** — Ticker, order book, trades, aggregated trades, exchange info
- 💰 **Full Account Management** — Balances, open orders, order history, trade history
- 🛠️ **Powerful Trading** — Place buy/sell orders (LIMIT/MARKET), cancel orders
- 💳 **OTC Banking** — IDR fiat deposit/withdrawal via Indonesian banks (BCA, BNI, Mandiri, etc.)
- 🔥 **Real-Time WebSocket** — Live depth, order updates, balance updates
- 🔐 **Secure Authentication** — HMAC-SHA256 API signing with multiple credential resolution
- 📋 **Flexible Output** — Human-friendly tables or machine-readable JSON
- 🖥️ **Interactive Shell** — Built-in REPL for exploratory usage
- ⚡ **Blazing Fast** — Built with Rust for maximum performance and safety

## Installation

### From Source (requires [Rust](https://rustup.rs/))

```bash
git clone https://github.com/ibidathoillah/bittime-cli.git
cd bittime-cli
cargo install --path .
```

### From Cargo (Crates.io)

```bash
cargo install bittime-cli
```

## Quick Start

### 1. Check Market Data (No API Key Needed)

```bash
bittime market ping
bittime market server-time
bittime market ticker USDTIDR
bittime market orderbook BTCUSDT -l 10
bittime market price USDTIDR
bittime market trades BTCUSDT
```

### 2. Configure API Credentials

```bash
bittime auth set --api-key YOUR_API_KEY --api-secret YOUR_API_SECRET
```

Or use environment variables:

```bash
export BITTIME_API_KEY=your_api_key
export BITTIME_API_SECRET=your_api_secret
```

Credentials are resolved in this priority order:
1. CLI flags (`--api-key`, `--api-secret`)
2. Environment variables (`BITTIME_API_KEY`, `BITTIME_API_SECRET`)
3. Config file (`~/.config/bittime/config.toml` with 0600 permissions)

### 3. View Account (Requires API Key)

```bash
bittime account balance
bittime account info
```

### 4. Start the Interactive Shell

```bash
bittime shell
```

## Commands

```
bittime [OPTIONS] <COMMAND>

Options:
  -o, --output <OUTPUT>          Output format: table or json [default: table]
      --api-key <API_KEY>        API key (overrides config and env var)
      --api-secret <API_SECRET>  API secret (overrides config and env var)
  -v, --verbose                  Enable verbose output
      --host <HOST>              Override API host URL
  -h, --help                     Print help
  -V, --version                  Print version
```

### Market Data (Public API)
```bash
bittime market ping
bittime market server-time
bittime market exchange-info
bittime market ticker <PAIR>
bittime market ticker-all
bittime market price <PAIR>
bittime market book-ticker <PAIR>
bittime market orderbook <PAIR> [-l LIMIT]
bittime market trades <PAIR> [-l LIMIT]
bittime market historical-trades <PAIR>
bittime market agg-trades <PAIR>
```

### Account (Private API)
```bash
bittime account info
bittime account balance
bittime account info-v2
bittime account assets <COIN>
bittime account trades <PAIR>
bittime account trades-v2 <PAIR> [--from-id ID]
bittime account trade-history <PAIR>
```

### Trading (Private API)
```bash
bittime trade buy <PAIR> -t LIMIT -p <PRICE> -q <QTY>
bittime trade sell <PAIR> -t LIMIT -p <PRICE> -q <QTY>
bittime trade buy <PAIR> -t MARKET -q <QTY>
bittime trade cancel <PAIR> --order-id <ID>
bittime trade query <PAIR> --order-id <ID>
bittime trade open-orders <PAIR>
bittime trade all-orders <PAIR>
bittime trade pending-orders <PAIR>
bittime trade book-orders <PAIR>
bittime trade convert <PAIR>
```

### Funding (Private API)
```bash
bittime funding withdraw --coin USDT --amount 100 --address 0x... --chain ERC20
bittime funding withdraw-history
bittime funding deposit-history
bittime funding otc-va-code --bank-id 8
bittime funding otc-deposit-history
bittime funding otc-withdraw --bank-name BCA --account-name "NAME" --bank-number 123 --amount 50000
bittime funding otc-withdraw-history
```

### WebSocket Streaming
```bash
bittime ws depth <PAIR>       # Market depth stream
bittime ws orders             # Private order updates
bittime ws balances           # Private balance updates
```

### Authentication Management
```bash
bittime auth set --api-key KEY --api-secret SECRET
bittime auth show
bittime auth test
bittime auth reset
```

### Interactive Shell
```bash
bittime shell
```

## Output Formats

**Table mode** (default) — human-friendly aligned tables:
```bash
bittime market ticker USDTIDR
```

**JSON mode** — for scripting and automation:
```bash
bittime -o json market ticker USDTIDR
```

## Architecture

Built with modern Rust:
- **clap** — powerful derive-based CLI parsing
- **tokio** — async runtime for non-blocking I/O
- **tokio-tungstenite** — WebSocket client for real-time streams
- **reqwest** — HTTP client for REST API calls
- **serde** — robust serialization/deserialization
- **comfy-table** — beautiful terminal tables

## API Documentation

- Bittime REST API: https://bittime-docs.github.io
- Base endpoint: `https://openapi.bittime.com`
- WebSocket market: `wss://ws.bittime.com/market/ws`
- WebSocket user: `wss://wsapi.bittime.com`

## License

MIT

## Disclaimer

This is an unofficial CLI and is not affiliated with or endorsed by Bittime. Use at your own risk. Cryptocurrency trading involves significant risk of loss.
