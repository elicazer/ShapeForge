import { useEffect, useRef } from 'react';
import * as THREE from 'three';

interface MeshViewerProps {
    meshData: {
        vertices: number[];
        normals: number[];
        indices: number[];
    };
    wireframe: boolean;
    color?: string;
}

function MeshViewer({ meshData, wireframe, color = '#ff8800' }: MeshViewerProps) {
    const meshRef = useRef<THREE.Mesh>(null);

    useEffect(() => {
        if (!meshRef.current || !meshData) return;

        const geometry = new THREE.BufferGeometry();

        // Set vertices
        const vertices = new Float32Array(meshData.vertices);
        geometry.setAttribute('position', new THREE.BufferAttribute(vertices, 3));

        // Set normals
        if (meshData.normals && meshData.normals.length > 0) {
            const normals = new Float32Array(meshData.normals);
            geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
        } else {
            geometry.computeVertexNormals();
        }

        // Set indices
        if (meshData.indices && meshData.indices.length > 0) {
            const indices = new Uint32Array(meshData.indices);
            geometry.setIndex(new THREE.BufferAttribute(indices, 1));
        }

        geometry.computeBoundingSphere();

        meshRef.current.geometry.dispose();
        meshRef.current.geometry = geometry;
    }, [meshData]);

    return (
        <mesh ref={meshRef} position={[0, 10, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <bufferGeometry />
            <meshStandardMaterial
                color={color}
                wireframe={wireframe}
                metalness={0.3}
                roughness={0.7}
                side={THREE.DoubleSide}
            />
        </mesh>
    );
}

export default MeshViewer;
