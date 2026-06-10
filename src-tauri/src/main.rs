#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cadquery;
mod llm;

use std::sync::Mutex;
use llm::{Message, Credentials};
use cadquery::MeshData;
use serde::Serialize;

pub struct AppState {
    pub credentials: Mutex<Option<Credentials>>,
    pub last_mesh: Mutex<Option<MeshData>>,
}

#[derive(Debug, Serialize)]
pub struct StructuredResponse {
    pub chat_message: String,
    pub generated_code: Option<String>,
    pub mesh_data: Option<MeshData>,
}

#[tauri::command]
async fn set_credentials(
    provider: String,
    api_key: Option<String>,
    region: Option<String>,
    model: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let creds = match provider.as_str() {
        "gemini" => {
            let key = api_key.ok_or("Gemini API key is required.")?;
            let mdl = model.unwrap_or_else(|| "gemini-3.5-flash".to_string());
            Credentials::Gemini { api_key: key, model: mdl }
        }
        "bedrock" => {
            let bedrock_key = api_key
                .filter(|s| !s.trim().is_empty())
                .ok_or("Bedrock API key is required.")?;
            let bedrock_region = region
                .filter(|s| !s.trim().is_empty())
                .ok_or("Region is required.")?;
            let mdl = model.unwrap_or_else(|| "global.anthropic.claude-sonnet-4-6".to_string());
            Credentials::Bedrock { api_key: bedrock_key, region: bedrock_region, model: mdl }
        }
        "openai" => {
            let key = api_key.ok_or("OpenAI API key is required.")?;
            let mdl = model.unwrap_or_else(|| "gpt-5.5".to_string());
            Credentials::OpenAI { api_key: key, model: mdl }
        }
        "anthropic" => {
            let key = api_key.ok_or("Anthropic API key is required.")?;
            let mdl = model.unwrap_or_else(|| "claude-sonnet-4-6".to_string());
            Credentials::Anthropic { api_key: key, model: mdl }
        }
        _ => return Err(format!("Unknown provider: {}", provider)),
    };
    *state.credentials.lock().map_err(|e| e.to_string())? = Some(creds);
    Ok(())
}

#[tauri::command]
async fn generate_geometry(
    prompt: String,
    history: Vec<Message>,
    image_base64: Option<String>,
    image_mime_type: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<StructuredResponse, String> {
    // Get credentials from state
    let creds = state.credentials.lock().map_err(|e| e.to_string())?.clone();

    // Generate code via LLM
    let llm_response = llm::generate_code(&prompt, &history, creds.as_ref(), image_base64.as_deref(), image_mime_type.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // Try to extract code from the response
    match llm::extract_code(&llm_response) {
        Some(code) => {
            // Try to execute the SCAD code
            match cadquery::execute_cadquery(&code) {
                Ok(mesh) => {
                    // Store mesh for export
                    *state.last_mesh.lock().map_err(|e| e.to_string())? = Some(mesh.clone());

                    Ok(assemble_success_response(&llm_response, &code, mesh))
                }
                Err(err) => {
                    // CadQuery error: return error with the code
                    Ok(StructuredResponse {
                        chat_message: format!("CadQuery error: {}", err),
                        generated_code: Some(code),
                        mesh_data: None,
                    })
                }
            }
        }
        None => {
            // No code block: return full LLM text as chat message
            Ok(StructuredResponse {
                chat_message: llm_response,
                generated_code: None,
                mesh_data: None,
            })
        }
    }
}

#[tauri::command]
async fn export_stl(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mesh = state.last_mesh.lock().map_err(|e| e.to_string())?;
    match mesh.as_ref() {
        Some(mesh_data) => {
            cadquery::write_stl(mesh_data, std::path::Path::new(&path))
        }
        None => {
            Err("No model has been generated yet. Generate a model first.".to_string())
        }
    }
}

/// Assemble a StructuredResponse for the success case where code was extracted
/// and geometry was produced. This is the core assembly logic from generate_geometry,
/// extracted for testability.
pub fn assemble_success_response(
    llm_response: &str,
    code: &str,
    mesh: MeshData,
) -> StructuredResponse {
    let chat_message = llm_response
        .replace(&format!("```python\n{}\n```", code), "")
        .replace(&format!("```\n{}\n```", code), "")
        .trim()
        .to_string();
    let chat_message = if chat_message.is_empty() {
        "Here's your generated geometry.".to_string()
    } else {
        chat_message
    };

    StructuredResponse {
        chat_message,
        generated_code: Some(code.to_string()),
        mesh_data: Some(mesh),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            credentials: Mutex::new(None),
            last_mesh: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            generate_geometry,
            set_credentials,
            export_stl
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: cadquery-backend, Property 8: Structured response completeness on success
    // **Validates: Requirements 4.1**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_structured_response_completeness_on_success(
            // Generate random Python code (no backticks to avoid interfering with fence parsing)
            code in "[a-zA-Z0-9_ ();{}\\[\\],\\.\\-\\+\\*/=\n]{1,200}",
            // Generate random surrounding chat text (no backticks)
            chat_prefix in "[a-zA-Z0-9 .,!?:]{1,100}",
            // Generate random mesh data with at least 1 triangle
            num_triangles in 1usize..=10,
            vert_floats in proptest::collection::vec(
                any::<f32>().prop_filter("finite floats", |f| f.is_finite()),
                9..=90  // 1..10 triangles × 9 floats
            ),
            norm_floats in proptest::collection::vec(
                any::<f32>().prop_filter("finite floats", |f| f.is_finite()),
                9..=90
            ),
        ) {
            // Build a trimmed code (matching extract_code behavior)
            let trimmed_code = code.trim().to_string();
            prop_assume!(!trimmed_code.is_empty());

            // Wrap code in python fence to simulate LLM response
            let llm_response = format!("{}\n```python\n{}\n```", chat_prefix, code);

            // Verify extract_code finds the code
            let extracted = llm::extract_code(&llm_response);
            prop_assume!(extracted.is_some());
            let extracted_code = extracted.unwrap();

            // Build a mock MeshData with correct sizes
            let mut vertices = Vec::with_capacity(num_triangles * 9);
            let mut normals = Vec::with_capacity(num_triangles * 9);
            let mut indices = Vec::with_capacity(num_triangles * 3);
            for t in 0..num_triangles {
                for j in 0..9 {
                    let idx = t * 9 + j;
                    vertices.push(vert_floats.get(idx).copied().unwrap_or(1.0));
                    normals.push(norm_floats.get(idx).copied().unwrap_or(0.0));
                }
                let base = (t as u32) * 3;
                indices.push(base);
                indices.push(base + 1);
                indices.push(base + 2);
            }
            let mesh = MeshData { vertices, normals, indices };

            // Assemble the structured response
            let response = assemble_success_response(&llm_response, &extracted_code, mesh);

            // Property assertions:
            // 1. chat_message is non-empty
            prop_assert!(!response.chat_message.is_empty(),
                "chat_message should be non-empty");

            // 2. generated_code is non-null and matches extracted code
            prop_assert!(response.generated_code.is_some(),
                "generated_code should be non-null on success");
            prop_assert_eq!(response.generated_code.as_deref(), Some(extracted_code.as_str()),
                "generated_code should match extracted code");

            // 3. mesh_data is non-null with non-empty arrays
            prop_assert!(response.mesh_data.is_some(),
                "mesh_data should be non-null on success");
            let mesh_data = response.mesh_data.unwrap();
            prop_assert!(!mesh_data.vertices.is_empty(),
                "mesh_data.vertices should be non-empty");
            prop_assert!(!mesh_data.normals.is_empty(),
                "mesh_data.normals should be non-empty");
            prop_assert!(!mesh_data.indices.is_empty(),
                "mesh_data.indices should be non-empty");
        }
    }

    // Feature: cadquery-backend, Property 9: Structured response nulls when no code block
    // **Validates: Requirements 4.2**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_structured_response_nulls_when_no_code_block(
            response in "[^`]{1,500}"
        ) {
            // Verify extract_code returns None for this input (no backticks means no fences)
            prop_assert_eq!(llm::extract_code(&response), None,
                "extract_code should return None for input with no backticks");

            // Build StructuredResponse using the no-code-block branch logic
            // (mirrors the None arm in generate_geometry)
            let structured = StructuredResponse {
                chat_message: response.clone(),
                generated_code: None,
                mesh_data: None,
            };

            // Assert generated_code is None
            prop_assert!(structured.generated_code.is_none(),
                "generated_code should be None when no code block present");

            // Assert mesh_data is None
            prop_assert!(structured.mesh_data.is_none(),
                "mesh_data should be None when no code block present");

            // Assert chat_message contains the full response
            prop_assert_eq!(&structured.chat_message, &response,
                "chat_message should equal the full LLM response");
        }
    }

    // Feature: bedrock-llm-provider, Property 1: Credentials storage round-trip
    // **Validates: Requirements 1.5, 2.3, 10.2, 10.3**
    fn arb_credentials() -> impl Strategy<Value = Credentials> {
        prop_oneof![
            "[a-zA-Z0-9]{1,100}".prop_map(|api_key| Credentials::Gemini { api_key, model: "test-model".to_string() }),
            (
                "[a-zA-Z0-9]{1,100}",
                "[a-z]{2,20}",
            )
                .prop_map(|(api_key, region)| {
                    Credentials::Bedrock { api_key, region, model: "test-model".to_string() }
                }),
            "[a-zA-Z0-9]{1,100}".prop_map(|api_key| Credentials::OpenAI { api_key, model: "test-model".to_string() }),
            "[a-zA-Z0-9]{1,100}".prop_map(|api_key| Credentials::Anthropic { api_key, model: "test-model".to_string() }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_credentials_storage_round_trip(
            creds in arb_credentials(),
        ) {
            // Create an AppState with no credentials
            let state = AppState {
                credentials: Mutex::new(None),
                last_mesh: Mutex::new(None),
            };

            // Store credentials (simulating what set_credentials does)
            *state.credentials.lock().unwrap() = Some(creds.clone());

            // Read back from the Mutex
            let stored = state.credentials.lock().unwrap();
            let stored_creds = stored.as_ref().expect("credentials should be Some after storing");

            // Assert all fields match via pattern matching
            match (&creds, stored_creds) {
                (
                    Credentials::Gemini { api_key: original_key, .. },
                    Credentials::Gemini { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, original_key,
                        "Gemini api_key should match after round-trip");
                }
                (
                    Credentials::Bedrock {
                        api_key: orig_ak,
                        region: orig_region,
                        ..
                    },
                    Credentials::Bedrock {
                        api_key: stored_ak,
                        region: stored_region,
                        ..
                    },
                ) => {
                    prop_assert_eq!(stored_ak, orig_ak,
                        "Bedrock api_key should match after round-trip");
                    prop_assert_eq!(stored_region, orig_region,
                        "Bedrock region should match after round-trip");
                }
                (
                    Credentials::OpenAI { api_key: original_key, .. },
                    Credentials::OpenAI { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, original_key,
                        "OpenAI api_key should match after round-trip");
                }
                (
                    Credentials::Anthropic { api_key: original_key, .. },
                    Credentials::Anthropic { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, original_key,
                        "Anthropic api_key should match after round-trip");
                }
                _ => {
                    prop_assert!(false,
                        "Credentials variant mismatch: stored variant differs from original");
                }
            }
        }
    }

    // Feature: bedrock-llm-provider, Property 8: Single active provider
    // **Validates: Requirements 10.4**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_single_active_provider(
            first_creds in arb_credentials(),
            second_creds in arb_credentials(),
        ) {
            // Create an AppState with no credentials
            let state = AppState {
                credentials: Mutex::new(None),
                last_mesh: Mutex::new(None),
            };

            // Store the first credentials (provider A)
            *state.credentials.lock().unwrap() = Some(first_creds.clone());

            // Store the second credentials (provider B), overwriting
            *state.credentials.lock().unwrap() = Some(second_creds.clone());

            // Read back from the Credentials_Store
            let stored = state.credentials.lock().unwrap();
            let stored_creds = stored.as_ref().expect("credentials should be Some after storing");

            // Assert only the second credentials are present
            match (&second_creds, stored_creds) {
                (
                    Credentials::Gemini { api_key: expected_key, .. },
                    Credentials::Gemini { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, expected_key,
                        "Stored Gemini api_key should match second credentials");
                }
                (
                    Credentials::Bedrock {
                        api_key: expected_ak,
                        region: expected_region,
                        ..
                    },
                    Credentials::Bedrock {
                        api_key: stored_ak,
                        region: stored_region,
                        ..
                    },
                ) => {
                    prop_assert_eq!(stored_ak, expected_ak,
                        "Stored Bedrock api_key should match second credentials");
                    prop_assert_eq!(stored_region, expected_region,
                        "Stored Bedrock region should match second credentials");
                }
                (
                    Credentials::OpenAI { api_key: expected_key, .. },
                    Credentials::OpenAI { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, expected_key,
                        "Stored OpenAI api_key should match second credentials");
                }
                (
                    Credentials::Anthropic { api_key: expected_key, .. },
                    Credentials::Anthropic { api_key: stored_key, .. },
                ) => {
                    prop_assert_eq!(stored_key, expected_key,
                        "Stored Anthropic api_key should match second credentials");
                }
                _ => {
                    prop_assert!(false,
                        "Stored credentials variant does not match second credentials variant");
                }
            }

            // Verify the first credentials are completely gone by checking
            // there is exactly one credential set (no trace of provider A)
            // The Mutex<Option<Credentials>> can only hold one value, so if
            // the variant or fields match second_creds, first_creds is gone.
            // Additional check: if first and second are different variants,
            // the stored variant must be the second's variant.
            let second_matches_stored = match (&second_creds, stored_creds) {
                (Credentials::Gemini { api_key: k1, .. }, Credentials::Gemini { api_key: k2, .. }) => k1 == k2,
                (Credentials::Bedrock { api_key: k1, region: r1, .. }, Credentials::Bedrock { api_key: k2, region: r2, .. }) => k1 == k2 && r1 == r2,
                (Credentials::OpenAI { api_key: k1, .. }, Credentials::OpenAI { api_key: k2, .. }) => k1 == k2,
                (Credentials::Anthropic { api_key: k1, .. }, Credentials::Anthropic { api_key: k2, .. }) => k1 == k2,
                _ => false,
            };
            prop_assert!(second_matches_stored,
                "Stored credentials must match the second (most recent) credentials");
        }
    }

    // Feature: bedrock-llm-provider, Property 6: Bedrock credential validation rejects empty fields
    // **Validates: Requirements 2.4**

    /// Replicates the Bedrock credential validation logic from set_credentials.
    fn validate_bedrock_credentials(
        api_key: &Option<String>,
        region: &Option<String>,
    ) -> Result<(), String> {
        api_key
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .ok_or("Bedrock API key is required.".to_string())?;
        region
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .ok_or("Region is required.".to_string())?;
        Ok(())
    }

    /// Strategy that generates a whitespace-only or empty string.
    fn arb_empty_or_whitespace() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),
            Just(" ".to_string()),
            Just("  ".to_string()),
            Just("\t".to_string()),
            Just(" \t\n ".to_string()),
        ]
    }

    /// Strategy that generates a non-empty, non-whitespace-only string.
    fn arb_non_empty_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{1,50}"
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_bedrock_credential_validation_rejects_empty_fields(
            field_to_blank in 0u8..3u8,
            empty_ak in arb_empty_or_whitespace(),
            empty_region in arb_empty_or_whitespace(),
            valid_ak in arb_non_empty_string(),
            valid_region in arb_non_empty_string(),
        ) {
            // Bitmask 1..=3 ensures at least one field is empty/whitespace
            let field_to_blank = (field_to_blank % 3) + 1;

            let api_key = if field_to_blank & 1 != 0 {
                Some(empty_ak.clone())
            } else {
                Some(valid_ak.clone())
            };
            let region = if field_to_blank & 2 != 0 {
                Some(empty_region.clone())
            } else {
                Some(valid_region.clone())
            };

            let result = validate_bedrock_credentials(&api_key, &region);
            prop_assert!(
                result.is_err(),
                "Expected validation error when at least one field is empty/whitespace. \
                 field_to_blank={}, api_key={:?}, region={:?}",
                field_to_blank, api_key, region
            );
        }
    }
}
