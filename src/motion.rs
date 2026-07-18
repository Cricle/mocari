//! Cubism motion playback.
//!
//! Load a motion with [`crate::motion::load_motion`], wrap it in a
//! [`MotionPlayer`], and call [`MotionPlayer::tick`] plus [`MotionPlayer::apply`]
//! each frame before updating runtime meshes.

use std::{fs, path::Path};

use crate::{
    json::{
        Motion3, apply_motion_fade, motion_fade_in_weight, motion_fade_out_weight,
        parameter_curve_fade_weight,
    },
    runtime::ModelRuntime,
};

const PARAMETER_TARGET: &str = "Parameter";
const PART_OPACITY_TARGET: &str = "PartOpacity";
const DRAWABLE_TARGET: &str = "Drawable";

#[derive(Debug, Clone)]
/// Plays a parsed `motion3.json` animation against a [`ModelRuntime`].
///
/// A player owns playback time and blend weight. It does not update meshes by
/// itself; call [`ModelRuntime::update_meshes`] after applying one or more
/// players.
pub struct MotionPlayer {
    motion: Motion3,
    time: f32,
    weight: f32,
    looping: bool,
    finished: bool,
    last_event_time: f32,
    event_cursor: usize,
}

impl MotionPlayer {
    /// Creates a player at time `0.0` with full weight.
    ///
    /// The player follows the motion's own `Loop` metadata.
    pub fn new(motion: Motion3) -> Self {
        let looping = motion.meta().is_looping();
        Self::with_looping(motion, looping)
    }

    /// Creates a player that stops at the end, ignoring the motion's `Loop` metadata.
    pub fn new_once(motion: Motion3) -> Self {
        Self::with_looping(motion, false)
    }

    /// Creates a player with an explicit loop mode.
    pub fn with_looping(motion: Motion3, looping: bool) -> Self {
        Self {
            motion,
            time: 0.0,
            weight: 1.0,
            looping,
            finished: false,
            last_event_time: 0.0,
            event_cursor: 0,
        }
    }

    /// Returns the motion data owned by this player.
    pub fn motion(&self) -> &Motion3 {
        &self.motion
    }

    /// Returns the current playback time in seconds.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Returns the player's global blend weight.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Returns whether this player wraps at the motion duration.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Sets the player's global blend weight, clamped to `0.0..=1.0`.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight.clamp(0.0, 1.0);
    }

    /// Returns whether a one-shot motion has reached its end.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Restarts playback from the beginning.
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.finished = false;
        self.last_event_time = 0.0;
        self.event_cursor = 0;
    }

    /// Advances playback time by `delta_seconds`.
    ///
    /// Negative deltas are treated as zero. Looping motions wrap at their
    /// declared duration; non-looping motions stop at the end.
    pub fn tick(&mut self, delta_seconds: f32) {
        if self.finished {
            return;
        }

        self.last_event_time = self.time;
        self.time += delta_seconds.max(0.0);
        let duration = self.motion.meta().duration();
        if duration <= 0.0 {
            return;
        }

        if self.looping {
            if self.time >= duration {
                self.time %= duration;
                self.event_cursor = 0;
                self.last_event_time = 0.0;
            }
        } else if self.time >= duration {
            self.time = duration;
            self.finished = true;
        }
    }

    /// Returns user data events whose time falls within the last tick delta.
    ///
    /// Call this after `tick()` to process events that fired this frame.
    /// Works correctly with looping motions.
    ///
    /// Returns a `Vec` rather than an iterator because the borrow checker
    /// cannot express the lifetime of `&str` references borrowed through
    /// `&mut self` when the event cursor is also mutated.
    pub fn drain_events(&mut self) -> Vec<&str> {
        let motion_data = self.motion.user_data();
        let mut events = Vec::new();
        while self.event_cursor < motion_data.len() {
            let event = &motion_data[self.event_cursor];
            if event.time() > self.last_event_time && event.time() <= self.time {
                events.push(event.value());
            }
            if event.time() > self.time {
                break;
            }
            self.event_cursor += 1;
        }
        events
    }

    /// Applies the current motion sample to a model runtime.
    ///
    /// Curves targeting unknown parameters or parts are ignored. Call
    /// [`ModelRuntime::update_meshes`] after all motion and expression players
    /// have been applied for the frame.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let duration = self.motion.meta().duration();
        let end_time = if self.looping { -1.0 } else { duration };
        let fade_in = motion_fade_in_weight(self.time, 0.0, 0.0);
        let fade_out = motion_fade_out_weight(self.time, end_time, 0.0);

        for curve in self.motion.curves() {
            let Some(sampled) = curve.sample(self.time) else {
                continue;
            };
            let curve_weight = parameter_curve_fade_weight(
                self.weight,
                fade_in,
                fade_out,
                curve.fade_in_time(),
                curve.fade_out_time(),
                self.time,
                0.0,
                end_time,
            );

            match curve.target() {
                PARAMETER_TARGET => {
                    let Some(index) = runtime.parameter_index(curve.id()) else {
                        continue;
                    };
                    let Some(current) = runtime.parameter_value_by_index(index) else {
                        continue;
                    };
                    let value = apply_motion_fade(current, sampled, curve_weight);
                    runtime.set_parameter_by_index(index, value);
                }
                PART_OPACITY_TARGET => {
                    let Some(index) = runtime.part_index(curve.id()) else {
                        continue;
                    };
                    let value = apply_motion_fade(1.0, sampled, curve_weight);
                    runtime.set_part_opacity_by_index(index, value);
                }
                DRAWABLE_TARGET => {
                    let Some((drawable_id, field)) = curve.id().rsplit_once('.') else {
                        continue;
                    };
                    let Some(drawable_index) = runtime.drawable_index(drawable_id) else {
                        continue;
                    };
                    match field {
                        "Opacity" => {
                            let current = runtime.meshes().get(drawable_index).map(|m| m.opacity()).unwrap_or(1.0);
                            let faded = apply_motion_fade(current, sampled, curve_weight);
                            runtime.set_drawable_opacity_override(drawable_index, faded);
                        }
                        "DrawOrder" => {
                            runtime.set_drawable_draw_order_override(drawable_index, sampled);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Apply vertex position deformation curves
        for vertex_curve in self.motion.vertex_curves() {
            let drawable_id = vertex_curve.id();
            let Some(drawable_index) = runtime.drawable_index(drawable_id) else {
                continue;
            };
            let sampled = vertex_curve.sample(self.time);
            if !sampled.is_empty() {
                runtime.set_drawable_vertex_override(drawable_index, sampled.to_vec());
            }
        }
    }
}

/// Priority level for motions in a [`MotionManager`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MotionPriority {
    /// Only plays if no other motions are active.
    Idle,
    /// Normal priority. Queues (FIFO) if another normal motion is playing.
    Normal,
    /// Immediately starts, crossfading out current motions.
    Force,
}

#[derive(Debug, Clone)]
struct ManagedMotion {
    player: MotionPlayer,
    priority: MotionPriority,
    group: String,
    fade_in_remaining: f32,
    fading_out: bool,
}

/// Priority-based motion queue with crossfade blending.
///
/// Manages multiple active [`MotionPlayer`]s with priority levels and
/// crossfade transitions. Same-group motions replace each other;
/// different-group motions can play simultaneously.
///
/// ```no_run
/// use mocari::motion::{MotionManager, MotionPriority};
/// # use mocari::motion::load_motion;
/// # let motion = load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
/// let mut manager = MotionManager::new();
/// manager.start_motion(motion, MotionPriority::Normal, "Idle");
/// ```
#[derive(Debug, Clone)]
pub struct MotionManager {
    players: Vec<ManagedMotion>,
    crossfade_duration: f32,
    queue: Vec<(Motion3, MotionPriority, String)>,
}

impl MotionManager {
    /// Creates an empty motion manager with a 0.5 second crossfade.
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            crossfade_duration: 0.5,
            queue: Vec::new(),
        }
    }

    /// Sets the crossfade duration in seconds.
    pub fn set_crossfade_duration(&mut self, seconds: f32) {
        self.crossfade_duration = seconds.max(0.0);
    }

    /// Returns the crossfade duration in seconds.
    pub fn crossfade_duration(&self) -> f32 {
        self.crossfade_duration
    }

    /// Starts a motion with the given priority and group.
    ///
    /// Returns the number of active motions after starting.
    pub fn start_motion(&mut self, motion: Motion3, priority: MotionPriority, group: &str) -> usize {
        match priority {
            MotionPriority::Idle => {
                if self.players.iter().any(|p| !p.fading_out) {
                    // Other motions are active, idle can't start
                    return self.players.len();
                }
                self.start_motion_internal(motion, priority, group);
            }
            MotionPriority::Normal => {
                // Check if same group is already playing
                let same_group_index = self.players.iter().position(|p| p.group == group && !p.fading_out);
                if let Some(index) = same_group_index {
                    // Crossfade: start fading out old, start new
                    self.players[index].fading_out = true;
                    self.players[index].fade_in_remaining = self.crossfade_duration;
                    self.start_motion_internal(motion, priority, group);
                } else if self.players.iter().any(|p| p.priority == MotionPriority::Normal && !p.fading_out) {
                    // Queue if a normal motion is playing
                    self.queue.push((motion, priority, group.to_owned()));
                    return self.players.len();
                } else {
                    self.start_motion_internal(motion, priority, group);
                }
            }
            MotionPriority::Force => {
                // Fade out all current motions
                for player in &mut self.players {
                    if !player.fading_out {
                        player.fading_out = true;
                        player.fade_in_remaining = self.crossfade_duration;
                    }
                }
                self.start_motion_internal(motion, priority, group);
            }
        }
        self.players.len()
    }

    /// Starts a motion from a file path.
    pub fn start_motion_from_path(
        &mut self,
        path: &str,
        priority: MotionPriority,
        group: &str,
    ) -> Result<usize, MotionLoadError> {
        let motion = load_motion(path)?;
        Ok(self.start_motion(motion, priority, group))
    }

    /// Advances all active motions by `delta_seconds`.
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        let crossfade = self.crossfade_duration;

        for managed in &mut self.players {
            managed.player.tick(dt);
            if managed.fading_out {
                managed.fade_in_remaining -= dt;
                let progress = ((crossfade - managed.fade_in_remaining) / crossfade).clamp(0.0, 1.0);
                managed.player.set_weight(1.0 - progress);
            } else if managed.fade_in_remaining > 0.0 {
                managed.fade_in_remaining -= dt;
                let progress = ((crossfade - managed.fade_in_remaining) / crossfade).clamp(0.0, 1.0);
                managed.player.set_weight(progress);
            } else {
                managed.player.set_weight(1.0);
            }
        }

        // Remove finished motions and motions that have fully faded out
        self.players.retain(|m| {
            !m.player.is_finished()
                && m.fade_in_remaining > -0.1
                && !(m.fading_out && m.player.weight() <= 0.01)
        });

        // Process queue: start queued motions if slots are available
        if !self.queue.is_empty() && !self.players.iter().any(|p| p.priority == MotionPriority::Normal && !p.fading_out) {
            let (motion, priority, group) = self.queue.remove(0);
            self.start_motion_internal(motion, priority, &group);
        }
    }

    /// Applies all active motions to the runtime.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        for managed in &self.players {
            managed.player.apply(runtime);
        }
    }

    /// Drains events from all active motion players.
    ///
    /// Returns event values from all players whose time ranges include events
    /// since their last tick. Call this after [`tick`](Self::tick).
    pub fn drain_events(&mut self) -> Vec<String> {
        self.players
            .iter_mut()
            .flat_map(|m| {
                m.player
                    .drain_events()
                    .into_iter()
                    .map(String::from)
            })
            .collect()
    }

    /// Stops all motions immediately.
    pub fn stop_all(&mut self) {
        self.players.clear();
        self.queue.clear();
    }

    /// Stops all motions with a fade-out.
    pub fn stop_all_with_fade(&mut self, fade_seconds: f32) {
        let fade = fade_seconds.max(0.0);
        for player in &mut self.players {
            if !player.fading_out {
                player.fading_out = true;
                player.fade_in_remaining = fade;
            }
        }
        self.queue.clear();
    }

    /// Returns whether there are no active or queued motions.
    pub fn is_finished(&self) -> bool {
        self.players.is_empty() && self.queue.is_empty()
    }

    /// Returns the number of active motions (including those fading out).
    pub fn active_count(&self) -> usize {
        self.players.len()
    }

    /// Removes all active and queued motions.
    pub fn clear(&mut self) {
        self.players.clear();
        self.queue.clear();
    }

    fn start_motion_internal(&mut self, motion: Motion3, priority: MotionPriority, group: &str) {
        let crossfade = self.crossfade_duration;
        let mut player = MotionPlayer::new(motion);
        if crossfade > 0.0 {
            player.set_weight(0.0);
        }
        self.players.push(ManagedMotion {
            player,
            priority,
            group: group.to_owned(),
            fade_in_remaining: crossfade,
            fading_out: false,
        });
    }
}

impl Default for MotionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads a Cubism `motion3.json` file from disk.
pub fn load_motion(path: impl AsRef<Path>) -> Result<Motion3, MotionLoadError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| MotionLoadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Motion3::from_json_str(&source).map_err(MotionLoadError::Parse)
}

#[derive(Debug, thiserror::Error)]
/// Errors that can occur while loading a motion file.
pub enum MotionLoadError {
    /// The motion file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path of the file that failed to load.
        path: String,
        /// Original I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The motion JSON was invalid or unsupported.
    #[error("failed to parse motion3: {0}")]
    Parse(#[source] crate::Error),
}
