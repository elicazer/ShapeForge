import { Canvas } from '@react-three/fiber';
import { OrbitControls, Grid, PerspectiveCamera } from '@react-three/drei';
import { useState } from 'react';
import MeshViewer from './MeshViewer';
import { Theme } from '../App';
import './ViewportPanel.css';

interface ViewportPanelProps {
    meshData: string | null;
    theme: Theme;
}

interface ParsedMeshData {
    vertices: number[];
    normals: number[];
    indices: number[];
}

const themeConfig = {
    dark: {
        bg: '#1a1a1a',
        gridCell: '#333',
        gridSection: '#555',
        meshColor: '#ff8800',
        ambient: 0.6,
    },
    light: {
        bg: '#e8e8e8',
        gridCell: '#ccc',
        gridSection: '#aaa',
        meshColor: '#e07000',
        ambient: 0.8,
    },
    midnight: {
        bg: '#0a0a14',
        gridCell: '#1a1a2e',
        gridSection: '#2a2a4e',
        meshColor: '#6a9fff',
        ambient: 0.5,
    },
};

function ViewportPanel({ meshData, theme }: ViewportPanelProps) {
    const [showGrid, setShowGrid] = useState(true);
    const [wireframe, setWireframe] = useState(false);
    const colors = themeConfig[theme];

    let parsedMesh: ParsedMeshData | null = null;
    if (meshData) {
        try {
            parsedMesh = JSON.parse(meshData);
        } catch (e) {
            console.error('Failed to parse mesh data:', e);
        }
    }

    const hasMesh = parsedMesh && parsedMesh.vertices && parsedMesh.vertices.length > 0;

    return (
        <div className="viewport-panel" style={{ background: colors.bg }}>
            <Canvas gl={{ preserveDrawingBuffer: true }}>
                <PerspectiveCamera makeDefault position={[40, 40, 40]} fov={50} far={5000} />
                <ambientLight intensity={colors.ambient} />
                <directionalLight position={[10, 10, 5]} intensity={0.8} />
                <directionalLight position={[-10, -10, -5]} intensity={0.3} />
                <pointLight position={[0, 10, 0]} intensity={0.5} />
                <color attach="background" args={[colors.bg]} />

                {showGrid && (
                    <Grid
                        args={[100, 100]}
                        cellSize={5}
                        cellThickness={0.5}
                        cellColor={colors.gridCell}
                        sectionSize={25}
                        sectionThickness={1}
                        sectionColor={colors.gridSection}
                        fadeDistance={1000}
                        fadeStrength={1}
                        position={[0, 0, 0]}
                    />
                )}

                <OrbitControls
                    enableDamping
                    dampingFactor={0.05}
                    minDistance={5}
                    maxDistance={2000}
                    target={[0, 10, 0]}
                />

                {hasMesh && parsedMesh && (
                    <MeshViewer meshData={parsedMesh} wireframe={wireframe} color={colors.meshColor} />
                )}
            </Canvas>

            <div className="viewport-controls">
                <button
                    onClick={() => setWireframe(!wireframe)}
                    className={wireframe ? 'active' : ''}
                    title="Toggle wireframe mode"
                >
                    Wireframe
                </button>
                <button
                    onClick={() => setShowGrid(!showGrid)}
                    className={showGrid ? 'active' : ''}
                    title="Toggle grid"
                >
                    Grid
                </button>
            </div>

            {!hasMesh && (
                <div className="viewport-message">
                    <p>No model generated yet</p>
                    <p className="hint">Ask the AI to create a 3D part in the chat</p>
                </div>
            )}

            {hasMesh && parsedMesh && (
                <div className="viewport-info">
                    <p>✓ Model loaded</p>
                    <p className="hint">{parsedMesh.vertices.length / 3} vertices</p>
                </div>
            )}
        </div>
    );
}

export default ViewportPanel;
