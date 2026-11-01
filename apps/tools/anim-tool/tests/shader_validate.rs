fn strip_editor_header(source: &str) -> &str {
    const MARKER: &str = "// ----- Header End -----";
    match source.find(MARKER) {
        Some(pos) => {
            let rest = &source[pos + MARKER.len()..];
            rest.strip_prefix("\r\n")
                .or_else(|| rest.strip_prefix('\n'))
                .unwrap_or(rest)
        }
        None => source,
    }
}

fn parse_and_validate(source: &str, label: &str) -> naga::Module {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{label} WGSL parse failed: {e:?}"));

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    );
    validator
        .validate(&module)
        .unwrap_or_else(|e| panic!("{label} WGSL validation failed: {e:?}"));
    module
}

#[test]
fn designer_sdf_wgsl_is_valid() {
    let functions = include_str!("../src/sdf_functions.wgsl");
    let editor = include_str!("../src/designer_sdf.wgsl");

    // The file as opened in an editor: stub header + body. This must also
    // validate so VS Code does not show undefined-symbol errors.
    let _ = parse_and_validate(editor, "editor designer_sdf.wgsl");

    let body = strip_editor_header(editor);
    let source = format!("{functions}\n{body}");
    let module = parse_and_validate(&source, "runtime designer_sdf source");

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
        al_math::sdf::SDF_PRIM_SIZE,
        64,
        "Sdf2dPrim should stay at the compressed 64-byte layout",
    );
    assert_eq!(
        sdf_prim as usize,
        al_math::sdf::SDF_PRIM_SIZE,
        "SdfPrim WGSL layout does not match Sdf2dPrim",
    );

    let sdf_op = module
        .types
        .iter()
        .find_map(|(_, ty)| {
            if ty.name.as_deref() != Some("SdfOp") {
                return None;
            }
            match &ty.inner {
                naga::TypeInner::Struct { span, .. } => Some(*span),
                _ => None,
            }
        })
        .expect("SdfOp struct not found in designer_sdf.wgsl");

    assert_eq!(
        al_math::sdf::SDF_OP_SIZE,
        16,
        "Sdf2dOp should stay at the compressed 16-byte layout",
    );
    assert_eq!(
        sdf_op as usize,
        al_math::sdf::SDF_OP_SIZE,
        "SdfOp WGSL layout does not match Sdf2dOp",
    );
}
