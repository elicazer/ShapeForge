import { useState } from 'react';
import ChatPanel from './components/ChatPanel';
import ViewportPanel from './components/ViewportPanel';
import Toolbar from './components/Toolbar';
import SettingsModal from './components/SettingsModal';
import './App.css';

export type Theme = 'dark' | 'light' | 'midnight';

export interface Message {
    role: 'user' | 'assistant';
    content: string;
}

export interface MeshData {
    vertices: number[];
    normals: number[];
    indices: number[];
}

export interface StructuredResponse {
    chat_message: string;
    generated_code: string | null;
    mesh_data: MeshData | null;
}

export interface ConversationState {
    messages: Message[];
    generated_code: string | null;
    mesh_data: MeshData | null;
}

function App() {
    const [messages, setMessages] = useState<Message[]>([]);
    const [generatedCode, setGeneratedCode] = useState<string | null>(null);
    const [meshData, setMeshData] = useState<string | null>(null);
    const [settingsOpen, setSettingsOpen] = useState(false);
    const [theme, setTheme] = useState<Theme>('dark');

    const handleNew = () => {
        if (confirm('Start a new conversation? This will clear the current chat.')) {
            setMessages([]);
            setGeneratedCode(null);
            setMeshData(null);
        }
    };

    const handleStructuredResponse = (response: StructuredResponse) => {
        if (response.generated_code) setGeneratedCode(response.generated_code);
        if (response.mesh_data) setMeshData(JSON.stringify(response.mesh_data));
    };

    const handleRestore = (state: ConversationState) => {
        setMessages(state.messages);
        setGeneratedCode(state.generated_code);
        setMeshData(state.mesh_data ? JSON.stringify(state.mesh_data) : null);
    };

    return (
        <div className={`app theme-${theme}`}>
            <Toolbar
                onSettingsClick={() => setSettingsOpen(true)}
                onNew={handleNew}
                messages={messages}
                generatedCode={generatedCode}
                meshData={meshData}
                onRestore={handleRestore}
                theme={theme}
                onThemeChange={setTheme}
            />
            <div className="main-content">
                <ChatPanel
                    onMeshGenerated={setMeshData}
                    messages={messages}
                    setMessages={setMessages}
                    onStructuredResponse={handleStructuredResponse}
                />
                <ViewportPanel meshData={meshData} theme={theme} />
            </div>
            <SettingsModal
                isOpen={settingsOpen}
                onClose={() => setSettingsOpen(false)}
            />
        </div>
    );
}

export default App;
