# Development Guide

## Prerequisites

- Node.js 18+
- Rust 1.70+
- .NET 8.0 SDK
- PicoGK library

## Setup

```bash
# Install frontend dependencies
npm install

# Build PicoGK bridge
cd picogk-bridge
dotnet build
cd ..

# Run development server
npm run tauri dev
```

## Project Structure

```
ai-cad-app/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── App.tsx            # Main app
│   └── main.tsx           # Entry point
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri app
│   │   ├── llm.rs         # LLM integration
│   │   └── picogk.rs      # PicoGK bridge
│   └── Cargo.toml
├── picogk-bridge/         # C# PicoGK wrapper
│   ├── Program.cs
│   └── picogk-bridge.csproj
└── docs/                  # Documentation
```

## Adding LLM Support

Edit `src-tauri/src/llm.rs`:

```rust
// Add your API key to environment or config
let api_key = std::env::var("OPENAI_API_KEY")?;

// Make API request
let response = reqwest::Client::new()
    .post("https://api.openai.com/v1/chat/completions")
    .header("Authorization", format!("Bearer {}", api_key))
    .json(&request_body)
    .send()
    .await?;
```

## Testing PicoGK Integration

```bash
# Test C# bridge directly
cd picogk-bridge
dotnet run test-code.cs
```

## Building for Production

```bash
npm run tauri build
```

Output in `src-tauri/target/release/bundle/`
