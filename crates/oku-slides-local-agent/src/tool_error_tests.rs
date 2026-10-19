//! Tests de la classification des erreurs d'outils.

use rust_mcp_sdk::error::McpSdkError;
use rust_mcp_sdk::schema::RpcError;
use speculoos::prelude::*;

use super::ToolError;

#[test]
fn from_sdk_rpc_error_is_rejected_with_its_message() {
    let error = ToolError::from(McpSdkError::RpcError(RpcError::invalid_params().with_message("bad args")));
    assert_that!(error.is_rejected()).is_true();
    assert_that!(error.to_string()).is_equal_to("bad args".to_owned());
}

#[test]
fn from_sdk_other_error_is_transport() {
    let error = ToolError::from(McpSdkError::Internal { description: "down".into() });
    assert_that!(error.is_rejected()).is_false();
}
