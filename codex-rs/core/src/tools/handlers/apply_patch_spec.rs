use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

const APPLY_PATCH_LARK_GRAMMAR: &str = include_str!("../../../assets/tools/apply_patch.lark");

/// Returns a custom tool that can be used to edit files. Well-suited for GPT-5 models
/// https://platform.openai.com/docs/guides/function-calling#custom-tools
pub fn create_apply_patch_freeform_tool(include_environment_id: bool) -> ToolSpec {
    let definition = if include_environment_id {
        APPLY_PATCH_LARK_GRAMMAR.replace(
            "start: begin_patch hunk+ end_patch",
            "start: begin_patch environment_id? hunk+ end_patch\nenvironment_id: \"*** Environment ID: \" filename LF",
        )
    } else {
        APPLY_PATCH_LARK_GRAMMAR.to_string()
    };
    ToolSpec::Freeform(FreeformTool {
        name: "apply_patch".to_string(),
        description: "The `apply_patch` tool can be used to edit files. This is a FREEFORM tool, so do not wrap the patch in JSON.".to_string(),
        defer_loading: None,
        format: FreeformToolFormat {
            r#type: "grammar".to_string(),
            syntax: "lark".to_string(),
            definition,
        },
    })
}

/// Returns the same patch tool as a JSON function, for providers that drop custom tools.
pub fn create_apply_patch_function_tool(include_environment_id: bool) -> ToolSpec {
    let environment_line = if include_environment_id {
        "*** Environment ID: <id>\n"
    } else {
        ""
    };
    let properties = BTreeMap::from([(
        "input".to_string(),
        JsonSchema::string(Some(format!(
            "The entire patch. This is NOT a unified diff: never write line numbers such as \
             `@@ -1 +1,2 @@`.\n\
             *** Begin Patch\n\
             {environment_line}\
             *** Update File: src/app.py\n\
             @@ def greet():\n\
             -    return \"hi\"\n\
             +    return \"hello\"\n\
             *** End Patch\n\
             `@@ <line>` names an existing line ABOVE the change (such as the enclosing function); \
             never repeat a line you are changing there. Use a bare `@@` near the top of a file. \
             Context lines start with a space, removed lines with `-`, added lines with `+`. \
             Create a file with `*** Add File: <path>` and every line prefixed `+`; delete one with \
             `*** Delete File: <path>`; rename with `*** Move to: <path>` after `*** Update File`."
        ))),
    )]);
    ToolSpec::Function(ResponsesApiTool {
        name: "apply_patch".to_string(),
        description:
            "Edit files by applying a patch. Prefer this over shell commands for file edits."
                .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["input".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "apply_patch_spec_tests.rs"]
mod tests;
