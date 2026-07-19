use crate::types::{Motion, MotionCurve, Parameter};
use std::f32::consts::PI;

/// Easing functions for motion curves.
#[expect(dead_code)]
fn ease_in(t: f32) -> f32 {
    t * t
}

#[expect(dead_code)]
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

#[expect(dead_code)]
fn elastic_out(t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = (2.0 * PI) / 3.0;
    2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

struct ParamSpec {
    min: f32,
    max: f32,
    default: f32,
}

fn build_specs(params: &[Parameter]) -> std::collections::HashMap<String, ParamSpec> {
    params
        .iter()
        .map(|p| {
            (
                p.id.clone(),
                ParamSpec {
                    min: p.min,
                    max: p.max,
                    default: p.default,
                },
            )
        })
        .collect()
}

fn clamp_val(specs: &std::collections::HashMap<String, ParamSpec>, id: &str, val: f32) -> f32 {
    if let Some(s) = specs.get(id) {
        val.clamp(s.min, s.max)
    } else {
        val
    }
}

/// Generate motions for a single type (for parallel processing).
pub fn generate_motions_for_type(parameters: &[Parameter], motion_type: &str) -> Vec<Motion> {
    let specs = build_specs(parameters);
    let param_ids: Vec<&str> = parameters.iter().map(|p| p.id.as_str()).collect();

    match motion_type {
        "idle" => vec![
            breathing_motion(&specs, &param_ids),
            blink_motion(&specs, &param_ids),
            sway_motion(&specs, &param_ids),
        ],
        "tap" => vec![
            tap_motion(&specs, &param_ids, "head", "Head"),
            tap_motion(&specs, &param_ids, "body", "Body"),
        ],
        "move" => vec![
            walk_motion(&specs, &param_ids),
            wave_motion(&specs, &param_ids),
            sit_motion(&specs, &param_ids),
        ],
        "emotional" => vec![
            emotion_motion(&specs, &param_ids, "happy", "Happy"),
            emotion_motion(&specs, &param_ids, "surprised", "Surprised"),
            emotion_motion(&specs, &param_ids, "thinking", "Thinking"),
        ],
        _ => Vec::new(),
    }
}

type Frame = (f32, Vec<(String, f32)>);
fn frames_to_curves(frames: &[Frame]) -> Vec<MotionCurve> {
    let mut all_params = std::collections::HashSet::new();
    for (_, vals) in frames {
        for (id, _) in vals {
            all_params.insert(id.clone());
        }
    }

    let mut curves = Vec::new();
    for param_id in all_params {
        let mut segments = Vec::new();
        for (time, vals) in frames {
            if let Some((_, value)) = vals.iter().find(|(id, _)| id == &param_id) {
                segments.push(*time);
                segments.push(*value);
            }
        }
        if !segments.is_empty() {
            curves.push(MotionCurve {
                target: "Model".into(),
                id: param_id,
                segments,
            });
        }
    }
    curves
}

fn make_motion(
    name: &str,
    motion_type: &str,
    duration: f32,
    fps: f32,
    is_loop: bool,
    frames: Vec<Frame>,
) -> Motion {
    Motion {
        name: name.into(),
        motion_type: motion_type.into(),
        duration,
        fps,
        is_loop,
        curves: frames_to_curves(&frames),
    }
}

fn breathing_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 3.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let breath = ((t * 2.0 * PI * 0.5).sin() + 1.0) / 2.0;
        let mut vals = Vec::new();
        if param_ids.contains(&"ParamBreath") {
            vals.push(("ParamBreath".into(), clamp_val(specs, "ParamBreath", breath)));
        }
        if param_ids.contains(&"ParamBodyAngleY") {
            vals.push(("ParamBodyAngleY".into(), clamp_val(specs, "ParamBodyAngleY", breath * 2.0)));
        }
        frames.push((t * duration, vals));
    }

    make_motion("Idle_Breath", "idle", duration, fps, true, frames)
}

fn blink_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 0.3;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let eye_open = if t < 0.5 { 1.0 - t * 2.0 } else { (t - 0.5) * 2.0 };
        let mut vals = Vec::new();
        for pid in param_ids {
            if pid.contains("Eye") && pid.contains("Open") {
                vals.push((pid.to_string(), clamp_val(specs, pid, eye_open)));
            }
        }
        frames.push((t * duration, vals));
    }

    make_motion("Idle_Blink", "idle", duration, fps, false, frames)
}

fn sway_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 4.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let sway_x = (t * 2.0 * PI * 0.25).sin() * 3.0;
        let sway_z = (t * 2.0 * PI * 0.2).cos() * 2.0;
        let mut vals = Vec::new();
        if param_ids.contains(&"ParamAngleX") {
            vals.push(("ParamAngleX".into(), clamp_val(specs, "ParamAngleX", sway_x)));
        }
        if param_ids.contains(&"ParamAngleZ") {
            vals.push(("ParamAngleZ".into(), clamp_val(specs, "ParamAngleZ", sway_z)));
        }
        frames.push((t * duration, vals));
    }

    make_motion("Idle_Sway", "idle", duration, fps, true, frames)
}

fn tap_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
    area: &str,
    area_name: &str,
) -> Motion {
    let duration = 0.5;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let mut vals = Vec::new();
        for pid in param_ids {
            if area == "head" && pid.contains("Angle") && !pid.contains("Body") {
                vals.push((pid.to_string(), clamp_val(specs, pid, (t * PI).sin() * 5.0)));
            } else if area == "body" && pid.contains("Body") {
                vals.push((pid.to_string(), clamp_val(specs, pid, (t * PI).sin() * 3.0)));
            }
        }
        frames.push((t * duration, vals));
    }

    make_motion(&format!("Tap_{area_name}"), "tap", duration, fps, false, frames)
}

fn walk_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 1.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let leg_angle = (t * 2.0 * PI).sin() * 30.0;
        let mut vals = Vec::new();
        if param_ids.contains(&"ParamLegL") {
            vals.push(("ParamLegL".into(), clamp_val(specs, "ParamLegL", leg_angle)));
        }
        if param_ids.contains(&"ParamLegR") {
            vals.push(("ParamLegR".into(), clamp_val(specs, "ParamLegR", -leg_angle)));
        }
        if param_ids.contains(&"ParamBodyAngleY") {
            vals.push(("ParamBodyAngleY".into(), clamp_val(specs, "ParamBodyAngleY", (t * 2.0 * PI).sin().abs() * 3.0)));
        }
        frames.push((t * duration, vals));
    }

    make_motion("Move_Walk", "move", duration, fps, true, frames)
}

fn wave_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 2.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let arm_angle = if t < 0.3 {
            -t * 200.0
        } else {
            -60.0 + ((t - 0.3) * 2.0 * PI * 2.0).sin() * 20.0
        };
        let mut vals = Vec::new();
        if param_ids.contains(&"ParamArmRA") {
            vals.push(("ParamArmRA".into(), clamp_val(specs, "ParamArmRA", arm_angle)));
        }
        frames.push((t * duration, vals));
    }

    make_motion("Move_Wave", "move", duration, fps, false, frames)
}

fn sit_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
) -> Motion {
    let duration = 2.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let eased = ease_in_out(t);
        let mut vals = Vec::new();

        // Sit down: body angle Y goes negative, legs bend
        if param_ids.contains(&"ParamBodyAngleY") {
            let val = -15.0 * eased;
            vals.push(("ParamBodyAngleY".into(), clamp_val(specs, "ParamBodyAngleY", val)));
        }
        if param_ids.contains(&"ParamLegL") {
            let val = 45.0 * eased;
            vals.push(("ParamLegL".into(), clamp_val(specs, "ParamLegL", val)));
        }
        if param_ids.contains(&"ParamLegR") {
            let val = 45.0 * eased;
            vals.push(("ParamLegR".into(), clamp_val(specs, "ParamLegR", val)));
        }
        frames.push((t * duration, vals));
    }

    make_motion("Move_Sit", "move", duration, fps, false, frames)
}

fn emotion_keyframes(emotion: &str) -> std::collections::HashMap<String, f32> {
    let mut m = std::collections::HashMap::new();
    match emotion {
        "happy" => {
            m.insert("ParamEyeLSmile".into(), 1.0);
            m.insert("ParamEyeRSmile".into(), 1.0);
            m.insert("ParamMouthOpenY".into(), 0.3);
            m.insert("ParamMouthForm".into(), 1.0);
            m.insert("ParamBrowLY".into(), -0.3);
            m.insert("ParamBrowRY".into(), -0.3);
            m.insert("ParamAngleY".into(), -5.0);
        }
        "surprised" => {
            m.insert("ParamEyeLOpen".into(), 1.5);
            m.insert("ParamEyeROpen".into(), 1.5);
            m.insert("ParamMouthOpenY".into(), 0.8);
            m.insert("ParamBrowLY".into(), 0.5);
            m.insert("ParamBrowRY".into(), 0.5);
            m.insert("ParamAngleY".into(), -10.0);
        }
        "thinking" => {
            m.insert("ParamEyeLOpen".into(), 0.6);
            m.insert("ParamEyeROpen".into(), 0.6);
            m.insert("ParamMouthForm".into(), -0.3);
            m.insert("ParamBrowLY".into(), 0.2);
            m.insert("ParamBrowRY".into(), -0.3);
            m.insert("ParamAngleX".into(), 5.0);
            m.insert("ParamAngleY".into(), 5.0);
        }
        _ => {}
    }
    m
}

fn emotion_motion(
    specs: &std::collections::HashMap<String, ParamSpec>,
    param_ids: &[&str],
    emotion: &str,
    emotion_name: &str,
) -> Motion {
    let duration = 2.0;
    let fps = 30.0;
    let num_frames = (duration * fps) as usize;
    let keyframes = emotion_keyframes(emotion);
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let t = i as f32 / num_frames as f32;
        let blend = if t < 0.3 {
            ease_in_out(t / 0.3)
        } else if t > 0.7 {
            ease_in_out((1.0 - t) / 0.3)
        } else {
            1.0
        };

        let mut vals = Vec::new();
        for pid in param_ids {
            if let Some(&target) = keyframes.get(*pid)
                && let Some(s) = specs.get(*pid)
            {
                let value = s.default + (target - s.default) * blend;
                vals.push((pid.to_string(), clamp_val(specs, pid, value)));
            }
        }
        frames.push((t * duration, vals));
    }

    make_motion(
        &format!("Emotion_{emotion_name}"),
        "emotional",
        duration,
        fps,
        false,
        frames,
    )
}
