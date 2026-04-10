# Architecture

## Overview

```
┌─────────────────────────────────────────────┐
│           Tauri Desktop App                 │
├─────────────────────────────────────────────┤
│  Frontend (React + Three.js)                │
│  ├─ ChatPanel: User input & history         │
│  ├─ ViewportPanel: 3D preview               │
│  └─ Toolbar: File operations                │
├─────────────────────────────────────────────┤
│  Backend (Rust)                             │
│  ├─ LLM Module: API calls                   │
│  ├─ PicoGK Bridge: Subprocess comm          │
│  └─ File I/O: STL/3MF export                │
└─────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
    ┌─────────┐        ┌──────────────┐
    │   LLM   │        │ PicoGK (C#)  │
    │  APIs   │        │  Subprocess  │
    └─────────┘        └──────────────┘
```

## Data Flow

1. User types prompt in ChatPanel
2. Frontend sends to Rust backend via Tauri IPC
3. Backend calls LLM API with prompt + context
4. LLM returns PicoGK C# code
5. Backend writes code to temp file
6. Backend spawns PicoGK subprocess
7. PicoGK executes code, returns mesh JSON
8. Backend parses mesh data
9. Frontend receives mesh, renders in Three.js

## Key Components

### Frontend (React)
- **ChatPanel**: Message history, input field, LLM streaming
- **ViewportPanel**: Three.js canvas, orbit controls, grid
- **Toolbar**: New/Open/Save/Export/Settings buttons

### Backend (Rust)
- **llm.rs**: OpenAI/Anthropic/Ollama API integration
- **picogk.rs**: Subprocess management, IPC with C# bridge
- **main.rs**: Tauri commands, state management

### PicoGK Bridge (C#)
- Standalone executable
- Receives code via stdin or file
- Executes in sandboxed environment
- Returns mesh data as JSON

## Security

- PicoGK runs in subprocess (isolated)
- Code execution timeout (30s default)
- Resource limits (memory, CPU)
- No file system access from generated code
