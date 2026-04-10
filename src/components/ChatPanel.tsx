import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Message, StructuredResponse } from '../App';
import './ChatPanel.css';

interface ChatPanelProps {
    messages: Message[];
    setMessages: React.Dispatch<React.SetStateAction<Message[]>>;
    onStructuredResponse: (response: StructuredResponse) => void;
    onMeshGenerated: (meshData: string) => void;
}

function ChatPanel({ messages, setMessages, onStructuredResponse, onMeshGenerated }: ChatPanelProps) {
    const [input, setInput] = useState('');
    const [loading, setLoading] = useState(false);
    const [attachedImage, setAttachedImage] = useState<string | null>(null);
    const [imageMimeType, setImageMimeType] = useState<string>('image/png');
    const [imagePreview, setImagePreview] = useState<string | null>(null);
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const fileInputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages, loading]);

    useEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.style.height = 'auto';
            textareaRef.current.style.height = Math.min(textareaRef.current.scrollHeight, 120) + 'px';
        }
    }, [input]);

    const handleImageSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;
        const reader = new FileReader();
        reader.onload = () => {
            const dataUrl = reader.result as string;
            setImagePreview(dataUrl);
            // Extract base64 without the data:image/xxx;base64, prefix
            const base64 = dataUrl.split(',')[1];
            // Extract MIME type from data URL
            const mime = dataUrl.match(/^data:(image\/\w+);/)?.[1] || 'image/png';
            setImageMimeType(mime);
            setAttachedImage(base64);
        };
        reader.readAsDataURL(file);
        // Reset so same file can be re-selected
        e.target.value = '';
    };

    const clearImage = () => {
        setAttachedImage(null);
        setImagePreview(null);
    };

    const handleSend = async () => {
        if ((!input.trim() && !attachedImage) || loading) return;

        const promptText = input.trim() || 'Generate a 3D model based on this image';
        const userContent = imagePreview
            ? `[Image attached]\n${promptText}`
            : promptText;

        const userMessage: Message = { role: 'user', content: userContent };
        setMessages(prev => [...prev, userMessage]);

        const currentImage = attachedImage;
        const currentMime = imageMimeType;
        setInput('');
        clearImage();
        setLoading(true);

        try {
            const response = await invoke<StructuredResponse>('generate_geometry', {
                prompt: promptText,
                history: messages,
                imageBase64: currentImage || null,
                imageMimeType: currentImage ? currentMime : null,
            });

            let displayContent = response.chat_message;
            if (response.generated_code) {
                displayContent += '\n\n```python\n' + response.generated_code + '\n```';
            }

            setMessages(prev => [...prev, { role: 'assistant', content: displayContent }]);
            onStructuredResponse(response);

            if (response.mesh_data) {
                onMeshGenerated(JSON.stringify(response.mesh_data));
            }
        } catch (error) {
            setMessages(prev => [...prev, {
                role: 'assistant',
                content: `Error: ${error}`,
            }]);
        } finally {
            setLoading(false);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    const handleDrop = (e: React.DragEvent) => {
        e.preventDefault();
        const file = e.dataTransfer.files[0];
        if (file && file.type.startsWith('image/')) {
            const reader = new FileReader();
            reader.onload = () => {
                const dataUrl = reader.result as string;
                setImagePreview(dataUrl);
                const mime = dataUrl.match(/^data:(image\/\w+);/)?.[1] || 'image/png';
                setImageMimeType(mime);
                setAttachedImage(dataUrl.split(',')[1]);
            };
            reader.readAsDataURL(file);
        }
    };

    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
    };

    const renderMessage = (content: string) => {
        const parts = content.split(/(```[\s\S]*?```)/g);
        return parts.map((part, i) => {
            if (part.startsWith('```') && part.endsWith('```')) {
                const code = part.replace(/^```\w*\n?/, '').replace(/\n?```$/, '');
                return <pre key={i} className="code-block">{code}</pre>;
            }
            const trimmed = part.trim();
            if (!trimmed) return null;
            return <p key={i} className="msg-text">{trimmed}</p>;
        });
    };

    return (
        <div className="chat-panel" onDrop={handleDrop} onDragOver={handleDragOver}>
            <div className="messages">
                {messages.length === 0 && !loading && (
                    <div className="empty-chat">
                        <p className="empty-title">ShapeForge</p>
                        <p className="empty-hint">Describe a 3D part or drop an image</p>
                    </div>
                )}
                {messages.map((msg, i) => (
                    <div key={i} className={`message ${msg.role}`}>
                        <div className="msg-avatar">{msg.role === 'user' ? 'U' : 'AI'}</div>
                        <div className="msg-body">
                            {msg.role === 'assistant' ? renderMessage(msg.content) : (
                                <p className="msg-text">{msg.content}</p>
                            )}
                        </div>
                    </div>
                ))}
                {loading && (
                    <div className="message assistant">
                        <div className="msg-avatar">AI</div>
                        <div className="msg-body">
                            <div className="loading-dots">
                                <span></span><span></span><span></span>
                            </div>
                        </div>
                    </div>
                )}
                <div ref={messagesEndRef} />
            </div>

            {imagePreview && (
                <div className="image-preview">
                    <img src={imagePreview} alt="Attached" />
                    <button className="image-remove" onClick={clearImage} title="Remove image">✕</button>
                </div>
            )}

            <div className="input-area">
                <input
                    ref={fileInputRef}
                    type="file"
                    accept="image/*"
                    onChange={handleImageSelect}
                    style={{ display: 'none' }}
                />
                <button
                    className="attach-btn"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={loading}
                    title="Attach image"
                    aria-label="Attach image"
                >
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path d="M14 8l-5.3 5.3a3.5 3.5 0 01-5 0 3.5 3.5 0 010-5L9 3a2.3 2.3 0 013.3 0 2.3 2.3 0 010 3.3L7 11.6a1.2 1.2 0 01-1.7 0 1.2 1.2 0 010-1.7L10 5.2" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                </button>
                <textarea
                    ref={textareaRef}
                    value={input}
                    onChange={e => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder={attachedImage ? "Describe what to make from this image..." : "Describe the part you want to create..."}
                    rows={1}
                    disabled={loading}
                />
                <button onClick={handleSend} disabled={loading || (!input.trim() && !attachedImage)} aria-label="Send message">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path d="M1 8L14 1L8 15L6.5 9.5L1 8Z" fill="currentColor" />
                    </svg>
                </button>
            </div>
        </div>
    );
}

export default ChatPanel;
