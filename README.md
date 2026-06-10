<p align="center">
  <img src="src-tauri/icons/icon.png" width="120" alt="ShapeForge logo" />
</p>

<h1 align="center">ShapeForge</h1>

<p align="center">
  AI-powered desktop CAD — describe a part, get a 3D model.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-v1-blue" alt="Tauri" />
  <img src="https://img.shields.io/badge/React-18-61dafb" alt="React" />
  <img src="https://img.shields.io/badge/Rust-🦀-orange" alt="Rust" />
  <img src="https://img.shields.io/badge/CadQuery-Python-green" alt="CadQuery" />
  <img src="https://img.shields.io/badge/license-MIT-lightgrey" alt="MIT" />
</p>

---

ShapeForge turns natural language into 3D geometry. Type a description like *"a phone stand with a cable slot"* or drop in a photo of an object, and ShapeForge generates CadQuery code, executes it, and renders the result in an interactive 3D viewport — all locally on your machine.

## ✨ Features

| Feature | Description |
|---|---|
| 🗣️ Text to 3D | Describe a part in plain English and get geometry |
| 📷 Image to 3D | Attach a photo or sketch — the AI analyzes it, then generates a model |
| 🔄 Conversational | Refine your model through follow-up messages |
| 🤖 Multi-provider | Gemini, OpenAI, Anthropic, and AWS Bedrock with model selection |
| 🎨 Themes | Dark, Light, and Midnight viewport themes |
| 📸 Screenshots | One-click viewport capture to PNG |
| 💾 Export | Save models as STL for 3D printing or further CAD work |
| 📂 Sessions | Save and load conversation history as JSON |

## 🚀 Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://rustup.rs/) (latest stable)
- Python 3 with CadQuery:
  ```bash
  pip install cadquery
  ```

### Run

```bash
npm install
npm run tauri dev
```

Open **Settings** from the toolbar, pick your LLM provider, paste your API key, and start describing parts.

## 🏗️ Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  React UI   │────▶│  Tauri/Rust  │────▶│  LLM API    │
│  Three.js   │◀────│  Backend     │◀────│  (Gemini,   │
│  Viewport   │     │              │     │   OpenAI,   │
└─────────────┘     │  CadQuery    │     │   Claude,   │
                    │  (Python)    │     │   Bedrock)  │
                    └──────────────┘     └─────────────┘
```

1. User describes a part (text or image)
2. Rust backend sends the prompt to the selected LLM
3. LLM returns CadQuery Python code
4. Backend executes the code via Python, producing an STL mesh
5. Frontend renders the mesh in a Three.js viewport

For images, a two-step pipeline is used: the image is first analyzed to extract shape descriptions, then that description drives the code generation.

## 🤖 Supported Providers

| Provider | Models | Auth |
|---|---|---|
| **Gemini** | Gemini 3.1 Pro, 3.5 Flash, 3 Flash, 3.1 Flash-Lite, 2.5 Pro/Flash | API Key |
| **OpenAI** | GPT-5.5, GPT-5.5 Pro, GPT-5.4, GPT-5.2, GPT-5.1, GPT-5 Mini/Nano | API Key |
| **Anthropic** | Claude Fable 5, Opus 4.8, Opus 4.7, Sonnet 4.6, Haiku 4.5 | API Key |
| **AWS Bedrock** | Claude Fable 5, Opus 4.8, Sonnet 4.6, Opus 4.6, Haiku 4.5 + more | Bedrock API Key + Region |

## 📁 Project Structure

```
├── src/                    # React frontend
│   ├── components/         # ChatPanel, ViewportPanel, Toolbar, etc.
│   ├── App.tsx             # Main app with theme + state management
│   └── styles.css          # Global styles
├── src-tauri/
│   ├── src/
│   │   ├── main.rs         # Tauri commands (generate, export, credentials)
│   │   ├── llm.rs          # LLM provider routing + API calls
│   │   └── cadquery.rs     # Python execution + STL parsing
│   └── tauri.conf.json     # Tauri config
├── index.html
└── package.json
```

## 🛠️ Development

```bash
# Run in dev mode
npm run tauri dev

# Run Rust tests
cd src-tauri && cargo test

# Build for production
npm run tauri build
```

## 📄 License

MIT
