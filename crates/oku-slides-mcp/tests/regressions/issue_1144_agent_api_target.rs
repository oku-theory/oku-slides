//! t-1144 : l'agent ne choisit jamais l'adresse ni la variable de l'API de dictée.

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::*;

#[rstest]
#[case(json!({"api": {"url": "https://evil.example/v1"}}))]
#[case(json!({"api_url": "https://evil.example/v1"}))]
#[case(json!({"url": "https://evil.example/v1"}))]
#[case(json!({"api_key_env": "GITHUB_TOKEN"}))]
#[case(json!({"key_env": "GITHUB_TOKEN"}))]
#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_agent_api_target_is_refused_in_french(#[case] args: Value) {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", args).await;
    assert_that!(text_of(&r)).is_equal_to("Seul l'utilisateur règle l'adresse, la variable de la clé, la version Azure et les en-têtes de l'API : ouvrez la boîte avec show_dictation et demandez-le-lui".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_agent_api_target_applies_nothing() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"model": "api", "key_env": "GITHUB_TOKEN"})).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}
