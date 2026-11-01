#[test]
fn designer_sdf_wgsl_is_valid() {
    let source = include_str!("../src/designer_sdf.wgsl");
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("WGSL parse failed: {e:?}"));

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    );
    validator
        .validate(&module)
        .unwrap_or_else(|e| panic!("WGSL validation failed: {e:?}"));

    let sdf_prim = module
        .types
        .iter()
        .find_map(|(_, ty)| {
            if ty.name.as_deref() != Some("SdfPrim") {
                return None;
            }
            match &ty.inner {
                naga::TypeInner::Struct { span, .. } => Some(*span),
                _ => None,
            }
        })
        .expect("SdfPrim struct not found in designer_sdf.wgsl");

    assert_eq!(
        sdf_prim as usize,
        al_math::sdf::SDF_PRIM_SIZE,
        "SdfPrim WGSL layout does not match Sdf2dPrim",
    );
}
