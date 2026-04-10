# ShapeForge

AI-powered 3D CAD desktop app. Describe a part in natural language (or drop in an image), and ShapeForge generates CadQuery code, executes it, and renders the 3D model in real time.

## Features

- Natural language to 3D model generation
- Image-to-3D: attach a photo/sketch and get a 3D model
- Multi-provider LLM support: Gemini, OpenAI, Anthropic, AWS Bedrock
- Model selection per provider
- 3D viewport with orbit controls, wireframe, and grid toggle
- Dark / Light / Midnight themes
- Export to STL
- Viewport screenshots
- Conversation save/load

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- [Python 3](https://www.python.org/) with CadQuery: `pip install cadquery`

## Getting Started

```bash
npm install
npm run tauri dev
```

On first launch, open Settings and configure your LLM provider + API key.

## Tech Stack

- Frontend: React + TypeScript + Three.js (via react-three-fiber)
- Backend: Rust (Tauri v1)
- CAD Engine: CadQuery (Python)
- LLM Providers: Gemini, OpenAI, Anthropic, AWS Bedrock

## License

MIT
