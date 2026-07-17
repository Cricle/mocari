//! Auto-animation features that make models feel alive without manual parameter tweaking.
//!
//! Each struct owns its state, exposes [`tick`](EyeBlink::tick) and
//! [`apply`](EyeBlink::apply) methods, and can be used independently.
//!
//! ```no_run
//! use mocari::auto::{EyeBlink, Breath};
//! # use mocari::assets::load_model_runtime;
//! # let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
//! # let runtime = model.runtime_mut();
//! let mut blink = EyeBlink::with_defaults();
//! blink.tick(1.0 / 60.0);
//! blink.apply(runtime);
//! ```

mod breath;
mod eye_blink;
mod lip_sync;
mod mouse_tracker;

pub use breath::{Breath, BreathConfig};
pub use eye_blink::{EyeBlink, EyeBlinkConfig};
pub use lip_sync::{LipSync, LipSyncConfig};
pub use mouse_tracker::{MouseTracker, MouseTrackerConfig};
