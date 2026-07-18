use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowLevel};

use super::EngineError;

/// Configuration for a desktop pet window.
///
/// Defaults are tuned for a typical desktop pet: transparent background,
/// frameless, always on top, and click-through enabled.
pub struct DesktopPetConfig {
    pub transparent: bool,
    pub decorations: bool,
    pub always_on_top: bool,
    pub click_through: bool,
    pub size: (u32, u32),
    pub title: String,
}

impl Default for DesktopPetConfig {
    fn default() -> Self {
        Self {
            transparent: true,
            decorations: false,
            always_on_top: true,
            click_through: true,
            size: (400, 400),
            title: "Live2D Pet".into(),
        }
    }
}

impl DesktopPetConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn transparent(mut self, v: bool) -> Self {
        self.transparent = v;
        self
    }

    pub fn decorations(mut self, v: bool) -> Self {
        self.decorations = v;
        self
    }

    pub fn always_on_top(mut self, v: bool) -> Self {
        self.always_on_top = v;
        self
    }

    pub fn click_through(mut self, v: bool) -> Self {
        self.click_through = v;
        self
    }

    pub fn size(mut self, w: u32, h: u32) -> Self {
        self.size = (w, h);
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Builds winit `WindowAttributes` from this config.
    fn to_window_attributes(&self) -> WindowAttributes {
        let level = if self.always_on_top {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };

        Window::default_attributes()
            .with_title(&self.title)
            .with_transparent(self.transparent)
            .with_decorations(self.decorations)
            .with_window_level(level)
            .with_inner_size(winit::dpi::LogicalSize::new(self.size.0, self.size.1))
    }

    /// Creates a new window with this config.
    pub fn create_window(
        &self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Arc<Window>, EngineError> {
        let attrs = self.to_window_attributes();
        let window = event_loop
            .create_window(attrs)
            .map_err(|e| EngineError::Surface(e.to_string()))?;
        let window = Arc::new(window);

        if self.click_through {
            window
                .set_cursor_hittest(false)
                .map_err(|e| EngineError::Surface(e.to_string()))?;
        }

        Ok(window)
    }

    /// Applies the click-through setting to an existing window.
    pub fn apply_click_through(&self, window: &Window) -> Result<(), EngineError> {
        window
            .set_cursor_hittest(!self.click_through)
            .map_err(|e| EngineError::Surface(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = DesktopPetConfig::new();
        assert!(cfg.transparent);
        assert!(!cfg.decorations);
        assert!(cfg.always_on_top);
        assert!(cfg.click_through);
        assert_eq!(cfg.size, (400, 400));
        assert_eq!(cfg.title, "Live2D Pet");
    }

    #[test]
    fn builder_overrides() {
        let cfg = DesktopPetConfig::new()
            .transparent(false)
            .decorations(true)
            .always_on_top(false)
            .click_through(false)
            .size(800, 600)
            .title("My Pet");

        assert!(!cfg.transparent);
        assert!(cfg.decorations);
        assert!(!cfg.always_on_top);
        assert!(!cfg.click_through);
        assert_eq!(cfg.size, (800, 600));
        assert_eq!(cfg.title, "My Pet");
    }
}
