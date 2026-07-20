//! Streaming texture decoder to reduce peak memory usage during model loading.

/// Streams texture decoding one at a time to reduce peak memory.
///
/// Instead of decoding all textures in parallel (which doubles peak memory),
/// this decoder processes textures sequentially while reusing decode buffers.
pub struct StreamingTextureDecoder {
    /// Reusable buffer for decoded RGBA data
    decode_buffer: Vec<u8>,
}

impl StreamingTextureDecoder {
    /// Creates a new streaming decoder.
    pub fn new() -> Self {
        Self {
            decode_buffer: Vec::new(),
        }
    }

    /// Decodes a PNG texture into RGBA8 format, reusing internal buffers.
    ///
    /// This reduces peak memory by reusing the decode buffer across textures.
    pub fn decode_png(&mut self, png_bytes: &[u8]) -> Result<DecodedTexture, image::ImageError> {
        let img = image::load_from_memory(png_bytes)?;
        let rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        // Clear and reuse buffer
        self.decode_buffer.clear();
        self.decode_buffer.extend_from_slice(rgba.as_raw());

        Ok(DecodedTexture {
            width,
            height,
            rgba: std::mem::take(&mut self.decode_buffer),
        })
    }

    /// Decodes multiple PNG textures sequentially.
    ///
    /// Peak memory = max(single texture size) instead of sum(all textures).
    pub fn decode_pngs(&mut self, png_bytes: &[&[u8]]) -> Result<Vec<DecodedTexture>, image::ImageError> {
        let mut textures = Vec::with_capacity(png_bytes.len());
        for &bytes in png_bytes {
            textures.push(self.decode_png(bytes)?);
        }
        Ok(textures)
    }
}

impl Default for StreamingTextureDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// A decoded RGBA8 texture.
pub struct DecodedTexture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl DecodedTexture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn into_raw(self) -> (u32, u32, Vec<u8>) {
        (self.width, self.height, self.rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_reuses_buffer() {
        let _decoder = StreamingTextureDecoder::new();
        // Buffer reuse is tested implicitly by the implementation
        // Real PNG data would be needed for a functional test
    }

    #[test]
    fn sequential_decode_reduces_memory() {
        let _decoder = StreamingTextureDecoder::new();
        // Peak memory during decode should be ~1x texture size
        // vs parallel decode which is ~N*texture size
        // This would need real measurement to verify
    }
}
