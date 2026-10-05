use super::*;
use pretty_assertions::assert_eq;

#[test]
fn create_apply_patch_freeform_tool_matches_expected_spec() {
    assert_eq!(
        create_apply_patch_freeform_tool(/*include_environment_id*/ false),
        ToolSpec::Freeform(FreeformTool {
            name: "apply_patch".to_string(),
            description:
                "The `apply_patch` tool can be used to edit files. This is a FREEFORM tool, so do not wrap the patch in JSON."
                    .to_string(),
            defer_loading: None,
            format: FreeformToolFormat {
                r#type: "grammar".to_string(),
                syntax: "lark".to_string(),
                definition: APPLY_PATCH_LARK_GRAMMAR.to_string(),
            },
        })
    );
}

#[test]
fn create_apply_patch_freeform_tool_includes_environment_id_when_requested() {
    let ToolSpec::Freeform(tool) =
        create_apply_patch_freeform_tool(/*include_environment_id*/ true)
    else {
        panic!("expected freeform tool");
    };

    assert!(tool.format.definition.contains("environment_id?"));
    assert!(
        tool.format
            .definition
            .contains("\"*** Environment ID: \" filename LF")
    );
}

#[test]
fn create_apply_patch_function_tool_requires_input() {
    let ToolSpec::Function(tool) =
        create_apply_patch_function_tool(/*include_environment_id*/ false)
    else {
        panic!("expected a function tool");
    };
    assert_eq!(tool.name, "apply_patch");
    let schema = serde_json::to_value(&tool.parameters).expect("schema serializes");
    assert_eq!(schema["required"], serde_json::json!(["input"]));
    assert_eq!(schema["properties"]["input"]["type"], "string");
}
