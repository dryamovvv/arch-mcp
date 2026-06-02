#[macro_export]
macro_rules! stub_tool {
    ($name:ident, $tool_name:expr, $desc:expr) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl crate::tools::ToolHandler for $name {
            fn info(&self) -> crate::protocol::Tool {
                crate::protocol::Tool {
                    name: $tool_name.into(),
                    description: $desc.into(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                }
            }

            async fn call(
                &self,
                _args: std::collections::HashMap<String, serde_json::Value>,
            ) -> Result<serde_json::Value, crate::error::ToolError> {
                Err(crate::error::ToolError::internal("not yet implemented"))
            }
        }
    };
}
