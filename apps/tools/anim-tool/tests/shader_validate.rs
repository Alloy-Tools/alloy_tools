fn strip_editor_header(source: &str) -> String {
    const START_MARKER: &str = "// ----- Header Start -----";
    const END_MARKER: &str = "// ----- Header End -----";

    // Find the start of the editor header block.
    let start_pos = match source.find(START_MARKER) {
        Some(pos) => pos,
        None => {
            // No Header Start marker: fall back to stripping everything above Header End.
            match source.find(END_MARKER) {
                Some(pos) => {
                    let rest = &source[pos + END_MARKER.len()..];
                    return rest
                        .strip_prefix("\r\n")
                        .or_else(|| rest.strip_prefix('\n'))
                        .unwrap_or(rest)
                        .to_string();
                }
                None => return source.to_string(),
            }
        }
    };

    // Find the end of the editor header block.
    let end_search = &source[start_pos..];
    let end_pos = match end_search.find(END_MARKER) {
        Some(pos) => start_pos + pos + END_MARKER.len(),
        None => source.len(),
    };

    // Keep everything before Header Start, skip to after Header End.
    let before = &source[..start_pos];
    let after = &source[end_pos..];
    let after = after
        .strip_prefix("\r\n")
        .or_else(|| after.strip_prefix('\n'))
        .unwrap_or(after);

    // Join before and after with a newline between them.
    if before.is_empty() {
        after.to_string()
    } else if after.is_empty() {
        before.to_string()
    } else {
        format!("{before}\n{after}")
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
    let editor = include_str!("../src/designer_sdf.wgsl");

    // The file as opened in an editor: stub header + body. This must also
    // validate so VS Code does not show undefined-symbol errors.
    let _ = parse_and_validate(editor, "editor designer_sdf.wgsl");

    // The runtime source is composed by stripping the editor header block
    // (between Header Start and Header End) and prepending sdf_functions.wgsl.
    let functions = include_str!("../src/sdf_functions.wgsl");
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
        32,
        "Sdf2dOp should stay at the compressed 32-byte layout",
    );
    assert_eq!(
        sdf_op as usize,
        al_math::sdf::SDF_OP_SIZE,
        "SdfOp WGSL layout does not match Sdf2dOp",
    );

    let sdf_group = module
        .types
        .iter()
        .find_map(|(_, ty)| {
            if ty.name.as_deref() != Some("SdfGroup") {
                return None;
            }
            match &ty.inner {
                naga::TypeInner::Struct { span, .. } => Some(*span),
                _ => None,
            }
        })
        .expect("SdfGroup struct not found in designer_sdf.wgsl");

    assert_eq!(
        al_math::sdf::SDF_GROUP_SIZE,
        8,
        "Sdf2dGroup should stay at 8 bytes",
    );
    assert_eq!(
        sdf_group as usize,
        al_math::sdf::SDF_GROUP_SIZE,
        "SdfGroup WGSL layout does not match Sdf2dGroup",
    );
}
