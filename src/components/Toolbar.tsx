import { invoke } from '@tauri-apps/api/tauri';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile, writeBinaryFile } from '@tauri-apps/api/fs';
import { Message, ConversationState, Theme } from '../App';

interface ToolbarProps {
    onSettingsClick: () => void;
    onNew: () => void;
    messages: Message[];
    generatedCode: string | null;
    meshData: string | null;
    onRestore: (state: ConversationState) => void;
    theme: Theme;
    onThemeChange: (theme: Theme) => void;
}

const themeLabels: Record<Theme, string> = { dark: 'Dark', light: 'Light', midnight: 'Midnight' };
const themeOrder: Theme[] = ['dark', 'light', 'midnight'];

function Toolbar(props: ToolbarProps) {
    const { onSettingsClick, onNew, messages, generatedCode, meshData, onRestore, theme, onThemeChange } = props;

    const handleOpen = async () => {
        const selected = await open({ filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (selected && typeof selected === 'string') {
            const content = await readTextFile(selected);
            try {
                const state = JSON.parse(content) as ConversationState;
                if (!state.messages || !Array.isArray(state.messages)) throw new Error('Invalid');
                onRestore(state);
            } catch { alert('Invalid file format.'); }
        }
    };

    const handleSave = async () => {
        const filePath = await save({ filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (filePath) {
            await writeTextFile(filePath, JSON.stringify({
                messages, generated_code: generatedCode,
                mesh_data: meshData ? JSON.parse(meshData) : null,
            }, null, 2));
        }
    };

    const handleExport = async () => {
        try {
            const filePath = await save({ filters: [{ name: 'STL', extensions: ['stl'] }], defaultPath: 'model.stl' });
            if (filePath) await invoke('export_stl', { path: filePath });
        } catch (error) { alert('Export failed: ' + error); }
    };

    const handleScreenshot = async () => {
        const canvas = document.querySelector('.viewport-panel canvas') as HTMLCanvasElement | null;
        if (!canvas) return;
        try {
            const dataUrl = canvas.toDataURL('image/png');
            const filePath = await save({ filters: [{ name: 'PNG', extensions: ['png'] }], defaultPath: 'screenshot.png' });
            if (filePath) {
                const base64 = dataUrl.split(',')[1];
                const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
                await writeBinaryFile(filePath, bytes);
            }
        } catch (error) { alert('Screenshot failed: ' + error); }
    };

    const cycleTheme = () => {
        const idx = themeOrder.indexOf(theme);
        onThemeChange(themeOrder[(idx + 1) % themeOrder.length]);
    };

    return (
        <div className="toolbar" role="toolbar">
            <div className="toolbar-group">
                <button onClick={onNew} title="New conversation">New</button>
                <button onClick={handleOpen} title="Open conversation">Open</button>
                <button onClick={handleSave} title="Save conversation">Save</button>
            </div>
            <div className="toolbar-separator" />
            <div className="toolbar-group">
                <button onClick={handleExport} title="Export STL" disabled={!meshData}>Export STL</button>
                <button onClick={handleScreenshot} title="Screenshot" disabled={!meshData}>Screenshot</button>
            </div>
            <div className="toolbar-spacer" />
            <div className="toolbar-group">
                <button onClick={cycleTheme} title={'Theme: ' + themeLabels[theme]}>{themeLabels[theme]}</button>
                <button onClick={onSettingsClick} title="Settings">Settings</button>
            </div>
        </div>
    );
}

export default Toolbar;
