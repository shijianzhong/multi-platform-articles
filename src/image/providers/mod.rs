use super::{ImageError, ImageProvider};

mod gemini;
mod modelscope;
mod openai;
mod openrouter;
mod tuzi;

pub fn build_provider(
    provider: &str,
    api_key: String,
    api_base: Option<String>,
    model: Option<String>,
    size: Option<String>,
) -> Result<Box<dyn ImageProvider>, ImageError> {
    match provider {
        "" | "openai" => Ok(Box::new(openai::OpenAIProvider::new(
            api_key, api_base, model, size,
        )?)),
        "tuzi" => Ok(Box::new(tuzi::TuZiProvider::new(
            api_key, api_base, model, size,
        )?)),
        "modelscope" | "ms" => Ok(Box::new(modelscope::ModelScopeProvider::new(
            api_key, api_base, model, size,
        )?)),
        "openrouter" | "or" => Ok(Box::new(openrouter::OpenRouterProvider::new(
            api_key, api_base, model, size,
        )?)),
        "gemini" | "google" => Ok(Box::new(gemini::GeminiProvider::new(
            api_key, api_base, model, size,
        )?)),
        other => Err(ImageError::provider(
            "ImageProvider",
            format!(
                "未知的图片服务提供者: {other}. 支持: openai, tuzi, modelscope(ms), openrouter(or), gemini(google)"
            ),
        )),
    }
}

