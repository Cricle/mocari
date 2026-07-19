//! Mutable state for driving a loaded model.
//!
//! [`ModelRuntime`] is the type most applications update every frame. It stores
//! parameter values, part opacity overrides, pose state, and the current drawable
//! meshes. After changing parameters or applying a player from [`crate::motion`]
//! or [`crate::expression`], call [`ModelRuntime::update_meshes`] before drawing.

use std::collections::HashMap;

use crate::{
    core::{PhysicsOptions, PhysicsRuntime, clamp_parameter_value, draw_order_from_raw},
    json::{Model3, Physics3, Pose3, UserData3, UserDataTarget, copy_pose_link_opacities, update_pose_group_opacities},
    moc3::{
        Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CanvasInfo, Moc3Deformers, Moc3DrawOrderGroups,
        Moc3DrawableMesh, Moc3DrawableVertex, Moc3Glues, Moc3Ids, Moc3KeyformBindings,
        Moc3MeshUpdateScratch, Moc3OffscreenInfo, Moc3Parts,
        build_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities,
        update_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities,
    },
};

#[derive(Debug, Clone)]
struct PoseGroup {
    members: Vec<usize>,
    links: Vec<Vec<usize>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// A read-only view of one model parameter.
///
/// Values are reported in the model's native parameter range. Use
/// [`normalized_value`](Self::normalized_value) when UI code wants a stable
/// `0.0..=1.0` representation.
pub struct ParameterInfo<'a> {
    id: &'a str,
    minimum: f32,
    maximum: f32,
    default: f32,
    value: f32,
}

impl<'a> ParameterInfo<'a> {
    /// Returns the Cubism parameter id, such as `ParamAngleX`.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Returns the minimum value declared by the model.
    pub fn minimum(&self) -> f32 {
        self.minimum
    }

    /// Returns the maximum value declared by the model.
    pub fn maximum(&self) -> f32 {
        self.maximum
    }

    /// Returns the default value declared by the model.
    pub fn default(&self) -> f32 {
        self.default
    }

    /// Returns the current runtime value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Returns the current value mapped into `0.0..=1.0`.
    ///
    /// If the model declares an invalid range where `maximum <= minimum`, this
    /// returns `0.0`.
    pub fn normalized_value(&self) -> f32 {
        normalized_parameter_value(self.value, self.minimum, self.maximum)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// A hit area that contains a tested model-space point.
///
/// Hit areas come from `HitAreas` in `.model3.json`. Their ids reference
/// drawable ids in the loaded `.moc3` file, and their names are the user-facing
/// labels commonly used to choose tap motions such as `TapBody`.
pub struct HitAreaInfo<'a> {
    id: &'a str,
    name: &'a str,
    drawable_index: usize,
}

impl<'a> HitAreaInfo<'a> {
    /// Returns the hit area's drawable id.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Returns the user-facing hit area name, such as `Head` or `Body`.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the model-order drawable index used for hit testing.
    pub fn drawable_index(&self) -> usize {
        self.drawable_index
    }
}

#[derive(Debug, Clone)]
/// Runtime state for a loaded Live2D/Cubism-compatible model.
///
/// The runtime owns the current parameter values and the generated drawable
/// meshes. A typical frame updates parameters, applies motion and expression
/// players, applies pose fading if needed, then calls [`update_meshes`](Self::update_meshes).
///
/// Applications usually create this through [`crate::assets::load_model_runtime`]
/// instead of calling [`new`](Self::new) directly.
pub struct ModelRuntime {
    model: Model3,
    canvas: Moc3CanvasInfo,
    art_meshes: Moc3ArtMeshes,
    art_mesh_keyforms: Moc3ArtMeshKeyforms,
    deformers: Moc3Deformers,
    bindings: Moc3KeyformBindings,
    ids: Moc3Ids,
    offscreen: Moc3OffscreenInfo,
    glues: Moc3Glues,
    parts: Moc3Parts,
    draw_order_groups: Option<Moc3DrawOrderGroups>,
    drawable_index: HashMap<Box<str>, usize>,
    parameter_index: HashMap<Box<str>, usize>,
    parameter_values: Vec<f32>,
    parameter_overrides: Vec<Option<f32>>,
    physics: Option<PhysicsRuntime>,
    part_index: HashMap<Box<str>, usize>,
    part_opacity_overrides: Vec<Option<f32>>,
    part_opacities: Vec<f32>,
    pose_groups: Vec<PoseGroup>,
    pose_fade_time: f32,
    pose_opacities: Vec<f32>,
    meshes: Vec<Moc3DrawableMesh>,
    mesh_update_scratch: Moc3MeshUpdateScratch,
    drawable_visible: Vec<bool>,
    drawable_multiply_overrides: Vec<Option<[f32; 3]>>,
    drawable_screen_overrides: Vec<Option<[f32; 3]>>,
    drawable_opacity_overrides: Vec<Option<f32>>,
    drawable_draw_order_overrides: Vec<Option<f32>>,
    drawable_vertex_overrides: Vec<Option<Vec<f32>>>,
    user_data: Option<UserData3>,
    // Scratch buffers to avoid per-frame allocations
    scratch_drawable_part_opacities: Vec<f32>,
    scratch_pose_selection: Vec<f32>,
    scratch_pose_faded: Vec<f32>,
    scratch_drawable_draw_orders: Vec<i32>,
    scratch_part_draw_orders: Vec<i32>,
    scratch_part_enable: Vec<bool>,
    // Dirty tracking: when false after a frame, skip mesh rebuild
    dirtied: bool,
    parameter_values_generation: u64,
}

impl ModelRuntime {
    /// Builds a runtime from already parsed model components.
    ///
    /// This constructor is intended for custom loaders and tests. It returns
    /// `None` when the parsed parts cannot produce a valid initial mesh set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: Model3,
        canvas: Moc3CanvasInfo,
        art_meshes: Moc3ArtMeshes,
        art_mesh_keyforms: Moc3ArtMeshKeyforms,
        deformers: Moc3Deformers,
        bindings: Moc3KeyformBindings,
        ids: Moc3Ids,
        offscreen: Moc3OffscreenInfo,
        glues: Moc3Glues,
        parts: Moc3Parts,
        draw_order_groups: Option<Moc3DrawOrderGroups>,
        pose: Option<Pose3>,
    ) -> Option<Self> {
        let parameter_values = bindings.parameter_default_values().to_vec();
        let parameter_overrides = vec![None; parameter_values.len()];
        let drawable_index = build_index(ids.art_meshes());
        let drawable_count = ids.art_meshes().len();
        let parameter_index = build_index(ids.parameters());
        let part_index = build_index(ids.parts());
        let part_count = parts.part_count();

        let pose_fade_time = pose
            .as_ref()
            .map(Pose3::resolved_fade_in_time)
            .unwrap_or_default();
        let pose_groups = pose
            .as_ref()
            .map(|pose| build_pose_groups(pose, &part_index))
            .unwrap_or_default();
        let pose_opacities = initial_pose_opacities(&pose_groups, part_count);

        let mut runtime = Self {
            model,
            canvas,
            art_meshes,
            art_mesh_keyforms,
            deformers,
            bindings,
            ids,
            offscreen,
            glues,
            parts,
            draw_order_groups,
            drawable_index,
            parameter_index,
            parameter_values,
            parameter_overrides,
            physics: None,
            part_index,
            part_opacity_overrides: vec![None; part_count],
            part_opacities: vec![1.0; part_count],
            pose_groups,
            pose_fade_time,
            pose_opacities,
            meshes: Vec::new(),
            mesh_update_scratch: Moc3MeshUpdateScratch::default(),
            drawable_visible: vec![true; drawable_count],
            drawable_multiply_overrides: vec![None; drawable_count],
            drawable_screen_overrides: vec![None; drawable_count],
            drawable_opacity_overrides: vec![None; drawable_count],
            drawable_draw_order_overrides: vec![None; drawable_count],
            drawable_vertex_overrides: vec![None; drawable_count],
            user_data: None,
            scratch_drawable_part_opacities: Vec::new(),
            scratch_pose_selection: Vec::new(),
            scratch_pose_faded: Vec::new(),
            scratch_drawable_draw_orders: Vec::new(),
            scratch_part_draw_orders: Vec::new(),
            scratch_part_enable: Vec::new(),
            dirtied: true,
            parameter_values_generation: 0,
        };
        runtime.update_meshes()?;
        Some(runtime)
    }

    /// Returns the parsed `.model3.json` data associated with this runtime.
    pub fn model(&self) -> &Model3 {
        &self.model
    }

    /// Returns the model canvas information parsed from the `.moc3` file.
    pub fn canvas(&self) -> Moc3CanvasInfo {
        self.canvas
    }

    /// Returns all parameter ids in model order.
    pub fn parameter_ids(&self) -> &[String] {
        self.ids.parameters()
    }

    /// Returns all drawable ids in model order.
    pub fn drawable_ids(&self) -> &[String] {
        self.ids.art_meshes()
    }

    /// Returns the model-order drawable index for a drawable id.
    pub fn drawable_index(&self, id: &str) -> Option<usize> {
        self.drawable_index.get(id).copied()
    }

    /// Returns the model-order index for a parameter id.
    ///
    /// Cache this index in hot paths and use the `*_by_index` methods to avoid a
    /// string lookup each frame.
    pub fn parameter_index(&self, id: &str) -> Option<usize> {
        self.parameter_index.get(id).copied()
    }

    /// Returns the current value for a parameter id.
    pub fn parameter_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_values.get(index).copied()
    }

    /// Returns the current value for a parameter index.
    pub fn parameter_value_by_index(&self, index: usize) -> Option<f32> {
        self.parameter_values.get(index).copied()
    }

    /// Returns all current parameter values in model order.
    pub fn parameter_values(&self) -> &[f32] {
        &self.parameter_values
    }

    /// Returns metadata and the current value for a parameter id.
    pub fn parameter_info(&self, id: &str) -> Option<ParameterInfo<'_>> {
        let index = self.parameter_index(id)?;
        self.parameter_info_by_index(index)
    }

    /// Returns metadata and the current value for a parameter index.
    pub fn parameter_info_by_index(&self, index: usize) -> Option<ParameterInfo<'_>> {
        let (minimum, maximum) = self.parameter_range_by_index(index)?;
        Some(ParameterInfo {
            id: self.ids.parameters().get(index)?.as_str(),
            minimum,
            maximum,
            default: self.parameter_default_by_index(index)?,
            value: self.parameter_value_by_index(index)?,
        })
    }

    /// Iterates over all parameters with their ranges and current values.
    pub fn parameter_infos(&self) -> impl Iterator<Item = ParameterInfo<'_>> + '_ {
        (0..self.ids.parameters().len()).filter_map(|index| self.parameter_info_by_index(index))
    }

    /// Returns the declared minimum value for a parameter index.
    pub fn parameter_minimum_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_min_values().get(index).copied()
    }

    /// Returns the declared maximum value for a parameter index.
    pub fn parameter_maximum_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_max_values().get(index).copied()
    }

    /// Returns the declared default value for a parameter index.
    pub fn parameter_default_by_index(&self, index: usize) -> Option<f32> {
        self.bindings.parameter_default_values().get(index).copied()
    }

    /// Returns the current parameter value mapped into `0.0..=1.0`.
    pub fn parameter_normalized_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_normalized_value_by_index(index)
    }

    /// Returns the current parameter value for an index mapped into `0.0..=1.0`.
    pub fn parameter_normalized_value_by_index(&self, index: usize) -> Option<f32> {
        let minimum = self.parameter_minimum_by_index(index)?;
        let maximum = self.parameter_maximum_by_index(index)?;
        let value = self.parameter_value_by_index(index)?;
        Some(normalized_parameter_value(value, minimum, maximum))
    }

    /// Sets a parameter by id, clamping the value to the model's declared range.
    ///
    /// Returns `false` when the id is not present in the model.
    pub fn set_parameter(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_by_index(index, value),
            None => false,
        }
    }

    /// Sets a parameter by index, clamping the value to the model's declared range.
    ///
    /// Returns `false` when the index is out of range.
    pub fn set_parameter_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.parameter_values.get_mut(index) else {
            return false;
        };
        let (minimum, maximum) = parameter_clamp_range(&self.bindings, index);
        let clamped = clamp_parameter_value(value, minimum, maximum);
        if *slot != clamped {
            *slot = clamped;
            self.parameter_values_generation = self.parameter_values_generation.wrapping_add(1);
            self.dirtied = true;
        }
        true
    }

    /// Sets a parameter with a normalized `0.0..=1.0` value.
    pub fn set_parameter_normalized(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_normalized_by_index(index, value),
            None => false,
        }
    }

    /// Sets a parameter by index with a normalized `0.0..=1.0` value.
    pub fn set_parameter_normalized_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(raw) = self.raw_parameter_value_from_normalized_index(index, value) else {
            return false;
        };
        self.set_parameter_by_index(index, raw)
    }

    /// Returns the pending override value for a parameter id.
    ///
    /// Overrides are separate from current parameter values until
    /// [`apply_parameter_overrides`](Self::apply_parameter_overrides) is called.
    pub fn parameter_override_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_override_value_by_index(index)
    }

    /// Returns the pending override value for a parameter index.
    pub fn parameter_override_value_by_index(&self, index: usize) -> Option<f32> {
        self.parameter_overrides.get(index).copied().flatten()
    }

    /// Returns the pending override value mapped into `0.0..=1.0`.
    pub fn parameter_override_normalized_value(&self, id: &str) -> Option<f32> {
        let index = self.parameter_index(id)?;
        self.parameter_override_normalized_value_by_index(index)
    }

    /// Returns the pending override value for an index mapped into `0.0..=1.0`.
    pub fn parameter_override_normalized_value_by_index(&self, index: usize) -> Option<f32> {
        let minimum = self.parameter_minimum_by_index(index)?;
        let maximum = self.parameter_maximum_by_index(index)?;
        let value = self.parameter_override_value_by_index(index)?;
        Some(normalized_parameter_value(value, minimum, maximum))
    }

    /// Stores a parameter override by id without immediately changing the value.
    pub fn set_parameter_override(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_override_by_index(index, value),
            None => false,
        }
    }

    /// Stores a parameter override by index without immediately changing the value.
    pub fn set_parameter_override_by_index(&mut self, index: usize, value: f32) -> bool {
        if index >= self.parameter_overrides.len() {
            return false;
        }
        let Some((minimum, maximum)) = self.parameter_range_by_index(index) else {
            return false;
        };
        let clamped = clamp_parameter_value(value, minimum, maximum);
        if self.parameter_overrides[index] != Some(clamped) {
            self.parameter_overrides[index] = Some(clamped);
            self.dirtied = true;
        }
        true
    }

    /// Stores a normalized parameter override by id.
    pub fn set_parameter_override_normalized(&mut self, id: &str, value: f32) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.set_parameter_override_normalized_by_index(index, value),
            None => false,
        }
    }

    /// Stores a normalized parameter override by index.
    pub fn set_parameter_override_normalized_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(raw) = self.raw_parameter_value_from_normalized_index(index, value) else {
            return false;
        };
        self.set_parameter_override_by_index(index, raw)
    }

    /// Clears a pending override for a parameter id.
    pub fn clear_parameter_override(&mut self, id: &str) -> bool {
        match self.parameter_index(id) {
            Some(index) => self.clear_parameter_override_by_index(index),
            None => false,
        }
    }

    /// Clears a pending override for a parameter index.
    pub fn clear_parameter_override_by_index(&mut self, index: usize) -> bool {
        let Some(slot) = self.parameter_overrides.get_mut(index) else {
            return false;
        };
        if slot.is_some() {
            *slot = None;
            self.dirtied = true;
        }
        true
    }

    /// Clears all pending parameter overrides.
    pub fn clear_parameter_overrides(&mut self) {
        if self.parameter_overrides.iter().any(|o| o.is_some()) {
            self.parameter_overrides.fill(None);
            self.dirtied = true;
        }
    }

    /// Applies all pending parameter overrides to the current parameter values.
    pub fn apply_parameter_overrides(&mut self) {
        for index in 0..self.parameter_overrides.len() {
            if let Some(value) = self.parameter_overrides[index] {
                self.set_parameter_by_index(index, value);
            }
        }
    }

    fn raw_parameter_value_from_normalized_index(&self, index: usize, value: f32) -> Option<f32> {
        let (minimum, maximum) = self.parameter_range_by_index(index)?;
        Some(raw_parameter_value_from_normalized_range(
            minimum, maximum, value,
        ))
    }

    /// Returns the first hit area containing a model-space point.
    ///
    /// Coordinates must be in the same model space as drawable vertices. UI code
    /// that starts from window pixels should first invert its render transform.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<HitAreaInfo<'_>> {
        self.hit_test_all(x, y).next()
    }

    /// Returns all hit areas containing a model-space point.
    pub fn hit_test_all(&self, x: f32, y: f32) -> impl Iterator<Item = HitAreaInfo<'_>> + '_ {
        self.model.hit_areas().iter().filter_map(move |hit_area| {
            let drawable_index = self.drawable_index(hit_area.id())?;
            let mesh = self.meshes.get(drawable_index)?;
            drawable_contains_point(mesh, x, y).then_some(HitAreaInfo {
                id: hit_area.id(),
                name: hit_area.name(),
                drawable_index,
            })
        })
    }

    /// Resets current parameter values to the defaults declared by the model.
    pub fn reset_parameters(&mut self) {
        self.parameter_values
            .copy_from_slice(self.bindings.parameter_default_values());
    }

    /// Returns the user data loaded from `userdata3.json`, if any.
    pub fn user_data(&self) -> Option<&UserData3> {
        self.user_data.as_ref()
    }

    /// Finds a user data value by target type and element id.
    pub fn find_user_data(&self, target: &UserDataTarget, id: &str) -> Option<&str> {
        self.user_data.as_ref().and_then(|data| data.find(target, id))
    }

    /// Sets user data on the runtime.
    pub fn set_user_data(&mut self, data: UserData3) {
        self.user_data = Some(data);
    }

    pub fn set_physics(&mut self, physics: Physics3) {
        self.physics = Some(PhysicsRuntime::new(&physics, self.ids.parameters()));
    }

    pub fn clear_physics(&mut self) {
        self.physics = None;
    }

    pub fn physics(&self) -> Option<&PhysicsRuntime> {
        self.physics.as_ref()
    }

    pub fn physics_options(&self) -> Option<PhysicsOptions> {
        self.physics.as_ref().map(PhysicsRuntime::options)
    }

    pub fn set_physics_options(&mut self, options: PhysicsOptions) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.set_options(options);
        true
    }

    pub fn reset_physics(&mut self) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.reset();
        true
    }

    pub fn stabilize_physics(&mut self) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.stabilize(
            &mut self.parameter_values,
            self.bindings.parameter_min_values(),
            self.bindings.parameter_max_values(),
            self.bindings.parameter_default_values(),
        );
        true
    }

    pub fn apply_physics(&mut self, delta_time_seconds: f32) -> bool {
        let Some(physics) = &mut self.physics else {
            return false;
        };
        physics.evaluate(
            &mut self.parameter_values,
            self.bindings.parameter_min_values(),
            self.bindings.parameter_max_values(),
            self.bindings.parameter_default_values(),
            delta_time_seconds,
        );
        self.dirtied = true;
        true
    }

    /// Returns all part ids in model order.
    pub fn part_ids(&self) -> &[String] {
        self.ids.parts()
    }

    /// Returns the model-order index for a part id.
    pub fn part_index(&self, id: &str) -> Option<usize> {
        self.part_index.get(id).copied()
    }

    /// Returns the current part opacity for an index (before overrides).
    pub fn part_opacity_value(&self, index: usize) -> Option<f32> {
        self.part_opacities.get(index).copied()
    }

    /// Overrides a part opacity by id.
    ///
    /// Values are clamped to `0.0..=1.0`. Pose fading can still affect the final
    /// drawable opacity after this override is applied.
    pub fn set_part_opacity(&mut self, id: &str, value: f32) -> bool {
        match self.part_index(id) {
            Some(index) => self.set_part_opacity_by_index(index, value),
            None => false,
        }
    }

    /// Overrides a part opacity by index.
    pub fn set_part_opacity_by_index(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.part_opacity_overrides.get_mut(index) else {
            return false;
        };
        let clamped = value.clamp(0.0, 1.0);
        if *slot != Some(clamped) {
            *slot = Some(clamped);
            self.dirtied = true;
        }
        true
    }

    /// Clears all part opacity overrides.
    pub fn reset_part_opacities(&mut self) {
        if self.part_opacity_overrides.iter().any(|o| o.is_some()) {
            self.part_opacity_overrides.iter_mut().for_each(|o| *o = None);
            self.dirtied = true;
        }
    }

    /// Advances pose fade state by `delta_seconds`.
    ///
    /// Call this once per frame for models that include a `pose3.json` file.
    pub fn apply_pose(&mut self, delta_seconds: f32) {
        if self.pose_groups.is_empty() {
            return;
        }
        for group in &self.pose_groups {
            let len = group.members.len();
            self.scratch_pose_selection.clear();
            self.scratch_pose_selection.reserve(len);
            self.scratch_pose_faded.clear();
            self.scratch_pose_faded.reserve(len);
            for &part in &group.members {
                self.scratch_pose_selection.push(self.part_selection_opacity(part));
                self.scratch_pose_faded.push(self.pose_opacities[part]);
            }

            if update_pose_group_opacities(
                &self.scratch_pose_selection,
                &mut self.scratch_pose_faded,
                delta_seconds,
                self.pose_fade_time,
            )
            .is_none()
            {
                continue;
            }

            for (opacity, &part) in self.scratch_pose_faded.iter().zip(&group.members) {
                self.pose_opacities[part] = *opacity;
            }
            for (member_position, &part) in group.members.iter().enumerate() {
                let _ = copy_pose_link_opacities(
                    &mut self.pose_opacities,
                    part,
                    &group.links[member_position],
                );
            }
            self.dirtied = true;
        }
    }

    fn part_selection_opacity(&self, part_index: usize) -> f32 {
        self.part_opacity_overrides[part_index].unwrap_or_else(|| {
            self.parts
                .interpolate_opacity(part_index, &self.bindings, &self.parameter_values)
                .unwrap_or(1.0)
        })
    }

    fn part_drawable_opacity(&self, part_index: usize) -> f32 {
        self.part_opacity_overrides[part_index].unwrap_or(1.0)
    }

    fn update_part_opacities(&mut self) {
        if self.part_opacity_overrides.iter().any(|o| o.is_some()) {
            self.update_direct_part_opacities();
            self.apply_parent_part_opacities();
        } else {
            self.update_part_opacities_no_overrides();
        }
    }

    fn update_part_opacities_no_overrides(&mut self) {
        for index in 0..self.part_opacities.len() {
            self.part_opacities[index] = self.pose_opacities[index];
        }
        for index in 0..self.part_opacities.len() {
            let mut opacity = self.part_opacities[index];
            let mut parent = self.parts.parent_part_index(index);
            while let Some(parent_index) = parent.and_then(|p| usize::try_from(p).ok()) {
                opacity *= self.part_opacities[parent_index];
                parent = self.parts.parent_part_index(parent_index);
            }
            self.part_opacities[index] = opacity;
        }
    }

    fn update_direct_part_opacities(&mut self) {
        for index in 0..self.part_opacities.len() {
            let base = self.part_drawable_opacity(index);
            self.part_opacities[index] = base * self.pose_opacities[index];
        }
    }

    fn apply_parent_part_opacities(&mut self) {
        for index in 0..self.part_opacities.len() {
            let mut opacity = self.part_opacities[index];
            let mut parent = self.parts.parent_part_index(index);
            while let Some(parent_index) = parent.and_then(|p| usize::try_from(p).ok()) {
                opacity *= self.part_opacities[parent_index];
                parent = self.parts.parent_part_index(parent_index);
            }
            self.part_opacities[index] = opacity;
        }
    }

    fn update_drawable_part_opacities(&mut self) {
        let count = self.art_meshes.meshes().len();
        self.scratch_drawable_part_opacities.clear();
        self.scratch_drawable_part_opacities.reserve(count);
        for drawable_index in 0..count {
            let opacity = self
                .offscreen
                .drawable_parent_part_index(drawable_index)
                .and_then(|p| usize::try_from(p).ok())
                .and_then(|part_index| self.part_opacities.get(part_index).copied())
                .unwrap_or(1.0);
            self.scratch_drawable_part_opacities.push(opacity);
        }
    }

    /// Rebuilds drawable meshes from the current runtime state.
    ///
    /// Call this after changing parameters, applying motion or expression
    /// players, changing part opacities, or advancing pose state. Returns `None`
    /// when the model data cannot produce a valid mesh update.
    pub fn update_meshes(&mut self) -> Option<()> {
        if !self.dirtied {
            return Some(());
        }
        self.update_part_opacities();
        self.update_drawable_part_opacities();
        self.rebuild_or_update_meshes()?;
        self.apply_mesh_post_processing()?;
        self.apply_drawable_color_overrides();
        self.apply_drawable_motion_overrides();
        self.apply_drawable_visibility();
        self.dirtied = false;
        Some(())
    }

    /// Forces the next [`ModelRuntime::update_meshes`] call to rebuild meshes
    /// from scratch, even if no parameter or opacity change was recorded.
    ///
    /// Use this when mesh data was mutated externally and the dirty tracker
    /// would otherwise skip the rebuild.
    pub fn mark_dirty(&mut self) {
        self.dirtied = true;
    }

    /// Returns true when no parameter, opacity, or drawable override has changed
    /// since the last [`ModelRuntime::update_meshes`] call completed.
    ///
    /// When this returns `false`, calling `update_meshes` is a no-op.
    pub fn is_dirty(&self) -> bool {
        self.dirtied
    }

    fn rebuild_or_update_meshes(&mut self) -> Option<()> {
        if self.meshes.len() == self.art_meshes.meshes().len() {
            update_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities(
                &mut self.meshes,
                &mut self.mesh_update_scratch,
                &self.art_meshes,
                &self.art_mesh_keyforms,
                &self.deformers,
                &self.bindings,
                &self.ids,
                &self.offscreen,
                &self.parameter_values,
                &self.scratch_drawable_part_opacities,
            )?;
        } else {
            self.meshes = build_moc3_drawable_meshes_with_parameters_offscreen_and_part_opacities(
                &self.art_meshes,
                &self.art_mesh_keyforms,
                &self.deformers,
                &self.bindings,
                &self.ids,
                &self.offscreen,
                &self.parameter_values,
                &self.scratch_drawable_part_opacities,
            )?;
        }
        Some(())
    }

    fn apply_mesh_post_processing(&mut self) -> Option<()> {
        self.glues
            .apply(&mut self.meshes, &self.bindings, &self.parameter_values)?;
        self.apply_group_render_orders();
        Some(())
    }

    fn apply_drawable_color_overrides(&mut self) {
        for (index, mesh) in self.meshes.iter_mut().enumerate() {
            if let Some(color) = self.drawable_multiply_overrides.get(index).and_then(|c| *c) {
                mesh.set_multiply_color(color);
            }
            if let Some(color) = self.drawable_screen_overrides.get(index).and_then(|c| *c) {
                mesh.set_screen_color(color);
            }
        }
    }

    fn apply_drawable_motion_overrides(&mut self) {
        for (index, mesh) in self.meshes.iter_mut().enumerate() {
            if let Some(opacity) = self.drawable_opacity_overrides.get(index).and_then(|v| *v) {
                mesh.set_opacity(opacity);
            }
            if let Some(draw_order) = self.drawable_draw_order_overrides.get(index).and_then(|v| *v) {
                mesh.set_draw_order(draw_order);
            }
            if let Some(positions) = self.drawable_vertex_overrides.get(index).and_then(|v| v.as_ref()) {
                let vertices = mesh.vertices_mut();
                for (i, vertex) in vertices.iter_mut().enumerate() {
                    let px = positions.get(i * 2).copied().unwrap_or(vertex.position()[0]);
                    let py = positions.get(i * 2 + 1).copied().unwrap_or(vertex.position()[1]);
                    *vertex = Moc3DrawableVertex::new([px, py], vertex.uv());
                }
            }
        }
    }

    fn apply_drawable_visibility(&mut self) {
        for (index, mesh) in self.meshes.iter_mut().enumerate() {
            if !self.drawable_visible.get(index).copied().unwrap_or(true) {
                for vertex in mesh.vertices_mut() {
                    *vertex = Moc3DrawableVertex::new([0.0, 0.0], vertex.uv());
                }
            }
        }
    }

    fn apply_group_render_orders(&mut self) {
        let Some(groups) = self.draw_order_groups.as_ref() else {
            return;
        };

        self.scratch_drawable_draw_orders.clear();
        self.scratch_drawable_draw_orders.reserve(self.meshes.len());
        for mesh in &self.meshes {
            self.scratch_drawable_draw_orders.push(draw_order_from_raw(mesh.draw_order()));
        }

        let part_count = self.parts.part_count();
        self.scratch_part_draw_orders.clear();
        self.scratch_part_draw_orders.resize(part_count, 0);
        self.scratch_part_enable.clear();
        self.scratch_part_enable.resize(part_count, false);
        for index in 0..part_count {
            if let Some(raw) =
                self.parts
                    .interpolate_draw_order(index, &self.bindings, &self.parameter_values)
            {
                self.scratch_part_draw_orders[index] = draw_order_from_raw(raw);
                self.scratch_part_enable[index] = true;
            }
        }

        let Some(render_orders) = groups.render_orders(
            &self.scratch_drawable_draw_orders,
            &self.scratch_part_draw_orders,
            &self.scratch_part_enable,
            self.offscreen.part_offscreen_indices(),
            self.offscreen.offscreen_count(),
        ) else {
            return;
        };
        for (mesh, render_order) in self.meshes.iter_mut().zip(&render_orders) {
            mesh.set_render_order(*render_order);
        }
    }

    /// Sets whether a drawable is visible by id.
    ///
    /// Hidden drawables produce zero-area meshes. Returns `false` when the id is
    /// not present in the model.
    pub fn set_drawable_visible(&mut self, id: &str, visible: bool) -> bool {
        match self.drawable_index(id) {
            Some(index) => self.set_drawable_visible_by_index(index, visible),
            None => false,
        }
    }

    /// Sets whether a drawable is visible by index.
    pub fn set_drawable_visible_by_index(&mut self, index: usize, visible: bool) -> bool {
        let Some(slot) = self.drawable_visible.get_mut(index) else {
            return false;
        };
        if *slot != visible {
            *slot = visible;
            self.dirtied = true;
        }
        true
    }

    /// Returns whether a drawable is currently visible.
    pub fn is_drawable_visible(&self, index: usize) -> bool {
        self.drawable_visible.get(index).copied().unwrap_or(true)
    }

    /// Returns whether a drawable disables back-face culling.
    pub fn is_drawable_double_sided(&self, index: usize) -> bool {
        self.meshes
            .get(index)
            .map(|m| m.is_double_sided())
            .unwrap_or(false)
    }

    /// Resets all drawables to visible.
    pub fn reset_drawable_visibility(&mut self) {
        self.drawable_visible.fill(true);
    }

    /// Returns the pending multiply color override for a drawable index.
    pub fn drawable_multiply_color_override(&self, index: usize) -> Option<[f32; 3]> {
        self.drawable_multiply_overrides.get(index).copied().flatten()
    }

    /// Returns the pending screen color override for a drawable index.
    pub fn drawable_screen_color_override(&self, index: usize) -> Option<[f32; 3]> {
        self.drawable_screen_overrides.get(index).copied().flatten()
    }

    /// Sets a multiply color override by drawable id.
    pub fn set_drawable_multiply_color(&mut self, id: &str, color: [f32; 3]) -> bool {
        match self.drawable_index(id) {
            Some(index) => self.set_drawable_multiply_color_by_index(index, color),
            None => false,
        }
    }

    /// Sets a multiply color override by drawable index.
    pub fn set_drawable_multiply_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool {
        let Some(slot) = self.drawable_multiply_overrides.get_mut(index) else {
            return false;
        };
        if *slot != Some(color) {
            *slot = Some(color);
            self.dirtied = true;
        }
        true
    }

    /// Sets a screen color override by drawable id.
    pub fn set_drawable_screen_color(&mut self, id: &str, color: [f32; 3]) -> bool {
        match self.drawable_index(id) {
            Some(index) => self.set_drawable_screen_color_by_index(index, color),
            None => false,
        }
    }

    /// Sets a screen color override by drawable index.
    pub fn set_drawable_screen_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool {
        let Some(slot) = self.drawable_screen_overrides.get_mut(index) else {
            return false;
        };
        if *slot != Some(color) {
            *slot = Some(color);
            self.dirtied = true;
        }
        true
    }

    /// Clears all drawable color overrides.
    pub fn clear_drawable_color_overrides(&mut self) {
        let had_any = self.drawable_multiply_overrides.iter().any(|o| o.is_some())
            || self.drawable_screen_overrides.iter().any(|o| o.is_some());
        self.drawable_multiply_overrides.fill(None);
        self.drawable_screen_overrides.fill(None);
        if had_any {
            self.dirtied = true;
        }
    }

    /// Sets a drawable opacity override by index.
    pub fn set_drawable_opacity_override(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.drawable_opacity_overrides.get_mut(index) else {
            return false;
        };
        let clamped = value.clamp(0.0, 1.0);
        if *slot != Some(clamped) {
            *slot = Some(clamped);
            self.dirtied = true;
        }
        true
    }

    /// Sets a drawable draw order override by index.
    pub fn set_drawable_draw_order_override(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.drawable_draw_order_overrides.get_mut(index) else {
            return false;
        };
        if *slot != Some(value) {
            *slot = Some(value);
            self.dirtied = true;
        }
        true
    }

    /// Clears all drawable motion overrides (opacity, draw order, vertex positions).
    ///
    /// Call this at the start of each frame before applying motion players.
    pub fn clear_drawable_motion_overrides(&mut self) {
        self.drawable_opacity_overrides.fill(None);
        self.drawable_draw_order_overrides.fill(None);
        self.drawable_vertex_overrides.iter_mut().for_each(|o| *o = None);
    }

    /// Sets a vertex position override for a drawable.
    ///
    /// `positions` should contain interleaved x, y pairs for each vertex.
    pub fn set_drawable_vertex_override(&mut self, index: usize, positions: Vec<f32>) -> bool {
        let Some(slot) = self.drawable_vertex_overrides.get_mut(index) else {
            return false;
        };
        *slot = Some(positions);
        true
    }

    /// Clears all vertex position overrides.
    pub fn clear_drawable_vertex_overrides(&mut self) {
        self.drawable_vertex_overrides.iter_mut().for_each(|o| *o = None);
    }

    /// Returns the current multiply color on a mesh (after update_meshes).
    pub fn mesh_multiply_color(&self, index: usize) -> Option<[f32; 3]> {
        self.meshes.get(index).map(|m| m.multiply_color())
    }

    /// Returns the current screen color on a mesh (after update_meshes).
    pub fn mesh_screen_color(&self, index: usize) -> Option<[f32; 3]> {
        self.meshes.get(index).map(|m| m.screen_color())
    }

    /// Returns the current drawable meshes in model order.
    ///
    /// Sort with [`crate::render::common::draw_order_indices`] or use a renderer
    /// backend before issuing draw calls.
    pub fn meshes(&self) -> &[Moc3DrawableMesh] {
        &self.meshes
    }

    /// Returns a mutable reference to the drawable meshes.
    pub fn meshes_mut(&mut self) -> &mut [Moc3DrawableMesh] {
        &mut self.meshes
    }

    fn parameter_range_by_index(&self, index: usize) -> Option<(f32, f32)> {
        Some((
            self.parameter_minimum_by_index(index)?,
            self.parameter_maximum_by_index(index)?,
        ))
    }

    /// Builds an [`EyeBlinkConfig`](crate::auto::EyeBlinkConfig) from the model's Groups data.
    ///
    /// Reads groups named "EyeBlink" and extracts their parameter indices.
    /// Returns a default config if no EyeBlink group is found.
    pub fn eye_blink_config_from_model(&self) -> crate::auto::EyeBlinkConfig {
        let indices: Vec<usize> = self
            .model
            .groups()
            .iter()
            .filter(|g| g.name() == "EyeBlink" && g.target() == "Parameter")
            .flat_map(|g| g.ids().iter())
            .filter_map(|id| self.parameter_index(id))
            .collect();
        if indices.is_empty() {
            crate::auto::EyeBlinkConfig::default()
        } else {
            crate::auto::EyeBlinkConfig::for_parameters(indices)
        }
    }

    /// Builds a [`LipSyncConfig`](crate::auto::LipSyncConfig) from the model's Groups data.
    ///
    /// Reads groups named "LipSync" and extracts their parameter indices.
    /// Returns a default config if no LipSync group is found.
    pub fn lip_sync_config_from_model(&self) -> crate::auto::LipSyncConfig {
        let indices: Vec<usize> = self
            .model
            .groups()
            .iter()
            .filter(|g| g.name() == "LipSync" && g.target() == "Parameter")
            .flat_map(|g| g.ids().iter())
            .filter_map(|id| self.parameter_index(id))
            .collect();
        if indices.is_empty() {
            crate::auto::LipSyncConfig::default()
        } else {
            crate::auto::LipSyncConfig::for_parameters(indices)
        }
    }
}

fn normalized_parameter_value(value: f32, minimum: f32, maximum: f32) -> f32 {
    if maximum <= minimum {
        0.0
    } else {
        ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
    }
}

fn raw_parameter_value_from_normalized_range(minimum: f32, maximum: f32, value: f32) -> f32 {
    let amount = value.clamp(0.0, 1.0);
    minimum + (maximum - minimum) * amount
}

fn parameter_clamp_range(bindings: &Moc3KeyformBindings, index: usize) -> (f32, f32) {
    let minimum = bindings
        .parameter_min_values()
        .get(index)
        .copied()
        .unwrap_or(f32::MIN);
    let maximum = bindings
        .parameter_max_values()
        .get(index)
        .copied()
        .unwrap_or(f32::MAX);
    (minimum, maximum)
}

fn build_index(ids: &[String]) -> HashMap<Box<str>, usize> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| (id.as_str().into(), index))
        .collect()
}

fn drawable_contains_point(mesh: &Moc3DrawableMesh, x: f32, y: f32) -> bool {
    let Some(first) = mesh.vertices().first() else {
        return false;
    };

    let [first_x, first_y] = first.position();
    let mut min_x = first_x;
    let mut min_y = first_y;
    let mut max_x = first_x;
    let mut max_y = first_y;

    for vertex in mesh.vertices().iter().skip(1) {
        let [vertex_x, vertex_y] = vertex.position();
        min_x = min_x.min(vertex_x);
        min_y = min_y.min(vertex_y);
        max_x = max_x.max(vertex_x);
        max_y = max_y.max(vertex_y);
    }

    (min_x..=max_x).contains(&x) && (min_y..=max_y).contains(&y)
}

fn build_pose_groups(pose: &Pose3, part_index: &HashMap<Box<str>, usize>) -> Vec<PoseGroup> {
    pose.groups()
        .iter()
        .filter_map(|group| {
            let mut members = Vec::new();
            let mut links = Vec::new();
            for part in group {
                let Some(&part_idx) = part_index.get(part.id()) else {
                    continue;
                };
                members.push(part_idx);
                links.push(
                    part.links()
                        .iter()
                        .filter_map(|link| part_index.get(link.as_str()).copied())
                        .collect(),
                );
            }
            (members.len() >= 2).then_some(PoseGroup { members, links })
        })
        .collect()
}

fn initial_pose_opacities(groups: &[PoseGroup], part_count: usize) -> Vec<f32> {
    let mut opacities = vec![1.0; part_count];
    for group in groups {
        for (position, &part) in group.members.iter().enumerate() {
            let opacity = if position == 0 { 1.0 } else { 0.0 };
            opacities[part] = opacity;
            for &link in &group.links[position] {
                opacities[link] = opacity;
            }
        }
    }
    opacities
}

#[cfg(test)]
mod tests {
    use crate::{
        assets::load_model_runtime,
        moc3::{Moc3ArtMeshInfo, Moc3ArtMeshes, Moc3OffscreenInfo, Moc3Parts},
    };

    #[test]
    fn part_keyform_opacity_does_not_drive_drawable_visibility() {
        let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
        let runtime = model.runtime_mut();
        runtime.art_meshes = Moc3ArtMeshes::from_parts(
            vec![Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 3, 0, 0)],
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            vec![0, 1, 2],
            Vec::new(),
        )
        .unwrap();
        runtime.offscreen = Moc3OffscreenInfo::from_parts(vec![-1], vec![0], vec![-1], Vec::new());
        runtime.parts =
            Moc3Parts::from_parts(vec![-1], vec![-1], vec![0], vec![1], vec![0.0], vec![0.0]);
        runtime.part_opacity_overrides = vec![None];
        runtime.part_opacities = vec![1.0];
        runtime.pose_opacities = vec![1.0];

        runtime.update_part_opacities();
        runtime.update_drawable_part_opacities();

        assert_eq!(runtime.scratch_drawable_part_opacities, vec![1.0]);
    }
}
