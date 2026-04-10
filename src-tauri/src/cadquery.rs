use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Triangle mesh data compatible with Three.js BufferGeometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
}

/// Guard that removes a file when dropped, ensuring temp file cleanup.
struct TempFileGuard {
    path: std::path::PathBuf,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Parse a binary or ASCII STL file into MeshData.
///
/// Supports both binary STL (80-byte header + triangle records) and ASCII STL
/// (text-based `solid`/`endsolid` format).
pub fn parse_stl(stl_bytes: &[u8]) -> Result<MeshData, String> {
    if stl_bytes.is_empty() {
        return Err("Failed to parse STL file: file is empty".to_string());
    }

    // Heuristic: ASCII STL starts with "solid " followed by a name (not binary data).
    // Binary STL has an 80-byte header that may also start with "solid", so we check
    // if the content looks like valid ASCII STL by searching for "facet" keyword.
    if is_ascii_stl(stl_bytes) {
        parse_ascii_stl(stl_bytes)
    } else {
        parse_binary_stl(stl_bytes)
    }
}

/// Check if the STL data is ASCII format.
fn is_ascii_stl(data: &[u8]) -> bool {
    // ASCII STL starts with "solid" and contains "facet normal"
    if data.len() < 6 {
        return false;
    }
    let starts_with_solid = data.starts_with(b"solid");
    if !starts_with_solid {
        return false;
    }
    // Look for "facet" keyword to distinguish from binary STL with "solid" in header
    let text = String::from_utf8_lossy(data);
    text.contains("facet normal") || text.contains("facet\n")
}

/// Parse ASCII STL format.
fn parse_ascii_stl(stl_bytes: &[u8]) -> Result<MeshData, String> {
    let text = String::from_utf8_lossy(stl_bytes);
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_index: u32 = 0;
    let mut current_normal = [0.0f32; 3];

    for line in text.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("facet normal") {
            let parts: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|s| s.parse::<f32>().ok())
                .collect();
            if parts.len() == 3 {
                current_normal = [parts[0], parts[1], parts[2]];
            }
        } else if let Some(rest) = trimmed.strip_prefix("vertex") {
            let parts: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|s| s.parse::<f32>().ok())
                .collect();
            if parts.len() == 3 {
                vertices.extend_from_slice(&parts);
                normals.extend_from_slice(&current_normal);
                indices.push(vertex_index);
                vertex_index += 1;
            }
        }
    }

    let num_triangles = indices.len() / 3;
    if num_triangles == 0 {
        return Err("CadQuery produced no geometry. Check your script.".to_string());
    }

    Ok(MeshData {
        vertices,
        normals,
        indices,
    })
}

/// Parse binary STL format.
///
/// Binary STL layout:
/// - 80 bytes: header (ignored)
/// - 4 bytes: number of triangles (u32 LE)
/// - Per triangle (50 bytes):
///   - 12 bytes: normal vector (3 × f32 LE)
///   - 36 bytes: vertices (3 × 3 × f32 LE)
///   - 2 bytes: attribute byte count (u16 LE, usually 0)
fn parse_binary_stl(stl_bytes: &[u8]) -> Result<MeshData, String> {
    const HEADER_SIZE: usize = 80;
    const TRIANGLE_SIZE: usize = 50;

    if stl_bytes.len() < HEADER_SIZE + 4 {
        return Err("Failed to parse STL file: file too short for binary STL header".to_string());
    }

    let num_triangles = u32::from_le_bytes(
        stl_bytes[HEADER_SIZE..HEADER_SIZE + 4]
            .try_into()
            .map_err(|_| "Failed to parse STL file: could not read triangle count".to_string())?,
    ) as usize;

    if num_triangles == 0 {
        return Err("CadQuery produced no geometry. Check your script.".to_string());
    }

    let expected_size = HEADER_SIZE + 4 + num_triangles * TRIANGLE_SIZE;
    if stl_bytes.len() < expected_size {
        return Err(format!(
            "Failed to parse STL file: expected {} bytes for {} triangles, got {}",
            expected_size,
            num_triangles,
            stl_bytes.len()
        ));
    }

    let mut vertices = Vec::with_capacity(num_triangles * 9);
    let mut normals = Vec::with_capacity(num_triangles * 9);
    let mut indices = Vec::with_capacity(num_triangles * 3);

    let data_start = HEADER_SIZE + 4;

    for i in 0..num_triangles {
        let offset = data_start + i * TRIANGLE_SIZE;

        // Read normal (3 × f32)
        let nx = f32::from_le_bytes(stl_bytes[offset..offset + 4].try_into().unwrap());
        let ny = f32::from_le_bytes(stl_bytes[offset + 4..offset + 8].try_into().unwrap());
        let nz = f32::from_le_bytes(stl_bytes[offset + 8..offset + 12].try_into().unwrap());

        // Read 3 vertices (each 3 × f32)
        for v in 0..3 {
            let v_offset = offset + 12 + v * 12;
            let x = f32::from_le_bytes(stl_bytes[v_offset..v_offset + 4].try_into().unwrap());
            let y =
                f32::from_le_bytes(stl_bytes[v_offset + 4..v_offset + 8].try_into().unwrap());
            let z =
                f32::from_le_bytes(stl_bytes[v_offset + 8..v_offset + 12].try_into().unwrap());

            vertices.push(x);
            vertices.push(y);
            vertices.push(z);

            normals.push(nx);
            normals.push(ny);
            normals.push(nz);
        }

        // Indices: sequential (each triangle gets 3 consecutive indices)
        let base = (i as u32) * 3;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
    }

    Ok(MeshData {
        vertices,
        normals,
        indices,
    })
}

/// Write MeshData as a binary STL file.
pub fn write_stl(mesh: &MeshData, path: &Path) -> Result<(), String> {
    let num_vertices = mesh.vertices.len() / 3;
    let num_triangles = mesh.indices.len() / 3;

    let mut file = fs::File::create(path).map_err(|e| format!("Failed to write STL file: {}", e))?;

    // 80-byte header
    let header = [0u8; 80];
    file.write_all(&header)
        .map_err(|e| format!("Failed to write STL file: {}", e))?;

    // Number of triangles
    file.write_all(&(num_triangles as u32).to_le_bytes())
        .map_err(|e| format!("Failed to write STL file: {}", e))?;

    // Write each triangle
    for tri in 0..num_triangles {
        let i0 = mesh.indices[tri * 3] as usize;
        let i1 = mesh.indices[tri * 3 + 1] as usize;
        let i2 = mesh.indices[tri * 3 + 2] as usize;

        // Use the normal from the first vertex of the triangle
        let nx = if i0 * 3 + 2 < mesh.normals.len() {
            mesh.normals[i0 * 3]
        } else {
            0.0
        };
        let ny = if i0 * 3 + 2 < mesh.normals.len() {
            mesh.normals[i0 * 3 + 1]
        } else {
            0.0
        };
        let nz = if i0 * 3 + 2 < mesh.normals.len() {
            mesh.normals[i0 * 3 + 2]
        } else {
            0.0
        };

        // Write normal
        file.write_all(&nx.to_le_bytes())
            .map_err(|e| format!("Failed to write STL file: {}", e))?;
        file.write_all(&ny.to_le_bytes())
            .map_err(|e| format!("Failed to write STL file: {}", e))?;
        file.write_all(&nz.to_le_bytes())
            .map_err(|e| format!("Failed to write STL file: {}", e))?;

        // Write 3 vertices
        for &idx in &[i0, i1, i2] {
            if idx * 3 + 2 < mesh.vertices.len() {
                file.write_all(&mesh.vertices[idx * 3].to_le_bytes())
                    .map_err(|e| format!("Failed to write STL file: {}", e))?;
                file.write_all(&mesh.vertices[idx * 3 + 1].to_le_bytes())
                    .map_err(|e| format!("Failed to write STL file: {}", e))?;
                file.write_all(&mesh.vertices[idx * 3 + 2].to_le_bytes())
                    .map_err(|e| format!("Failed to write STL file: {}", e))?;
            } else {
                // Write zeros for out-of-bounds vertices
                file.write_all(&[0u8; 12])
                    .map_err(|e| format!("Failed to write STL file: {}", e))?;
            }
        }

        // Attribute byte count (0)
        file.write_all(&0u16.to_le_bytes())
            .map_err(|e| format!("Failed to write STL file: {}", e))?;
    }

    let _ = num_vertices; // suppress unused warning
    Ok(())
}

/// Concatenate user CadQuery code with the injected STL export block.
///
/// `stl_path` should already have backslashes escaped for embedding in a Python string literal.
pub(crate) fn assemble_script(code: &str, stl_path: &str) -> String {
    format!(
        "{}\nimport cadquery as cq\nfrom cadquery import exporters\nexporters.export(result, '{}')\n",
        code, stl_path
    )
}

/// Classify a Python stderr string into a user-friendly error message.
///
/// Returns a CadQuery installation hint when stderr contains `ModuleNotFoundError`
/// or `ImportError`; otherwise returns the stderr content as a script error.
pub(crate) fn classify_error(stderr: &str) -> String {
    if stderr.contains("ModuleNotFoundError") || stderr.contains("ImportError") {
        "CadQuery is not installed. Install with: pip install cadquery".to_string()
    } else {
        format!("CadQuery script error: {}", stderr.trim())
    }
}

/// Write CadQuery Python code to a temp file, inject STL export, invoke python3,
/// parse the resulting STL.
///
/// This function is synchronous (blocking) — use `tokio::task::spawn_blocking`
/// if calling from an async context.
pub fn execute_cadquery(code: &str) -> Result<MeshData, String> {
    let temp_dir = std::env::temp_dir();
    let id = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let py_path = temp_dir.join(format!("kiro_{}_{}.py", id, timestamp));
    let stl_path = temp_dir.join(format!("kiro_{}_{}.stl", id, timestamp));

    // Scope guard: ensure both temp files are cleaned up on exit
    let _py_guard = TempFileGuard {
        path: py_path.clone(),
    };
    let _stl_guard = TempFileGuard {
        path: stl_path.clone(),
    };

    // Build the script: user code + injected STL export
    let stl_path_str = stl_path.to_string_lossy().replace('\\', "\\\\");
    let full_script = assemble_script(code, &stl_path_str);

    // Write Python script to temp file
    fs::write(&py_path, &full_script)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Invoke python3 (fallback to python on Windows)
    let python_cmd = if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    };

    let output = Command::new(python_cmd)
        .arg(&py_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "Python is not installed or not on PATH. Install from https://www.python.org/"
                    .to_string()
            } else {
                format!("Failed to execute Python: {}", e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_error(&stderr));
    }

    // Read and parse the resulting STL
    if !stl_path.exists() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if !stderr.trim().is_empty() {
            format!(" stderr: {}", stderr.trim())
        } else if !stdout.trim().is_empty() {
            format!(" stdout: {}", stdout.trim())
        } else {
            String::new()
        };
        return Err(format!(
            "CadQuery script ran but produced no geometry. The code may have a runtime error or the `result` variable may not be a valid CadQuery shape.{}",
            detail
        ));
    }
    let stl_bytes =
        fs::read(&stl_path).map_err(|e| format!("Failed to read STL output: {}", e))?;

    parse_stl(&stl_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_parse_binary_stl_single_triangle() {
        // Build a minimal binary STL with 1 triangle
        let mut data = Vec::new();
        // 80-byte header
        data.extend_from_slice(&[0u8; 80]);
        // 1 triangle
        data.extend_from_slice(&1u32.to_le_bytes());
        // Normal: (0, 0, 1)
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        // Vertex 1: (0, 0, 0)
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        // Vertex 2: (1, 0, 0)
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        // Vertex 3: (0, 1, 0)
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&0.0f32.to_le_bytes());
        // Attribute byte count
        data.extend_from_slice(&0u16.to_le_bytes());

        let mesh = parse_stl(&data).unwrap();
        assert_eq!(mesh.vertices.len(), 9); // 3 vertices × 3 coords
        assert_eq!(mesh.normals.len(), 9);  // 3 normals × 3 coords
        assert_eq!(mesh.indices.len(), 3);  // 1 triangle × 3 indices
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(mesh.vertices, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert_eq!(mesh.normals, vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_parse_ascii_stl() {
        let ascii_stl = b"solid test
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 0 1 0
    endloop
  endfacet
endsolid test";

        let mesh = parse_stl(ascii_stl).unwrap();
        assert_eq!(mesh.vertices.len(), 9);
        assert_eq!(mesh.normals.len(), 9);
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(mesh.vertices, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_parse_empty_stl() {
        let result = parse_stl(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_parse_binary_stl_zero_triangles() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 80]);
        data.extend_from_slice(&0u32.to_le_bytes());

        let result = parse_stl(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no geometry"));
    }

    #[test]
    fn test_parse_binary_stl_truncated() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0u8; 80]);
        data.extend_from_slice(&1u32.to_le_bytes());
        // Only 10 bytes of triangle data instead of 50
        data.extend_from_slice(&[0u8; 10]);

        let result = parse_stl(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse STL file"));
    }

    #[test]
    fn test_write_stl_round_trip() {
        let mesh = MeshData {
            vertices: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 2],
        };

        let temp_path = std::env::temp_dir().join("test_write_stl_round_trip_cq.stl");
        write_stl(&mesh, &temp_path).unwrap();

        let stl_bytes = fs::read(&temp_path).unwrap();
        let parsed = parse_stl(&stl_bytes).unwrap();

        assert_eq!(mesh.vertices, parsed.vertices);
        assert_eq!(mesh.normals, parsed.normals);
        assert_eq!(mesh.indices, parsed.indices);

        let _ = fs::remove_file(&temp_path);
    }

    /// Helper: build a valid binary STL byte array with N triangles using random f32 values.
    fn build_binary_stl(num_triangles: usize, floats: &[f32]) -> Vec<u8> {
        let mut data = Vec::new();
        // 80-byte header
        data.extend_from_slice(&[0u8; 80]);
        // Triangle count
        data.extend_from_slice(&(num_triangles as u32).to_le_bytes());
        // Each triangle: 12 floats (3 normal + 9 vertex) + 2 bytes attribute
        for i in 0..num_triangles {
            let base = i * 12;
            for j in 0..12 {
                let val = floats.get(base + j).copied().unwrap_or(0.0);
                data.extend_from_slice(&val.to_le_bytes());
            }
            // Attribute byte count = 0
            data.extend_from_slice(&0u16.to_le_bytes());
        }
        data
    }

    // Feature: cadquery-backend, Property 6: STL parse structural invariant
    // **Validates: Requirements 5.3, 5.6**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_stl_parse_structural_invariant(
            num_triangles in 1usize..=50,
            floats in proptest::collection::vec(any::<f32>().prop_filter("finite floats", |f| f.is_finite()), 1..=600)
        ) {
            let stl_bytes = build_binary_stl(num_triangles, &floats);
            let mesh = parse_stl(&stl_bytes).unwrap();

            // vertices.len() == 9 * N
            prop_assert_eq!(mesh.vertices.len(), 9 * num_triangles);
            // normals.len() == 9 * N
            prop_assert_eq!(mesh.normals.len(), 9 * num_triangles);
            // indices.len() == 3 * N
            prop_assert_eq!(mesh.indices.len(), 3 * num_triangles);
            // all indices < 3 * N
            let max_index = (3 * num_triangles) as u32;
            for &idx in &mesh.indices {
                prop_assert!(idx < max_index, "index {} >= max {}", idx, max_index);
            }
        }
    }

    /// Generate a valid MeshData with N triangles using per-face normals
    /// (same normal for all 3 vertices of each triangle) and sequential indices.
    fn gen_valid_mesh(num_triangles: usize, vert_floats: &[f32], norm_face_floats: &[f32]) -> MeshData {
        let mut vertices = Vec::with_capacity(num_triangles * 9);
        let mut normals = Vec::with_capacity(num_triangles * 9);
        let mut indices = Vec::with_capacity(num_triangles * 3);

        for t in 0..num_triangles {
            // 9 vertex floats per triangle
            for j in 0..9 {
                let idx = t * 9 + j;
                vertices.push(vert_floats.get(idx).copied().unwrap_or(0.0));
            }
            // 3 normal floats per face, replicated to all 3 vertices
            let nx = norm_face_floats.get(t * 3).copied().unwrap_or(0.0);
            let ny = norm_face_floats.get(t * 3 + 1).copied().unwrap_or(0.0);
            let nz = norm_face_floats.get(t * 3 + 2).copied().unwrap_or(0.0);
            for _ in 0..3 {
                normals.push(nx);
                normals.push(ny);
                normals.push(nz);
            }
            // Sequential indices
            let base = (t as u32) * 3;
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
        }

        MeshData { vertices, normals, indices }
    }

    // Feature: cadquery-backend, Property 7: STL binary round-trip
    // **Validates: Requirements 5.7, 5.8**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_stl_binary_round_trip(
            num_triangles in 1usize..=20,
            vert_floats in proptest::collection::vec(
                any::<f32>().prop_filter("finite floats", |f| f.is_finite()),
                1..=180  // up to 20 * 9
            ),
            norm_face_floats in proptest::collection::vec(
                any::<f32>().prop_filter("finite floats", |f| f.is_finite()),
                1..=60   // up to 20 * 3
            ),
        ) {
            let mesh = gen_valid_mesh(num_triangles, &vert_floats, &norm_face_floats);

            let temp_path = std::env::temp_dir().join(format!(
                "prop_stl_round_trip_cq_{:?}.stl",
                std::thread::current().id()
            ));

            write_stl(&mesh, &temp_path).unwrap();
            let stl_bytes = fs::read(&temp_path).unwrap();
            let parsed = parse_stl(&stl_bytes).unwrap();
            let _ = fs::remove_file(&temp_path);

            // Same lengths
            prop_assert_eq!(parsed.vertices.len(), mesh.vertices.len());
            prop_assert_eq!(parsed.normals.len(), mesh.normals.len());
            prop_assert_eq!(parsed.indices.len(), mesh.indices.len());

            // Vertices match exactly (f32 -> le bytes -> f32 is lossless for finite values)
            for (i, (a, b)) in mesh.vertices.iter().zip(parsed.vertices.iter()).enumerate() {
                prop_assert!(
                    (a - b).abs() < f32::EPSILON,
                    "vertex mismatch at index {}: {} vs {}", i, a, b
                );
            }

            // Normals match exactly
            for (i, (a, b)) in mesh.normals.iter().zip(parsed.normals.iter()).enumerate() {
                prop_assert!(
                    (a - b).abs() < f32::EPSILON,
                    "normal mismatch at index {}: {} vs {}", i, a, b
                );
            }

            // Indices match exactly
            prop_assert_eq!(mesh.indices, parsed.indices);
        }
    }

    /// Collect all `kiro_*` .py and .stl file names in the system temp directory.
    fn kiro_temp_files() -> std::collections::HashSet<std::path::PathBuf> {
        let temp_dir = std::env::temp_dir();
        let mut files = std::collections::HashSet::new();
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("kiro_")
                        && (name.ends_with(".py") || name.ends_with(".stl"))
                    {
                        files.insert(path);
                    }
                }
            }
        }
        files
    }

    // Feature: cadquery-backend, Property 1: Script assembly preserves code and appends export
    // **Validates: Requirements 1.3**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_script_assembly_preserves_code_and_appends_export(
            code in "[ -~\\n]{0,300}",
            stl_path in "[ -~]{1,100}",
        ) {
            let assembled = assemble_script(&code, &stl_path);

            // 1. The assembled script starts with the original code
            prop_assert!(
                assembled.starts_with(&code),
                "Assembled script does not start with original code"
            );

            // 2. The assembled script contains the export statement with the stl_path
            let expected_export = format!("exporters.export(result, '{}')", stl_path);
            prop_assert!(
                assembled.contains(&expected_export),
                "Assembled script does not contain expected export statement: {}",
                expected_export
            );

            // 3. The assembled script contains `import cadquery as cq`
            prop_assert!(
                assembled.contains("import cadquery as cq"),
                "Assembled script does not contain 'import cadquery as cq'"
            );

            // 4. The assembled script contains `from cadquery import exporters`
            prop_assert!(
                assembled.contains("from cadquery import exporters"),
                "Assembled script does not contain 'from cadquery import exporters'"
            );
        }
    }

    // Feature: cadquery-backend, Property 2: CadQuery import error detection from stderr
    // **Validates: Requirements 1.7**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_cadquery_import_error_detection(
            prefix in "[a-zA-Z0-9 :._\\-\\n]{0,100}",
            suffix in "[a-zA-Z0-9 :._\\-\\n]{0,100}",
            keyword_index in 0usize..2,
        ) {
            // Sub-property 1: stderr containing "ModuleNotFoundError" or "ImportError"
            // should produce an error message mentioning CadQuery installation.
            let keywords = ["ModuleNotFoundError", "ImportError"];
            let keyword = keywords[keyword_index];
            let stderr_with_keyword = format!("{}{}{}", prefix, keyword, suffix);
            let result = classify_error(&stderr_with_keyword);
            prop_assert!(
                result.contains("CadQuery is not installed"),
                "Expected CadQuery install message for stderr containing '{}', got: {}",
                keyword, result
            );
        }

        #[test]
        fn prop_cadquery_generic_error_passthrough(
            stderr in "[a-zA-Z0-9 :._\\-]{1,200}"
                .prop_filter("must not contain import error keywords", |s| {
                    !s.contains("ModuleNotFoundError") && !s.contains("ImportError")
                }),
        ) {
            // Sub-property 2: stderr containing neither keyword should pass through
            // the stderr content directly, starting with "CadQuery script error:".
            let result = classify_error(&stderr);
            prop_assert!(
                result.starts_with("CadQuery script error:"),
                "Expected result to start with 'CadQuery script error:', got: {}",
                result
            );
            prop_assert!(
                result.contains(stderr.trim()),
                "Expected result to contain stderr content '{}', got: {}",
                stderr.trim(), result
            );
        }
    }

    // Feature: cadquery-backend, Property 3: Temp file cleanup after execution
    // **Validates: Requirements 1.8**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_temp_file_cleanup_after_execution(
            py_code in "[a-zA-Z0-9 _();{}\\n]{0,200}"
        ) {
            let before = kiro_temp_files();

            // execute_cadquery will likely error (python3/cadquery not installed) — that's fine,
            // we only care that temp files are cleaned up regardless of outcome.
            let _ = execute_cadquery(&py_code);

            let after = kiro_temp_files();

            // Any files that appeared after the call but weren't there before
            let new_files: Vec<_> = after.difference(&before).collect();
            prop_assert!(
                new_files.is_empty(),
                "Temp files not cleaned up after execute_cadquery: {:?}",
                new_files
            );
        }
    }
}
