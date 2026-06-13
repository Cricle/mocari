use crate::{
    Result,
    core::{KeyformAxis, compute_keyform_axis_interval, expand_keyform_runtime_slots},
};

use super::{
    Moc3CountInfo, Moc3Header, Moc3SectionOffsets,
    parse::{read_f32_section, read_i32_section, to_usize},
};

const PARAMETER_MAX_VALUES_SLOT: usize = 51;
const PARAMETER_MIN_VALUES_SLOT: usize = 52;
const PARAMETER_DEFAULT_VALUES_SLOT: usize = 53;
const KEYFORM_BINDING_INDICES_SLOT: usize = 72;
const KEYFORM_BINDING_BAND_BEGIN_INDICES_SLOT: usize = 73;
const KEYFORM_BINDING_BAND_COUNTS_SLOT: usize = 74;
const KEYFORM_BINDING_KEYS_BEGIN_INDICES_SLOT: usize = 75;
const KEYFORM_BINDING_KEYS_COUNTS_SLOT: usize = 76;
const KEY_VALUES_SLOT: usize = 77;

#[derive(Debug, Clone, PartialEq)]
pub struct Moc3KeyformBindings {
    parameter_min_values: Vec<f32>,
    parameter_max_values: Vec<f32>,
    parameter_default_values: Vec<f32>,
    keyform_binding_indices: Vec<i32>,
    band_begin_indices: Vec<i32>,
    band_counts: Vec<i32>,
    keys_begin_indices: Vec<i32>,
    keys_counts: Vec<i32>,
    key_values: Vec<f32>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct Moc3KeyformSlot {
    pub(super) local_index: usize,
    pub(super) weight: f32,
}

impl Moc3KeyformBindings {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Moc3Header::parse(bytes)?;
        let offsets = Moc3SectionOffsets::parse(bytes)?;
        let counts = Moc3CountInfo::parse(bytes)?;
        let endianness = header.endianness();
        let parameter_count = to_usize(counts.parameters(), "parameter count")?;

        Ok(Self {
            parameter_min_values: read_f32_section(
                bytes,
                &offsets,
                PARAMETER_MIN_VALUES_SLOT,
                parameter_count,
                endianness,
            )?,
            parameter_max_values: read_f32_section(
                bytes,
                &offsets,
                PARAMETER_MAX_VALUES_SLOT,
                parameter_count,
                endianness,
            )?,
            parameter_default_values: read_f32_section(
                bytes,
                &offsets,
                PARAMETER_DEFAULT_VALUES_SLOT,
                parameter_count,
                endianness,
            )?,
            keyform_binding_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_INDICES_SLOT,
                to_usize(
                    counts.parameter_binding_indices(),
                    "keyform binding index count",
                )?,
                endianness,
            )?,
            band_begin_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_BAND_BEGIN_INDICES_SLOT,
                to_usize(counts.keyform_bindings(), "keyform binding band count")?,
                endianness,
            )?,
            band_counts: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_BAND_COUNTS_SLOT,
                to_usize(counts.keyform_bindings(), "keyform binding band count")?,
                endianness,
            )?,
            keys_begin_indices: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_KEYS_BEGIN_INDICES_SLOT,
                to_usize(counts.parameter_bindings(), "keyform binding count")?,
                endianness,
            )?,
            keys_counts: read_i32_section(
                bytes,
                &offsets,
                KEYFORM_BINDING_KEYS_COUNTS_SLOT,
                to_usize(counts.parameter_bindings(), "keyform binding count")?,
                endianness,
            )?,
            key_values: read_f32_section(
                bytes,
                &offsets,
                KEY_VALUES_SLOT,
                to_usize(counts.keys(), "key count")?,
                endianness,
            )?,
        })
    }

    pub fn parameter_default_values(&self) -> &[f32] {
        &self.parameter_default_values
    }

    pub fn parameter_min_values(&self) -> &[f32] {
        &self.parameter_min_values
    }

    pub fn parameter_max_values(&self) -> &[f32] {
        &self.parameter_max_values
    }

    pub fn default_keyform_index(&self, band_index: i32, keyform_count: usize) -> Option<usize> {
        self.keyform_slots(band_index, keyform_count, &self.parameter_default_values)?
            .into_iter()
            .max_by(|left, right| left.weight.total_cmp(&right.weight))
            .map(|slot| slot.local_index)
    }

    pub(super) fn keyform_slots(
        &self,
        band_index: i32,
        keyform_count: usize,
        parameter_values: &[f32],
    ) -> Option<Vec<Moc3KeyformSlot>> {
        if keyform_count == 0 {
            return None;
        }

        if band_index < 0 {
            return Some(vec![Moc3KeyformSlot {
                local_index: 0,
                weight: 1.0,
            }]);
        }

        let bindings = self.band_keyform_bindings(band_index)?;
        if bindings.is_empty() {
            return Some(vec![Moc3KeyformSlot {
                local_index: 0,
                weight: 1.0,
            }]);
        }

        let mut axes = Vec::with_capacity(bindings.len());
        let mut stride = 1usize;
        for &binding_index in bindings {
            let binding_index = usize::try_from(binding_index).ok()?;
            let keys = self.binding_keys(binding_index)?;
            let parameter_value = parameter_values
                .get(binding_index)
                .copied()
                .unwrap_or(0.0);
            let interval = compute_keyform_axis_interval(keys, parameter_value)?;
            let active_index = interval.left_index() + usize::from(interval.t() != 0.0);
            if active_index >= keys.len() {
                return None;
            }
            axes.push(KeyformAxis::new(
                interval.left_index(),
                interval.t(),
                stride,
            ));
            stride = stride.checked_mul(keys.len())?;
        }

        let slots = expand_keyform_runtime_slots(&axes)
            .into_iter()
            .map(|slot| {
                (slot.flat_index() < keyform_count).then_some(Moc3KeyformSlot {
                    local_index: slot.flat_index(),
                    weight: slot.weight(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(slots)
    }

    fn band_keyform_bindings(&self, band_index: i32) -> Option<&[i32]> {
        let band_index = usize::try_from(band_index).ok()?;
        let begin = usize::try_from(*self.band_begin_indices.get(band_index)?).ok()?;
        let len = usize::try_from(*self.band_counts.get(band_index)?).ok()?;
        self.keyform_binding_indices
            .get(begin..begin.checked_add(len)?)
    }

    fn binding_keys(&self, binding_index: usize) -> Option<&[f32]> {
        let begin = usize::try_from(*self.keys_begin_indices.get(binding_index)?).ok()?;
        let len = usize::try_from(*self.keys_counts.get(binding_index)?).ok()?;
        self.key_values.get(begin..begin.checked_add(len)?)
    }
}
