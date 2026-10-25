//! t-1213: the agent never sets the dictation API query or headers, which can carry secrets.

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::*;

#[rstest]
#[case(json!({"api_headers": "api-key: $GITHUB_TOKEN"}))]
#[case(json!({"headers": "api-key: $GITHUB_TOKEN"}))]
#[case(json!({"api_query": "api-version=1"}))]
#[case(json!({"query": "api-version=1"}))]
#[tokio::test(flavor = "multi_thread")]
async fn issue_1213_agent_api_headers_are_refused_in_french(#[case] args: Value) {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", args).await;
    assert_that!(text_of(&r)).is_equal_to("Seul l'utilisateur règle l'adresse, la variable de la clé, les paramètres et les en-têtes de l'API : ouvrez la boîte avec show_dictation et demandez-le-lui".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1213_agent_api_headers_apply_nothing() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"model": "api", "headers": "api-key: $GITHUB_TOKEN"})).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}
