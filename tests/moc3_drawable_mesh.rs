use rusty_live2d::moc3::{
    Moc3ArtMeshInfo, Moc3ArtMeshKeyformInfo, Moc3ArtMeshKeyforms, Moc3ArtMeshes,
    Moc3DrawableBlendMode, build_moc3_drawable_mesh, build_moc3_drawable_meshes,
};

#[test]
fn builds_moc3_drawable_mesh_from_art_mesh_sections() {
    let art_meshes = Moc3ArtMeshes::from_parts(
        vec![Moc3ArtMeshInfo::new(2, 0b0000_0100, 6, 0, 0, 4, 0, 1)],
        vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
        vec![0, 1, 2, 0, 2, 3],
        vec![7],
    )
    .unwrap();
    let keyforms = Moc3ArtMeshKeyforms::from_parts(
        vec![0],
        vec![1],
        vec![4],
        vec![Moc3ArtMeshKeyformInfo::new(0.8, 500.0, 0)],
        vec![-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0],
    )
    .unwrap();

    let mesh = build_moc3_drawable_mesh(&art_meshes, &keyforms, 0, 0).unwrap();

    assert_eq!(mesh.texture_index(), 2);
    assert_eq!(mesh.drawable_flags(), 0b0000_0100);
    assert_eq!(mesh.opacity(), 0.8);
    assert_eq!(mesh.draw_order(), 500.0);
    assert_eq!(mesh.masks(), &[7]);
    assert_eq!(mesh.indices(), &[0, 1, 2, 0, 2, 3]);
    assert_eq!(mesh.vertices().len(), 4);
    assert_eq!(mesh.vertices()[0].position(), [-1.0, -1.0]);
    assert_eq!(mesh.vertices()[0].uv(), [0.0, 0.0]);
    assert_eq!(mesh.vertices()[2].position(), [1.0, 1.0]);
    assert_eq!(mesh.vertices()[2].uv(), [1.0, 1.0]);
}

#[test]
fn resolves_drawable_blend_mode_from_core_flags() {
    assert_eq!(
        Moc3DrawableBlendMode::from_flags(0),
        Moc3DrawableBlendMode::Normal
    );
    assert_eq!(
        Moc3DrawableBlendMode::from_flags(1 << 0),
        Moc3DrawableBlendMode::Additive
    );
    assert_eq!(
        Moc3DrawableBlendMode::from_flags(1 << 1),
        Moc3DrawableBlendMode::Multiplicative
    );
}

#[test]
fn rejects_moc3_drawable_mesh_with_out_of_range_indices() {
    let art_meshes = Moc3ArtMeshes::from_parts(
        vec![Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 2, 0, 0)],
        vec![0.0, 0.0, 1.0, 1.0],
        vec![0, 1, 2],
        Vec::new(),
    )
    .unwrap();
    let keyforms = Moc3ArtMeshKeyforms::from_parts(
        vec![0],
        vec![1],
        vec![2],
        vec![Moc3ArtMeshKeyformInfo::new(1.0, 0.0, 0)],
        vec![0.0, 0.0, 1.0, 1.0],
    )
    .unwrap();

    assert!(build_moc3_drawable_mesh(&art_meshes, &keyforms, 0, 0).is_none());
}

#[test]
fn builds_all_moc3_drawable_meshes_from_first_keyform() {
    let art_meshes = Moc3ArtMeshes::from_parts(
        vec![
            Moc3ArtMeshInfo::new(0, 0, 3, 0, 0, 3, 0, 0),
            Moc3ArtMeshInfo::new(1, 4, 3, 6, 3, 3, 0, 0),
        ],
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5, 1.0, 0.5, 0.5, 1.0],
        vec![0, 1, 2, 0, 1, 2],
        Vec::new(),
    )
    .unwrap();
    let keyforms = Moc3ArtMeshKeyforms::from_parts(
        vec![0, 1],
        vec![1, 1],
        vec![3, 3],
        vec![
            Moc3ArtMeshKeyformInfo::new(1.0, 10.0, 0),
            Moc3ArtMeshKeyformInfo::new(0.5, 20.0, 6),
        ],
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 2.0, 2.0, 3.0, 2.0, 2.0, 3.0],
    )
    .unwrap();

    let meshes = build_moc3_drawable_meshes(&art_meshes, &keyforms).unwrap();

    assert_eq!(meshes.len(), 2);
    assert_eq!(meshes[0].texture_index(), 0);
    assert_eq!(meshes[1].texture_index(), 1);
    assert_eq!(meshes[1].opacity(), 0.5);
    assert_eq!(meshes[1].vertices()[0].position(), [2.0, 2.0]);
}
