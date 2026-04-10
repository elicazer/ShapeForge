import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import './SettingsModal.css';

interface SettingsModalProps {
    isOpen: boolean;
    onClose: () => void;
}

const geminiModels = [
    { id: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash (fast)' },
    { id: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro' },
    { id: 'gemini-3-flash-preview', label: 'Gemini 3 Flash (preview)' },
    { id: 'gemini-3-pro-preview', label: 'Gemini 3 Pro (preview)' },
];

const bedrockModels = [
    { id: 'global.anthropic.claude-sonnet-4-6', label: 'Claude Sonnet 4.6 (latest)' },
    { id: 'global.anthropic.claude-opus-4-6-v1', label: 'Claude Opus 4.6' },
    { id: 'global.anthropic.claude-sonnet-4-5-20250929-v1:0', label: 'Claude Sonnet 4.5' },
    { id: 'global.anthropic.claude-opus-4-5-20251101-v1:0', label: 'Claude Opus 4.5' },
    { id: 'global.anthropic.claude-opus-4-1-20250805-v1:0', label: 'Claude Opus 4.1' },
    { id: 'global.anthropic.claude-sonnet-4-20250514-v1:0', label: 'Claude Sonnet 4' },
    { id: 'global.anthropic.claude-haiku-4-5-20251001-v1:0', label: 'Claude Haiku 4.5 (fast)' },
];

const openaiModels = [
    { id: 'gpt-5.2', label: 'GPT-5.2 (flagship)' },
    { id: 'gpt-5.1', label: 'GPT-5.1' },
    { id: 'gpt-5', label: 'GPT-5' },
    { id: 'gpt-5-mini', label: 'GPT-5 Mini (fast)' },
    { id: 'gpt-4.1', label: 'GPT-4.1' },
    { id: 'gpt-4.1-mini', label: 'GPT-4.1 Mini' },
    { id: 'o3', label: 'o3 (reasoning)' },
    { id: 'o4-mini', label: 'o4-mini (reasoning, fast)' },
];

const anthropicModels = [
    { id: 'claude-opus-4-6', label: 'Claude Opus 4.6 (most capable)' },
    { id: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6' },
    { id: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5 (fast)' },
];

type Provider = 'gemini' | 'bedrock' | 'openai' | 'anthropic';

function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
    const [provider, setProvider] = useState<Provider>('gemini');
    const [apiKey, setApiKey] = useState('');
    const [bedrockApiKey, setBedrockApiKey] = useState('');
    const [openaiApiKey, setOpenaiApiKey] = useState('');
    const [anthropicApiKey, setAnthropicApiKey] = useState('');
    const [awsRegion, setAwsRegion] = useState('us-east-1');
    const [geminiModel, setGeminiModel] = useState(geminiModels[0].id);
    const [bedrockModel, setBedrockModel] = useState(bedrockModels[0].id);
    const [openaiModel, setOpenaiModel] = useState(openaiModels[0].id);
    const [anthropicModel, setAnthropicModel] = useState(anthropicModels[0].id);
    const [error, setError] = useState('');

    if (!isOpen) return null;

    const handleSave = async () => {
        setError('');
        if (provider === 'bedrock') {
            const missing: string[] = [];
            if (!bedrockApiKey.trim()) missing.push('Bedrock API Key');
            if (!awsRegion.trim()) missing.push('Region');
            if (missing.length > 0) {
                setError(`Missing required fields: ${missing.join(', ')}`);
                return;
            }
        }
        const apiKeyMap: Record<Provider, string> = {
            gemini: apiKey,
            bedrock: bedrockApiKey,
            openai: openaiApiKey,
            anthropic: anthropicApiKey,
        };
        const modelMap: Record<Provider, string> = {
            gemini: geminiModel,
            bedrock: bedrockModel,
            openai: openaiModel,
            anthropic: anthropicModel,
        };
        try {
            await invoke('set_credentials', {
                provider,
                apiKey: apiKeyMap[provider],
                region: awsRegion,
                model: modelMap[provider],
            });
            onClose();
        } catch (err) {
            alert(`Failed to save credentials: ${err}`);
        }
    };

    const modelsMap: Record<Provider, typeof geminiModels> = {
        gemini: geminiModels,
        bedrock: bedrockModels,
        openai: openaiModels,
        anthropic: anthropicModels,
    };
    const selectedModelMap: Record<Provider, string> = {
        gemini: geminiModel,
        bedrock: bedrockModel,
        openai: openaiModel,
        anthropic: anthropicModel,
    };
    const setSelectedModelMap: Record<Provider, (v: string) => void> = {
        gemini: setGeminiModel,
        bedrock: setBedrockModel,
        openai: setOpenaiModel,
        anthropic: setAnthropicModel,
    };
    const models = modelsMap[provider];
    const selectedModel = selectedModelMap[provider];
    const setSelectedModel = setSelectedModelMap[provider];

    return (
        <div className="modal-overlay" onClick={onClose}>
            <div className="modal-content" onClick={e => e.stopPropagation()}>
                <h2>LLM Provider Settings</h2>

                <div className="form-group">
                    <label>Provider</label>
                    <select value={provider} onChange={e => { setProvider(e.target.value as Provider); setError(''); }}>
                        <option value="gemini">Gemini</option>
                        <option value="bedrock">Bedrock</option>
                        <option value="openai">OpenAI</option>
                        <option value="anthropic">Anthropic</option>
                    </select>
                </div>

                <div className="form-group">
                    <label>Model</label>
                    <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)}>
                        {models.map(m => (
                            <option key={m.id} value={m.id}>{m.label}</option>
                        ))}
                    </select>
                </div>

                {provider === 'gemini' && (
                    <div className="form-group">
                        <label>Gemini API Key</label>
                        <input type="password" value={apiKey} onChange={e => { setApiKey(e.target.value); setError(''); }} placeholder="Enter your Gemini API key" />
                    </div>
                )}

                {provider === 'bedrock' && (
                    <>
                        <div className="form-group">
                            <label>Bedrock API Key</label>
                            <input type="password" value={bedrockApiKey} onChange={e => { setBedrockApiKey(e.target.value); setError(''); }} placeholder="Enter your Bedrock API key" />
                        </div>
                        <div className="form-group">
                            <label>Region</label>
                            <input type="text" value={awsRegion} onChange={e => { setAwsRegion(e.target.value); setError(''); }} placeholder="us-east-1" />
                        </div>
                    </>
                )}

                {provider === 'openai' && (
                    <div className="form-group">
                        <label>OpenAI API Key</label>
                        <input type="password" value={openaiApiKey} onChange={e => { setOpenaiApiKey(e.target.value); setError(''); }} placeholder="Enter your OpenAI API key" />
                    </div>
                )}

                {provider === 'anthropic' && (
                    <div className="form-group">
                        <label>Anthropic API Key</label>
                        <input type="password" value={anthropicApiKey} onChange={e => { setAnthropicApiKey(e.target.value); setError(''); }} placeholder="Enter your Anthropic API key" />
                    </div>
                )}

                {error && <p className="error-text" style={{ color: 'red', margin: '8px 0' }}>{error}</p>}

                <div className="modal-actions">
                    <button onClick={onClose} className="btn-secondary">Cancel</button>
                    <button onClick={handleSave} className="btn-primary">Save</button>
                </div>

                <div className="info-box">
                    <p><strong>Note:</strong> {
                        provider === 'gemini'
                            ? 'Your API key is stored locally and used only for Gemini API requests.'
                            : provider === 'bedrock'
                                ? 'Your Bedrock API key is stored locally and used only for Bedrock API requests.'
                                : provider === 'openai'
                                    ? 'Your API key is stored locally and used only for OpenAI API requests.'
                                    : 'Your API key is stored locally and used only for Anthropic API requests.'
                    }</p>
                </div>
            </div>
        </div>
    );
}

export default SettingsModal;
