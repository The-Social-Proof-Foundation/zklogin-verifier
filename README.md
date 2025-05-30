# zklogin-verifier

A rust server that verifies a zkLogin signature given bytes for transaction data or personal message for the MySocial blockchain network.

## 🚀 Features

- **Multi-Network Support**: Run against MainNet, TestNet, DevNet, or LocalNet
- **Configurable OAuth Providers**: Different networks support different OAuth providers
- **Automatic JWK Management**: Fetches and updates JSON Web Keys from OAuth providers
- **RESTful API**: Simple HTTP endpoints for signature verification
- **Health Monitoring**: Built-in health checks and configuration endpoints

## 📦 Installation

```bash
git clone https://github.com/The-Social-Proof-Foundation/zklogin-verifier
cd zklogin-verifier
cargo build --release
```

## 🔧 Usage

### Basic Commands

```bash
# Run on testnet (default) on port 8080
cargo run

# Run with specific network
cargo run -- --network mainnet
cargo run -- --network testnet  
cargo run -- --network devnet
cargo run -- --network localnet

# Run with custom port
cargo run -- --port 9000

# Custom JWK refresh interval (seconds)
cargo run -- --jwk-refresh-interval 1800

# Full configuration example
cargo run -- --network mainnet --port 8081 --jwk-refresh-interval 7200
```

### Command Line Options

```
-n, --network <NETWORK>          Network to connect to [default: testnet]
                                 Options: mainnet, testnet, devnet, localnet
-p, --port <PORT>               Port to bind the server to [default: 8080]
--jwk-refresh-interval <SECS>   JWK refresh interval in seconds [default: 3600]
-h, --help                      Print help
-V, --version                   Print version
```

## 🌐 Network Configurations

| Network   | RPC URL                              | OAuth Providers                                           | Faucet Available |
|-----------|--------------------------------------|-----------------------------------------------------------|------------------|
| MainNet   | https://mainnet.mysocial.network/rpc | Google, Facebook, Apple, Slack, Twitch, Kakao             | ❌                |
| TestNet   | https://testnet.mysocial.network/rpc | Google, Facebook, Apple, Slack, Twitch, Kakao             | ✅                |
| DevNet    | https://devnet.mysocial.network/rpc  | Google, Facebook                                          | ✅                |
| LocalNet  | http://localhost:9000/rpc            | Google                                                    | ✅                |

## 📡 API Endpoints

Once running, the server exposes these endpoints:

### Core Endpoints
- `GET /` - Ping endpoint (returns "Pong! zkLogin Verifier is running 🚀")
- `POST /verify` - Verify zkLogin signature
- `GET /health` - Health check endpoint
- `GET /config` - Show current network configuration

### Verify Signature

```bash
curl -X POST localhost:8080/verify \
  -H 'Content-Type: application/json' \
  -d '{
    "signature": "BQNNMTczMTgwODkxMjU5NTI0MjE3MzYzNDIyNjM3MTc5MzI3MTk0Mzc3MTc4NDQyODI0MTAxODc5NTc5ODQ3NTE5Mzk5NDI4OTgyNTEyNTBNMTEzNzM5NjY2NDU0NjkxMjI1ODIwNzQwODIyOTU5ODUzODgyNTg4NDA2ODE2MTgyNjg1OTM5NzY2OTczMjU4OTIyODA5MTU2ODEyMDcBMQMCTDU5Mzk4NzExNDczNDg4MzQ5OTczNjE3MjAxMjIyMzg5ODAxNzcxNTIzMDMyNzQzMTEwNDcyNDk5MDU5NDIzODQ5MTU3Njg2OTA4OTVMNDUzMzU2ODI3MTEzNDc4NTI3ODczMTIzNDU3MDM2MTQ4MjY1MTk5Njc0MDc5MTg4ODI4NTg2NDk2Njg4NDAzMjcxNzA0OTgxMTcwOAJNMTA1NjQzODcyODUwNzE1NTU0Njk3NTM5OTA2NjE0MTA4NDAxMTg2MzU5MjU0NjY1OTcwMzcwMTgwNTg3NzAwNDEzNDc1MTg0NjEzNjhNMTI1OTczMjM1NDcyNzc1NzkxNDQ2OTg0OTYzNzIyNDI2MTUzNjgwODU4MDEzMTMzNDMxN...",
    "bytes": "AAACAAgBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQsU8fdKUlDMnFVYPgeFtADg5rCJANEXAQr4+jxBe3QCp9QBJ86BI1lV84+5Cn2J45ks2xIIHNjXiWpWzgSFu8ZhyTX2GgMAAAAAAAAgYEVR6yJl8ZVdDBBhqGJd4D8PcYVqP1ZqV7J3vQN9z0ZQ0AVNS..."
  }'
```

### Check Configuration

```bash
curl localhost:8080/config
```

Example response:
```json
{
  "name": "testnet",
  "rpc_url": "https://testnet.mysocial.network/rpc",
  "faucet_url": "https://testnet.mysocial.network/faucet",
  "jwk_providers": ["Google", "Facebook", "Apple", "Slack", "Twitch", "Kakao"]
}
```

## 🔧 Development

### Local Development
```bash
# Run on localnet for development
cargo run -- --network localnet --port 3000

# With faster JWK refresh for testing
cargo run -- --network localnet --jwk-refresh-interval 300
```

### Build for Production
```bash
cargo build --release
./target/release/zklogin-verifier --network mainnet --port 8080
```

## 🔐 OAuth Provider Configuration

The verifier automatically fetches and manages JSON Web Keys (JWKs) from supported OAuth providers:

- **Google**: `https://accounts.google.com` 
- **Facebook**: `https://www.facebook.com`
- **Apple**: `https://appleid.apple.com`
- **Slack**: `https://slack.com`
- **Twitch**: `https://id.twitch.tv/oauth2`
- **Kakao**: Kakao OAuth provider

JWKs are automatically refreshed every hour (configurable with `--jwk-refresh-interval`).

## 🏗️ Architecture

- **FastCrypto**: Cryptographic operations and zkLogin verification
- **Axum**: High-performance web server framework
- **Tokio**: Async runtime for concurrent JWK fetching
- **MySocial Integration**: Custom blockchain integration through mys-core

## 📄 License

Apache-2.0