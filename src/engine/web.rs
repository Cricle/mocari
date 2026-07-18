//! Web-only fetch utilities for loading assets from URLs.

use wasm_bindgen::JsCast;

use super::EngineError;

/// Fetches bytes from a URL using the browser Fetch API.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, EngineError> {
    let resp_value = wasm_bindgen_futures::JsFuture::from(
        web_sys::window()
            .ok_or_else(|| EngineError::Surface("no window".into()))?
            .fetch_with_str(url),
    )
    .await
    .map_err(|e| EngineError::Surface(format!("fetch failed: {e:?}")))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| EngineError::Surface("invalid response".into()))?;

    if !resp.ok() {
        return Err(EngineError::Surface(format!(
            "HTTP {} from {url}",
            resp.status()
        )));
    }

    let array_buffer = wasm_bindgen_futures::JsFuture::from(
        resp.array_buffer()
            .map_err(|e| EngineError::Surface(format!("array_buffer failed: {e:?}")))?,
    )
    .await
    .map_err(|e| EngineError::Surface(format!("array_buffer read failed: {e:?}")))?;

    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

/// Fetches text from a URL using the browser Fetch API.
pub async fn fetch_text(url: &str) -> Result<String, EngineError> {
    let resp_value = wasm_bindgen_futures::JsFuture::from(
        web_sys::window()
            .ok_or_else(|| EngineError::Surface("no window".into()))?
            .fetch_with_str(url),
    )
    .await
    .map_err(|e| EngineError::Surface(format!("fetch failed: {e:?}")))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| EngineError::Surface("invalid response".into()))?;

    if !resp.ok() {
        return Err(EngineError::Surface(format!(
            "HTTP {} from {url}",
            resp.status()
        )));
    }

    let text = wasm_bindgen_futures::JsFuture::from(
        resp.text()
            .map_err(|e| EngineError::Surface(format!("text failed: {e:?}")))?,
    )
    .await
    .map_err(|e| EngineError::Surface(format!("text read failed: {e:?}")))?;

    text.as_string()
        .ok_or_else(|| EngineError::Surface("response is not a string".into()))
}
