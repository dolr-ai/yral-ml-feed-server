# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Development
```bash
# Build Rust server for development
cargo build --release

# Build for production (Linux musl target)
cargo build --release --target x86_64-unknown-linux-musl

# Run Rust server locally
cargo run --bin ml-feed-rust

# Setup Python ML service
cd python_src
pip install -r requirements.txt
./setup.sh
python3 ./recommendation_service/feed_rec_server.py
```

### Linting (currently disabled in CI)
```bash
# Format Rust code
cargo fmt --check

# Lint Rust code
cargo clippy --no-deps --all-features --release -- -Dwarnings
```

### Deployment
```bash
# Deploy to staging (auto-deploys from feature branches)
# Uses fly-staging.toml

# Deploy to production (auto-deploys from main branch)
# Uses fly-prod.toml
```

## Architecture

### Hybrid Rust/Python System
- **Rust server** (`rust_src/`): Main HTTP/gRPC server handling API requests
- **Python service** (`python_src/`): ML recommendation algorithms via gRPC

### Key Components
- `rust_src/main.rs`: Dual HTTP/gRPC server with content-type routing
- `rust_src/feed/`: Feed recommendation HTTP endpoints
- `rust_src/grpc_services.rs`: gRPC service implementations
- `python_src/recommendation_service/`: ML recommendation algorithms (v0, v2, v3)

### Protocol Communication
- Rust ↔ Python communication via gRPC using Protocol Buffers
- Protocol definitions in `contracts/video_recommendation.proto`
- Build script (`build.rs`) generates Rust bindings from protobuf + Candid files

### Feed Architecture
The system implements a sophisticated ML recommendation pipeline:
- **Exploitation** (0-90% for engaged users): Score-aware and recency-aware recommendations using vector search
- **Exploration** (10-100% for new users): Popular videos and random recent content
- **Content Filtering**: Separate paths for Clean, NSFW, and Mixed content
- **Caching**: Multi-layer caching with global, user-specific, and content-type specific strategies

### Internet Computer Integration
- User management through IC canisters
- DID files in `did/` directory
- Candid integration for type-safe canister communication

## Project Structure

```
rust_src/           # Main Rust server
  ├── feed/         # HTTP API endpoints
  ├── canister/     # IC canister integration
  └── grpc_services.rs # gRPC service implementations

python_src/         # ML recommendation service
  ├── recommendation_service/ # Core ML algorithms
  └── utils/        # Utility functions

contracts/          # Protocol buffer definitions
did/               # Internet Computer DID files
```

## Development Notes

### Dependencies
- **Rust**: Axum (web), Tonic (gRPC), Tokio (async), Candid (IC integration)
- **Python**: gRPC, BigQuery, Upstash Vector, Protocol Buffers
- **External**: Sentry (monitoring), Fly.io (deployment)

### Content Safety
The system handles three content types with separate recommendation paths:
- **Clean**: Family-friendly content
- **NSFW**: Adult content  
- **Mixed**: Combined content types

### Caching Strategy
- Upstash Vector for ML recommendations
- Multi-layer caching for performance optimization
- Content-type specific cache invalidation

### Monitoring
- Sentry integration for error tracking and performance monitoring
- Comprehensive logging throughout the recommendation pipeline