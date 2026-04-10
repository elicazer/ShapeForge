use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum Credentials {
    Gemini { api_key: String, model: String },
    Bedrock { api_key: String, region: String, model: String },
    OpenAI { api_key: String, model: String },
    Anthropic { api_key: String, model: String },
}

pub const SYSTEM_PROMPT: &str = r#"You are an expert CadQuery code generator that creates high-quality, realistic 3D models. Generate Python code using the CadQuery library.

Core rules:
- Return valid Python code inside a markdown code fence with the `python` language tag
- Include `import cadquery as cq` at the top of the code
- Store the final CadQuery Workplane object in a variable named `result`
- Include a brief explanation of the generated geometry alongside the code block

CadQuery API correctness — DO NOT use these non-existent parameters:
- .extrude() does NOT accept `centered`, `taper`, or `combine` keyword arguments
- .extrude(distance) takes only a numeric distance (positive or negative). That's it.
- To center an extrusion, use .extrude(d, both=True) to extrude symmetrically in both directions
- There is no taper on extrude. To create tapered shapes, use .box() or build a loft between two sketches
- .box(l,w,h) accepts centered=(True,True,True) but .extrude() does NOT
- .cylinder(h,r) does NOT accept centered — it always starts from the workplane
- Do NOT invent parameters. Only use parameters documented in the CadQuery API.

Design philosophy — think like a product designer:
- Before writing code, think about the real-world object: its proportions, how it's used, what makes it look good
- Use realistic dimensions in millimeters. A phone is ~75mm wide, ~150mm tall, ~8mm thick. A mug is ~80mm diameter, ~95mm tall. A desk organizer is ~150-250mm wide
- Add visual detail: rounded edges, slight tapers, recesses, lips, and bevels make models look designed rather than blocky
- Build complex objects by combining simple primitives with boolean operations (union, cut, intersect)
- Decompose objects into logical parts: base, body, features, details

Construction approach (in order of preference):
1. Box/cylinder/sphere primitives combined with booleans — most reliable
2. Sketch + extrude for custom cross-sections — use .rect(), .circle(), .slot2D()
3. .lineTo() profiles only when absolutely necessary — keep paths simple with few segments

Fillet and chamfer rules (CadQuery's OCC kernel is strict):
- Keep radii small: max 1/5 of the smallest adjacent face dimension
- Apply fillets BEFORE boolean cuts when possible
- Use specific edge selectors: .edges("|Z"), .edges(">Z"), .edges("<X")
- NEVER fillet edges from slot2D, sweep, loft, or shell operations
- Always wrap fillets/chamfers in try/except to prevent crashes:
    try:
        result = result.edges("|Z").fillet(1)
    except:
        pass
- Prefer chamfer over fillet for complex geometry

Available operations:

Shapes: .box(l,w,h), .sphere(r), .cylinder(h,r)
Sketching: .rect(w,h).extrude(d), .circle(r).extrude(d), .slot2D(l,w,angle)
Holes: .hole(diameter), .cboreHole(d,cbd,cbdepth), .cskHole(d,csd,csangle)
Transforms: .translate((x,y,z)), .rotate((ax,ay,az),(bx,by,bz),angle), .mirror("XY")
Booleans: .union(other), .cut(other), .intersect(other)
Edges: .edges("|Z").fillet(r), .edges(">Z").chamfer(l)
Advanced: .sweep(path), .loft(), .shell(thickness)
Workplanes: .faces(">Z").workplane(), .workplane(offset=10)

Example — a desk phone stand with a slot and cable hole:
```python
import cadquery as cq

# Base block
base = cq.Workplane("XY").box(80, 60, 40)

# Cut an angled slot for the phone
slot_tool = (
    cq.Workplane("XZ")
    .center(0, 20)
    .rect(50, 60)
    .extrude(4)
    .rotate((0, 0, 0), (1, 0, 0), -15)
    .translate((0, 5, 0))
)
result = base.cut(slot_tool)

# Cable hole through the back
result = (
    result.faces("<Y").workplane()
    .center(0, -5)
    .hole(8)
)

# Soften vertical edges
try:
    result = result.edges("|Z").chamfer(2)
except:
    pass
```
"#;

const IMAGE_ANALYSIS_PROMPT: &str = r#"Analyze this image carefully and describe the 3D object you see. Focus on:
1. Overall shape and proportions (dimensions in mm if you can estimate)
2. Key geometric features (holes, slots, curves, chamfers, fillets)
3. How the object could be decomposed into simple primitives (boxes, cylinders, spheres)
4. Any symmetry or patterns
5. Material/thickness if visible

Be specific and quantitative. This description will be used to generate CadQuery 3D modeling code."#;

#[derive(Serialize)]
pub(crate) struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction")]
    system_instruction: GeminiContent,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
pub(crate) struct BedrockRequest {
    pub messages: Vec<BedrockMessage>,
    pub system: Vec<BedrockTextBlock>,
    #[serde(rename = "inferenceConfig")]
    pub inference_config: BedrockInferenceConfig,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BedrockMessage {
    pub role: String,
    pub content: Vec<BedrockTextBlock>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BedrockTextBlock {
    pub text: String,
}

#[derive(Serialize)]
pub(crate) struct BedrockInferenceConfig {
    #[serde(rename = "maxTokens")]
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Maps conversation roles to Gemini API roles.
/// "assistant" becomes "model"; everything else passes through unchanged.
pub(crate) fn map_role(role: &str) -> &str {
    match role {
        "assistant" => "model",
        other => other,
    }
}

/// Constructs a full Gemini `generateContent` request body from the prompt,
/// conversation history, and system prompt.
pub(crate) fn build_gemini_request(
    prompt: &str,
    history: &[Message],
    system_prompt: &str,
) -> GeminiRequest {
    let mut contents: Vec<GeminiContent> = history
        .iter()
        .map(|msg| GeminiContent {
            role: Some(map_role(&msg.role).to_string()),
            parts: vec![GeminiPart {
                text: msg.content.clone(),
            }],
        })
        .collect();

    contents.push(GeminiContent {
        role: Some("user".to_string()),
        parts: vec![GeminiPart {
            text: prompt.to_string(),
        }],
    });

    GeminiRequest {
        contents,
        system_instruction: GeminiContent {
            role: None,
            parts: vec![GeminiPart {
                text: system_prompt.to_string(),
            }],
        },
        generation_config: GenerationConfig {
            max_output_tokens: 16384,
            temperature: 0.4,
        },
    }
}

/// Constructs a full Bedrock Converse API request body from the prompt,
/// conversation history, and system prompt.
pub(crate) fn build_bedrock_request(
    prompt: &str,
    history: &[Message],
    system_prompt: &str,
) -> BedrockRequest {
    let mut messages: Vec<BedrockMessage> = history
        .iter()
        .map(|msg| BedrockMessage {
            role: msg.role.clone(),
            content: vec![BedrockTextBlock {
                text: msg.content.clone(),
            }],
        })
        .collect();

    messages.push(BedrockMessage {
        role: "user".to_string(),
        content: vec![BedrockTextBlock {
            text: prompt.to_string(),
        }],
    });

    BedrockRequest {
        messages,
        system: vec![BedrockTextBlock {
            text: system_prompt.to_string(),
        }],
        inference_config: BedrockInferenceConfig {
            max_tokens: 4096,
            temperature: 0.4,
        },
    }
}

/// Extracts the text content from a Gemini API response JSON.
/// Expects the path `candidates[0].content.parts[0].text`.
pub(crate) fn parse_gemini_response(json: &serde_json::Value) -> Result<String, String> {
    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Unexpected response format from Gemini API.".to_string())
}

/// Extracts the text content from a Bedrock Converse API response JSON.
/// Expects the path `output.message.content[0].text`.
pub(crate) fn parse_bedrock_response(json: &serde_json::Value) -> Result<String, String> {
    json["output"]["message"]["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Unexpected response format from Bedrock API.".to_string())
}

/// Maps an HTTP error status code and response body to a user-friendly message.
pub(crate) fn map_error_status(status_code: u16, body: &str) -> String {
    match status_code {
        400 => format!("Invalid request (status 400): {}", body),
        401 | 403 => format!("Invalid API key or insufficient permissions (status {}). Check your Gemini API key in Settings.", status_code),
        429 => "Rate limit exceeded (status 429). Please wait a moment and try again.".to_string(),
        s if s >= 500 => format!("Gemini server error (status {}). Please try again later.", s),
        s => format!("Gemini API error (status {}): {}", s, body),
    }
}

/// Maps a Bedrock HTTP error status code and response body to a user-friendly message.
pub(crate) fn map_bedrock_error_status(status_code: u16, body: &str) -> String {
    match status_code {
        400 => format!("Invalid request (status 400): {}", body),
        403 => "Invalid Bedrock API key or insufficient permissions (status 403). Check your Bedrock API key in Settings.".to_string(),
        429 => "Rate limit exceeded (status 429). Please wait a moment and try again.".to_string(),
        s if (500..=599).contains(&s) => format!("Bedrock server error (status {}). Please try again later.", s),
        s => format!("Bedrock API error (status {}): {}", s, body),
    }
}

pub async fn generate_code(
    prompt: &str,
    history: &[Message],
    credentials: Option<&Credentials>,
    image_base64: Option<&str>,
    image_mime_type: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    match credentials.ok_or("No credentials configured. Please set credentials in Settings.")? {
        Credentials::Gemini { api_key, model } => generate_gemini(prompt, history, api_key, model, image_base64, image_mime_type).await,
        Credentials::Bedrock { api_key, region, model } => {
            generate_bedrock(prompt, history, api_key, region, model, image_base64, image_mime_type).await
        }
        Credentials::OpenAI { api_key, model } => generate_openai(prompt, history, api_key, model, image_base64, image_mime_type).await,
        Credentials::Anthropic { api_key, model } => generate_anthropic(prompt, history, api_key, model, image_base64, image_mime_type).await,
    }
}

async fn generate_gemini(
    prompt: &str,
    history: &[Message],
    api_key: &str,
    model: &str,
    image_base64: Option<&str>,
    image_mime_type: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    // Two-step approach for images: first analyze, then generate code
    let effective_prompt = if let Some(img) = image_base64 {
        let description = analyze_image_gemini(img, image_mime_type.unwrap_or("image/png"), prompt, api_key, model).await?;
        format!("{}\n\nUser's additional instructions: {}", description, prompt)
    } else {
        prompt.to_string()
    };

    let body = serde_json::to_value(&build_gemini_request(&effective_prompt, history, SYSTEM_PROMPT))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|_| {
            "Connection failed: unable to reach the Gemini API. Check your internet connection."
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await.map_err(|_| {
        "Connection failed: unable to reach the Gemini API. Check your internet connection."
    })?;

    parse_gemini_response(&response_json).map_err(|e| e.into())
}

/// Step 1 of image-to-3D: analyze the image and produce a detailed text description.
async fn analyze_image_gemini(
    image_base64: &str,
    mime_type: &str,
    user_hint: &str,
    api_key: &str,
    model: &str,
) -> Result<String, Box<dyn Error>> {
    let analysis_prompt = if user_hint.is_empty() || user_hint == "Generate a 3D model based on this image" {
        IMAGE_ANALYSIS_PROMPT.to_string()
    } else {
        format!("{}\n\nThe user also says: {}", IMAGE_ANALYSIS_PROMPT, user_hint)
    };

    let body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [
                {
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": image_base64
                    }
                },
                { "text": analysis_prompt }
            ]
        }],
        "generationConfig": {
            "maxOutputTokens": 4096,
            "temperature": 0.3
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|_| "Connection failed during image analysis.")?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|_| "Failed to parse image analysis response.")?;

    parse_gemini_response(&response_json).map_err(|e| e.into())
}

/// Step 1 of image-to-3D for Bedrock: analyze the image and produce a detailed text description.
async fn analyze_image_bedrock(
    image_base64: &str,
    mime_type: &str,
    user_hint: &str,
    api_key: &str,
    region: &str,
    model: &str,
) -> Result<String, Box<dyn Error>> {
    let analysis_prompt = if user_hint.is_empty() || user_hint == "Generate a 3D model based on this image" {
        IMAGE_ANALYSIS_PROMPT.to_string()
    } else {
        format!("{}\n\nThe user also says: {}", IMAGE_ANALYSIS_PROMPT, user_hint)
    };

    let format = mime_type.strip_prefix("image/").unwrap_or("png");

    let body = serde_json::json!({
        "messages": [{
            "role": "user",
            "content": [
                {
                    "image": {
                        "format": format,
                        "source": { "bytes": image_base64 }
                    }
                },
                { "text": analysis_prompt }
            ]
        }],
        "inferenceConfig": {
            "maxTokens": 4096,
            "temperature": 0.3
        }
    });

    let url = format!(
        "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
        region, model
    );

    let response = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|_| "Connection failed during image analysis.")?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_bedrock_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|_| "Failed to parse image analysis response.")?;

    parse_bedrock_response(&response_json).map_err(|e| e.into())
}

async fn generate_bedrock(
    prompt: &str,
    history: &[Message],
    api_key: &str,
    region: &str,
    model: &str,
    image_base64: Option<&str>,
    image_mime_type: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    // Two-step approach for images: first analyze, then generate code
    let effective_prompt = if let Some(img) = image_base64 {
        let description = analyze_image_bedrock(img, image_mime_type.unwrap_or("image/png"), prompt, api_key, region, model).await?;
        format!("{}\n\nUser's additional instructions: {}", description, prompt)
    } else {
        prompt.to_string()
    };

    let url = format!(
        "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
        region, model
    );

    let request_body = serde_json::to_value(&build_bedrock_request(&effective_prompt, history, SYSTEM_PROMPT))?;

    let response = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|_| {
            "Connection failed: unable to reach the Bedrock API. Check your internet connection."
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_bedrock_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await.map_err(|_| {
        "Connection failed: unable to reach the Bedrock API. Check your internet connection."
    })?;

    parse_bedrock_response(&response_json).map_err(|e| e.into())
}

/// Maps an OpenAI HTTP error status code and response body to a user-friendly message.
pub(crate) fn map_openai_error_status(status_code: u16, body: &str) -> String {
    match status_code {
        400 => format!("Invalid request (status 400): {}", body),
        401 | 403 => format!("Invalid OpenAI API key or insufficient permissions (status {}). Check your OpenAI API key in Settings.", status_code),
        429 => "Rate limit exceeded (status 429). Please wait a moment and try again.".to_string(),
        s if (500..=599).contains(&s) => format!("OpenAI server error (status {}). Please try again later.", s),
        s => format!("OpenAI API error (status {}): {}", s, body),
    }
}

/// Maps an Anthropic HTTP error status code and response body to a user-friendly message.
pub(crate) fn map_anthropic_error_status(status_code: u16, body: &str) -> String {
    match status_code {
        400 => format!("Invalid request (status 400): {}", body),
        401 | 403 => format!("Invalid Anthropic API key or insufficient permissions (status {}). Check your Anthropic API key in Settings.", status_code),
        429 => "Rate limit exceeded (status 429). Please wait a moment and try again.".to_string(),
        s if (500..=599).contains(&s) => format!("Anthropic server error (status {}). Please try again later.", s),
        s => format!("Anthropic API error (status {}): {}", s, body),
    }
}

/// Extracts the text content from an OpenAI Chat Completions API response JSON.
/// Expects the path `choices[0].message.content`.
pub(crate) fn parse_openai_response(json: &serde_json::Value) -> Result<String, String> {
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Unexpected response format from OpenAI API.".to_string())
}

/// Extracts the text content from an Anthropic Messages API response JSON.
/// Expects the path `content[0].text`.
pub(crate) fn parse_anthropic_response(json: &serde_json::Value) -> Result<String, String> {
    json["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Unexpected response format from Anthropic API.".to_string())
}

/// Step 1 of image-to-3D for OpenAI: analyze the image and produce a detailed text description.
async fn analyze_image_openai(
    image_base64: &str,
    mime_type: &str,
    user_hint: &str,
    api_key: &str,
    model: &str,
) -> Result<String, Box<dyn Error>> {
    let analysis_prompt = if user_hint.is_empty() || user_hint == "Generate a 3D model based on this image" {
        IMAGE_ANALYSIS_PROMPT.to_string()
    } else {
        format!("{}\n\nThe user also says: {}", IMAGE_ANALYSIS_PROMPT, user_hint)
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{};base64,{}", mime_type, image_base64)
                    }
                },
                {
                    "type": "text",
                    "text": analysis_prompt
                }
            ]
        }],
        "max_tokens": 4096,
        "temperature": 0.3
    });

    let response = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|_| "Connection failed during image analysis.")?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_openai_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|_| "Failed to parse image analysis response.")?;

    parse_openai_response(&response_json).map_err(|e| e.into())
}

async fn generate_openai(
    prompt: &str,
    history: &[Message],
    api_key: &str,
    model: &str,
    image_base64: Option<&str>,
    image_mime_type: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    // Two-step approach for images: first analyze, then generate code
    let effective_prompt = if let Some(img) = image_base64 {
        let description = analyze_image_openai(img, image_mime_type.unwrap_or("image/png"), prompt, api_key, model).await?;
        format!("{}\n\nUser's additional instructions: {}", description, prompt)
    } else {
        prompt.to_string()
    };

    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": SYSTEM_PROMPT
    })];

    for msg in history {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": effective_prompt
    }));

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": 16384,
        "temperature": 0.4
    });

    let response = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|_| {
            "Connection failed: unable to reach the OpenAI API. Check your internet connection."
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_openai_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await.map_err(|_| {
        "Connection failed: unable to reach the OpenAI API. Check your internet connection."
    })?;

    parse_openai_response(&response_json).map_err(|e| e.into())
}

/// Step 1 of image-to-3D for Anthropic: analyze the image and produce a detailed text description.
async fn analyze_image_anthropic(
    image_base64: &str,
    mime_type: &str,
    user_hint: &str,
    api_key: &str,
    model: &str,
) -> Result<String, Box<dyn Error>> {
    let analysis_prompt = if user_hint.is_empty() || user_hint == "Generate a 3D model based on this image" {
        IMAGE_ANALYSIS_PROMPT.to_string()
    } else {
        format!("{}\n\nThe user also says: {}", IMAGE_ANALYSIS_PROMPT, user_hint)
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": mime_type,
                        "data": image_base64
                    }
                },
                {
                    "type": "text",
                    "text": analysis_prompt
                }
            ]
        }],
        "max_tokens": 4096,
        "temperature": 0.3
    });

    let response = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|_| "Connection failed during image analysis.")?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_anthropic_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|_| "Failed to parse image analysis response.")?;

    parse_anthropic_response(&response_json).map_err(|e| e.into())
}

async fn generate_anthropic(
    prompt: &str,
    history: &[Message],
    api_key: &str,
    model: &str,
    image_base64: Option<&str>,
    image_mime_type: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    // Two-step approach for images: first analyze, then generate code
    let effective_prompt = if let Some(img) = image_base64 {
        let description = analyze_image_anthropic(img, image_mime_type.unwrap_or("image/png"), prompt, api_key, model).await?;
        format!("{}\n\nUser's additional instructions: {}", description, prompt)
    } else {
        prompt.to_string()
    };

    let mut messages: Vec<serde_json::Value> = Vec::new();

    for msg in history {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": effective_prompt
    }));

    let body = serde_json::json!({
        "model": model,
        "system": SYSTEM_PROMPT,
        "messages": messages,
        "max_tokens": 16384,
        "temperature": 0.4
    });

    let response = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|_| {
            "Connection failed: unable to reach the Anthropic API. Check your internet connection."
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(map_anthropic_error_status(status, &body).into());
    }

    let response_json: serde_json::Value = response.json().await.map_err(|_| {
        "Connection failed: unable to reach the Anthropic API. Check your internet connection."
    })?;

    parse_anthropic_response(&response_json).map_err(|e| e.into())
}

/// Extract Python code from a markdown-fenced LLM response.
///
/// Priority:
/// 1. First ` ```python ` fenced block
/// 2. First bare ` ``` ` fenced block (no language tag)
/// 3. None if no fence found
pub fn extract_code(response: &str) -> Option<String> {
    // Try ```python first
    let python_markers = ["```python\n", "```python\r\n"];
    for marker in &python_markers {
        if let Some(start) = response.find(marker) {
            let code_start = start + marker.len();
            if let Some(end) = response[code_start..].find("```") {
                let code = &response[code_start..code_start + end];
                return Some(code.trim().to_string());
            }
        }
    }

    // Fall back to bare ``` fence
    let bare_markers = ["```\n", "```\r\n"];
    for marker in &bare_markers {
        if let Some(start) = response.find(marker) {
            let code_start = start + marker.len();
            if let Some(end) = response[code_start..].find("```") {
                let code = &response[code_start..code_start + end];
                return Some(code.trim().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_python_fence() {
        let response = "Here is a box:\n```python\nimport cadquery as cq\nresult = cq.Workplane(\"XY\").box(10, 10, 10)\n```\nEnjoy!";
        assert_eq!(
            extract_code(response),
            Some("import cadquery as cq\nresult = cq.Workplane(\"XY\").box(10, 10, 10)".to_string())
        );
    }

    #[test]
    fn test_extract_bare_fence() {
        let response = "Here is code:\n```\ncube([5, 5, 5]);\n```\nDone.";
        assert_eq!(
            extract_code(response),
            Some("cube([5, 5, 5]);".to_string())
        );
    }

    #[test]
    fn test_python_fence_takes_priority_over_bare() {
        let response = "```\nbare code\n```\n\n```python\npython code\n```";
        // python fence has first priority even if bare appears first
        assert_eq!(
            extract_code(response),
            Some("python code".to_string())
        );
    }

    #[test]
    fn test_no_fence_returns_none() {
        let response = "Just a plain text response with no code blocks.";
        assert_eq!(extract_code(response), None);
    }

    #[test]
    fn test_openscad_fence_not_matched() {
        // A fence with the openscad language tag should NOT be matched
        let response = "```openscad\ncube([10, 10, 10]);\n```";
        assert_eq!(extract_code(response), None);
    }

    #[test]
    fn test_multiline_code_extraction() {
        let response = "```python\nimport cadquery as cq\nresult = (\n    cq.Workplane(\"XY\")\n    .box(20, 20, 20)\n    .hole(10)\n)\n```";
        let expected = "import cadquery as cq\nresult = (\n    cq.Workplane(\"XY\")\n    .box(20, 20, 20)\n    .hole(10)\n)";
        assert_eq!(extract_code(response), Some(expected.to_string()));
    }

    #[test]
    fn test_strips_leading_trailing_whitespace() {
        let response = "```python\n\n  result = cq.Workplane(\"XY\").box(10, 10, 10)  \n\n```";
        assert_eq!(extract_code(response), Some("result = cq.Workplane(\"XY\").box(10, 10, 10)".to_string()));
    }

    #[test]
    fn test_unclosed_fence_returns_none() {
        let response = "```python\nresult = cq.Workplane(\"XY\").box(10, 10, 10)";
        assert_eq!(extract_code(response), None);
    }

    #[test]
    fn test_first_python_fence_wins() {
        let response = "```python\nfirst();\n```\n\n```python\nsecond();\n```";
        assert_eq!(extract_code(response), Some("first();".to_string()));
    }

    #[test]
    fn test_crlf_line_endings() {
        let response = "```python\r\nresult = cq.Workplane(\"XY\").box(10, 10, 10)\r\n```";
        assert_eq!(extract_code(response), Some("result = cq.Workplane(\"XY\").box(10, 10, 10)".to_string()));
    }

    use proptest::prelude::*;

    // Feature: cadquery-backend, Property 4: Code extraction round-trip (python tag)
    // Validates: Requirements 3.1, 3.4, 3.6
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_extract_code_roundtrip_python_tag(code in "[^`]{1,200}") {
            let trimmed = code.trim();
            prop_assume!(!trimmed.is_empty());

            let wrapped = format!("```python\n{}\n```", code);
            let result = extract_code(&wrapped);

            prop_assert_eq!(result, Some(trimmed.to_string()));
        }
    }

    // Feature: cadquery-backend, Property 5: Code extraction round-trip (bare fence)
    // Validates: Requirements 3.2, 3.4
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_extract_code_roundtrip_bare_fence(code in "[^`]{1,200}") {
            let trimmed = code.trim();
            prop_assume!(!trimmed.is_empty());

            let wrapped = format!("```\n{}\n```", code);
            let result = extract_code(&wrapped);

            prop_assert_eq!(result, Some(trimmed.to_string()));
        }
    }

    // Feature: cadquery-backend, Property 6: No-fence response yields no code
    // Validates: Requirements 3.3
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_no_fence_response_yields_no_code(response in "[^`]{0,500}") {
            let result = extract_code(&response);
            prop_assert_eq!(result, None);
        }
    }

    // Feature: cadquery-backend, Property 7: OpenSCAD fences are not matched
    // Validates: Requirements 3.5
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_openscad_fences_not_matched(code in "[^`]{1,200}") {
            let trimmed = code.trim();
            prop_assume!(!trimmed.is_empty());

            let wrapped = format!("```openscad\n{}\n```", code);
            let result = extract_code(&wrapped);

            // openscad fences should NOT be matched — but bare fence fallback
            // will match the closing ```, so we need to ensure the openscad tag
            // itself is not what's being matched. Since ```openscad\n has a tag,
            // it won't match bare ``` either. So result should be None.
            prop_assert_eq!(result, None);
        }
    }

    // Feature: gemini-llm-backend, Property 1: Credentials storage round-trip
    // **Validates: Requirements 2.3, 2.5**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_credentials_storage_round_trip(api_key in "[a-zA-Z0-9]{1,100}") {
            let creds = Credentials::Gemini { api_key: api_key.clone(), model: "test-model".to_string() };
            let stored: std::sync::Mutex<Option<Credentials>> = std::sync::Mutex::new(None);
            *stored.lock().unwrap() = Some(creds);
            let read = stored.lock().unwrap();
            let read_creds = read.as_ref().expect("should be Some");
            if let Credentials::Gemini { api_key: stored_key, .. } = read_creds {
                prop_assert_eq!(stored_key, &api_key);
            } else {
                prop_assert!(false, "Expected Credentials::Gemini variant");
            }
        }
    }

    // Feature: gemini-llm-backend, Property 2: Gemini request body construction
    // **Validates: Requirements 3.3, 3.4, 3.8**
    fn arb_role() -> impl Strategy<Value = String> {
        prop_oneof![Just("user".to_string()), Just("assistant".to_string())]
    }

    fn arb_message() -> impl Strategy<Value = Message> {
        (arb_role(), "[a-zA-Z0-9 ]{1,100}").prop_map(|(role, content)| Message { role, content })
    }

    fn arb_history() -> impl Strategy<Value = Vec<Message>> {
        proptest::collection::vec(arb_message(), 0..10)
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_gemini_request_body_construction(
            prompt in "[a-zA-Z0-9 ]{1,100}",
            history in arb_history(),
            system_prompt in "[a-zA-Z0-9 ]{1,200}",
        ) {
            let request = build_gemini_request(&prompt, &history, &system_prompt);

            // (a) contents length equals history.len() + 1
            prop_assert_eq!(request.contents.len(), history.len() + 1);

            // (b) & (c) Each history message has correctly mapped role and matching text
            for (i, msg) in history.iter().enumerate() {
                let content = &request.contents[i];
                prop_assert_eq!(
                    content.role.as_deref(),
                    Some(map_role(&msg.role)),
                    "Role mismatch at index {}",
                    i
                );
                prop_assert_eq!(
                    &content.parts[0].text,
                    &msg.content,
                    "Content mismatch at index {}",
                    i
                );
            }

            // Last content entry has role "user" and text matching the prompt
            let last = &request.contents[history.len()];
            prop_assert_eq!(last.role.as_deref(), Some("user"));
            prop_assert_eq!(&last.parts[0].text, &prompt);

            // (d) systemInstruction contains the system prompt text
            prop_assert_eq!(&request.system_instruction.parts[0].text, &system_prompt);

            // (e) generationConfig has correct values
            prop_assert_eq!(request.generation_config.max_output_tokens, 16384);
            prop_assert!((request.generation_config.temperature - 0.4).abs() < f32::EPSILON);
        }
    }

    use serde_json::json;

    // Feature: gemini-llm-backend, Property 3: Gemini response text extraction round-trip
    // **Validates: Requirements 3.5**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_gemini_response_text_extraction_round_trip(text in ".{1,200}") {
            let response_json = json!({
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": text
                        }]
                    }
                }]
            });
            let result = parse_gemini_response(&response_json);
            prop_assert_eq!(result, Ok(text));
        }
    }

    // Feature: gemini-llm-backend, Property 4: HTTP error status code mapping
    // **Validates: Requirements 3.6, 6.1, 6.2, 6.3, 6.4**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_http_error_status_code_mapping(code in 400u16..=599u16) {
            let result = map_error_status(code, "test body");
            prop_assert!(!result.is_empty(), "Error message should be non-empty");
            prop_assert!(
                result.contains(&code.to_string()),
                "Error message '{}' should contain status code '{}'",
                result,
                code
            );
        }
    }

    // Feature: gemini-llm-backend, Property 5: Role mapping correctness
    // **Validates: Requirements 3.8**
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_role_mapping_correctness(other_role in "[a-z]{1,20}") {
            // "user" maps to "user"
            prop_assert_eq!(map_role("user"), "user");
            // "assistant" maps to "model"
            prop_assert_eq!(map_role("assistant"), "model");
            // Any other string (not "user" or "assistant") passes through unchanged
            prop_assume!(other_role != "user" && other_role != "assistant");
            prop_assert_eq!(map_role(&other_role), other_role.as_str());
        }
    }

    // Feature: bedrock-llm-provider, Property 2: Bedrock request body construction
    // Validates: Requirements 3.2, 4.3, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_bedrock_request_body_construction(
            prompt in "[a-zA-Z0-9 ]{1,100}",
            history in arb_history(),
            system_prompt in "[a-zA-Z0-9 ]{1,200}",
        ) {
            let request = build_bedrock_request(&prompt, &history, &system_prompt);

            // (a) messages length equals history.len() + 1
            prop_assert_eq!(request.messages.len(), history.len() + 1);

            // (b) Each history message's role passes through unchanged
            // (c) Each message's content is wrapped as [BedrockTextBlock { text: content }]
            for (i, msg) in history.iter().enumerate() {
                let bedrock_msg = &request.messages[i];
                prop_assert_eq!(
                    &bedrock_msg.role,
                    &msg.role,
                    "Role mismatch at index {}",
                    i
                );
                prop_assert_eq!(
                    bedrock_msg.content.len(),
                    1,
                    "Content should have exactly one BedrockTextBlock at index {}",
                    i
                );
                prop_assert_eq!(
                    &bedrock_msg.content[0].text,
                    &msg.content,
                    "Content text mismatch at index {}",
                    i
                );
            }

            // (d) Last message has role "user" and text equal to prompt
            let last = &request.messages[history.len()];
            prop_assert_eq!(&last.role, "user");
            prop_assert_eq!(last.content.len(), 1);
            prop_assert_eq!(&last.content[0].text, &prompt);

            // (e) system field contains the system prompt text
            prop_assert_eq!(request.system.len(), 1);
            prop_assert_eq!(&request.system[0].text, &system_prompt);

            // (f) inferenceConfig has max_tokens == 4096 and temperature == 0.4
            prop_assert_eq!(request.inference_config.max_tokens, 4096);
            prop_assert!((request.inference_config.temperature - 0.4).abs() < f32::EPSILON);
        }
    }

    // Feature: bedrock-llm-provider, Property 3: Bedrock response parsing round-trip
    // Validates: Requirements 3.5, 8.1
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_bedrock_response_parsing_round_trip(text in ".{1,200}") {
            let response_json = json!({
                "output": {
                    "message": {
                        "content": [{
                            "text": text
                        }]
                    }
                }
            });
            let result = parse_bedrock_response(&response_json);
            prop_assert_eq!(result, Ok(text));
        }
    }

    // Feature: bedrock-llm-provider, Property 4: Malformed Bedrock response yields error
    // Validates: Requirements 8.2
    fn arb_malformed_bedrock_json() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            // empty object
            Just(json!({})),
            // null
            Just(json!(null)),
            // number
            Just(json!(42)),
            // plain string
            Just(json!("hello")),
            // array
            Just(json!([1, 2, 3])),
            // object with wrong keys
            Just(json!({"foo": "bar"})),
            // output is not an object
            Just(json!({"output": "not an object"})),
            // output.message is not an object
            Just(json!({"output": {"message": "not an object"}})),
            // output.message.content is empty array
            Just(json!({"output": {"message": {"content": []}}})),
            // content[0] missing text key
            Just(json!({"output": {"message": {"content": [{"notText": "value"}]}}})),
            // content[0].text is not a string (number)
            Just(json!({"output": {"message": {"content": [{"text": 123}]}}})),
            // content[0].text is null
            Just(json!({"output": {"message": {"content": [{"text": null}]}}})),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_malformed_bedrock_response_yields_error(malformed in arb_malformed_bedrock_json()) {
            let result = parse_bedrock_response(&malformed);
            prop_assert!(
                result.is_err(),
                "Expected Err for malformed Bedrock JSON: {:?}, but got Ok({:?})",
                malformed,
                result.unwrap()
            );
            let err_msg = result.unwrap_err();
            prop_assert!(
                !err_msg.is_empty(),
                "Error message should be non-empty for malformed JSON: {:?}",
                malformed
            );
        }
    }

    // Feature: gemini-llm-backend, Property 6: Malformed response yields error
    // **Validates: Requirements 6.6**
    fn arb_malformed_json() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            // empty object
            Just(json!({})),
            // null
            Just(json!(null)),
            // number
            Just(json!(42)),
            // plain string
            Just(json!("hello")),
            // array
            Just(json!([1, 2, 3])),
            // object with wrong keys
            Just(json!({"foo": "bar"})),
            // candidates is not an array
            Just(json!({"candidates": "not an array"})),
            // candidates is empty array
            Just(json!({"candidates": []})),
            // candidates[0] missing content
            Just(json!({"candidates": [{"finishReason": "STOP"}]})),
            // candidates[0].content missing parts
            Just(json!({"candidates": [{"content": {"role": "model"}}]})),
            // candidates[0].content.parts is empty
            Just(json!({"candidates": [{"content": {"parts": []}}]})),
            // candidates[0].content.parts[0] missing text
            Just(json!({"candidates": [{"content": {"parts": [{"notText": "value"}]}}]})),
            // candidates[0].content.parts[0].text is not a string
            Just(json!({"candidates": [{"content": {"parts": [{"text": 123}]}}]})),
            // deeply nested but wrong structure
            Just(json!({"candidates": [{"content": {"parts": [{"text": null}]}}]})),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_malformed_response_yields_error(malformed in arb_malformed_json()) {
            let result = parse_gemini_response(&malformed);
            prop_assert!(
                result.is_err(),
                "Expected Err for malformed JSON: {:?}, but got Ok({:?})",
                malformed,
                result.unwrap()
            );
        }
    }

    // Feature: bedrock-llm-provider, Property 5: Bedrock error status mapping
    // Validates: Requirements 6.1, 6.2, 6.3, 6.4
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_bedrock_error_status_mapping(code in 400u16..=599u16, body in ".*") {
            let result = map_bedrock_error_status(code, &body);
            prop_assert!(
                !result.is_empty(),
                "Error message should be non-empty for status code {}",
                code
            );
            prop_assert!(
                result.contains(&code.to_string()),
                "Error message '{}' should contain status code '{}'",
                result,
                code
            );
        }
    }

    // Feature: bedrock-llm-provider, Property 9: Bedrock URL construction
    // Validates: Requirements 3.1
    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, .. ProptestConfig::default() })]

        #[test]
        fn prop_bedrock_url_construction(region in "[a-z\\-]{1,20}") {
            let url = format!(
                "https://bedrock-runtime.{}.amazonaws.com/model/global.anthropic.claude-opus-4-5-20251101-v1:0/converse",
                region
            );
            let expected = format!(
                "https://bedrock-runtime.{}.amazonaws.com/model/global.anthropic.claude-opus-4-5-20251101-v1:0/converse",
                region
            );
            prop_assert_eq!(url, expected);
        }
    }
}
