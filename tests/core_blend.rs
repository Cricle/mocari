use rusty_live2d::core::{
    BlendSlot, Rgb, blend_scalar_slots, blend_scalar_slots_clamped, multiply_rgb, screen_rgb,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn blends_scalar_slots_with_single_and_pair_kinds() {
    let values = [10.0, 20.0, 30.0, 40.0];
    let slots = [
        BlendSlot::Skip,
        BlendSlot::Single {
            base: 0,
            index: 1,
            weight: 0.5,
            final_weight: 0.25,
        },
        BlendSlot::Pair {
            base: 0,
            index0: 2,
            weight0: 0.75,
            index1: 3,
            weight1: 0.25,
            final_weight: 0.5,
        },
    ];

    assert_close(blend_scalar_slots(&slots, &values, 1.0).unwrap(), 19.75);
}

#[test]
fn clamps_scalar_blend_result() {
    let slots = [BlendSlot::Single {
        base: 0,
        index: 0,
        weight: 2.0,
        final_weight: 1.0,
    }];

    assert_eq!(
        blend_scalar_slots_clamped(&slots, &[0.75], 0.0, 0.0, 1.0).unwrap(),
        1.0
    );
}

#[test]
fn blends_multiply_and_screen_rgb() {
    let local = Rgb::new(0.25, 0.5, 1.25);
    let parent = Rgb::new(0.5, 0.25, 0.5);

    assert_eq!(multiply_rgb(local, parent), Rgb::new(0.125, 0.125, 0.625));
    assert_eq!(screen_rgb(local, parent), Rgb::new(0.625, 0.625, 1.0));
}
