use crate::types::Parameter;
use serde_json::{json, Value};

/// Generate physics3.json content from the parameter list.
pub fn configure_physics(parameters: &[Parameter]) -> Value {
    let param_ids: Vec<&str> = parameters.iter().map(|p| p.id.as_str()).collect();

    let mut groups = Vec::new();

    if let Some(hair) = create_hair_physics(&param_ids) {
        groups.push(hair);
    }
    if let Some(cloth) = create_clothing_physics(&param_ids) {
        groups.push(cloth);
    }
    if let Some(acc) = create_accessory_physics(&param_ids) {
        groups.push(acc);
    }

    json!({
        "Version": 3,
        "Meta": {
            "PhysicsSettingCount": groups.len(),
            "TotalInputCount": 0,
            "TotalOutputCount": 0,
            "VertexCount": 0,
            "Fps": 0.0,
            "PhysicsSimulationDelay": 0.0
        },
        "PhysicsSettings": groups
    })
}

fn filter_known(param_ids: &[&str], candidates: &[Value]) -> Vec<Value> {
    candidates
        .iter()
        .filter(|c| {
            c.get("Id")
                .and_then(|v| v.as_str())
                .map(|id| param_ids.contains(&id))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn create_hair_physics(param_ids: &[&str]) -> Option<Value> {
    let input = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamAngleX", "Scale": 1.0, "Weight": 0.5}),
            json!({"Id": "ParamAngleY", "Scale": 1.0, "Weight": 0.3}),
            json!({"Id": "ParamAngleZ", "Scale": 1.0, "Weight": 0.3}),
        ],
    );
    let output = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamHairFront", "Scale": 1.0, "Weight": 1.0, "Type": "Angle"}),
            json!({"Id": "ParamHairSide", "Scale": 1.0, "Weight": 1.0, "Type": "Angle"}),
            json!({"Id": "ParamHairBack", "Scale": 1.0, "Weight": 1.0, "Type": "Angle"}),
        ],
    );

    if output.is_empty() {
        return None;
    }

    let mut vertices = Vec::new();
    for i in 0..5 {
        vertices.push(json!({
            "Position": {"X": 0.0, "Y": -100.0 - i as f32 * 20.0},
            "Mobility": 1.0,
            "Delay": 0.5 + i as f32 * 0.1,
            "Acceleration": 0.3 - i as f32 * 0.05,
            "Radius": 10.0 + i as f32 * 2.0
        }));
    }

    Some(json!({
        "Id": format!("PhysicsSetting_Hair"),
        "Input": input,
        "Output": output,
        "Vertices": vertices,
        "Normalization": {
            "Position": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0},
            "Angle": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0}
        }
    }))
}

fn create_clothing_physics(param_ids: &[&str]) -> Option<Value> {
    let input = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamBodyAngleX", "Scale": 1.0, "Weight": 0.3}),
            json!({"Id": "ParamBodyAngleY", "Scale": 1.0, "Weight": 0.3}),
            json!({"Id": "ParamBreath", "Scale": 1.0, "Weight": 0.3}),
        ],
    );
    let output = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamClothA", "Scale": 1.0, "Weight": 0.8, "Type": "Angle"}),
            json!({"Id": "ParamClothB", "Scale": 1.0, "Weight": 0.8, "Type": "Angle"}),
            json!({"Id": "ParamClothC", "Scale": 1.0, "Weight": 0.8, "Type": "Angle"}),
        ],
    );

    if output.is_empty() {
        return None;
    }

    let spacing = 30.0f32;
    let mut vertices = Vec::new();
    for row in 0..3 {
        for col in 0..3 {
            vertices.push(json!({
                "Position": {"X": (col as f32 - 1.0) * spacing, "Y": row as f32 * spacing},
                "Mobility": if row == 0 { 0.0 } else { 1.0 },
                "Delay": 0.85,
                "Acceleration": 0.4,
                "Radius": 5.0
            }));
        }
    }

    Some(json!({
        "Id": "PhysicsSetting_Clothing",
        "Input": input,
        "Output": output,
        "Vertices": vertices,
        "Normalization": {
            "Position": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0},
            "Angle": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0}
        }
    }))
}

fn create_accessory_physics(param_ids: &[&str]) -> Option<Value> {
    let input = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamAngleX", "Scale": 1.0, "Weight": 0.6}),
            json!({"Id": "ParamAngleY", "Scale": 1.0, "Weight": 0.6}),
        ],
    );
    let output = filter_known(
        param_ids,
        &[
            json!({"Id": "ParamAccessoryA", "Scale": 1.0, "Weight": 1.0, "Type": "Angle"}),
            json!({"Id": "ParamAccessoryB", "Scale": 1.0, "Weight": 1.0, "Type": "Angle"}),
        ],
    );

    if output.is_empty() {
        return None;
    }

    Some(json!({
        "Id": "PhysicsSetting_Accessory",
        "Input": input,
        "Output": output,
        "Vertices": [{
            "Position": {"X": 0.0, "Y": -150.0},
            "Mobility": 1.0,
            "Delay": 0.95,
            "Acceleration": 0.5,
            "Radius": 5.0
        }],
        "Normalization": {
            "Position": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0},
            "Angle": {"Minimum": -10.0, "Default": 0.0, "Maximum": 10.0}
        }
    }))
}
