//! t-1144 : `set_setting` ne touche plus la dictée.

use crate::common::*;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_set_setting_dictation_model_is_refused_in_french() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_setting", json!({ "key": "dictation.model", "value": "base" })).await;
    assert_that!(text_of(&r)).is_equal_to("Le réglage dictation.model ne peut pas être changé par l'agent.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_set_setting_dictation_model_writes_nothing() {
    let h = start().await;
    call_within(&server(&h).await, "set_setting", json!({ "key": "dictation.model", "value": "base" })).await;
    let keys: Vec<String> = lock(&h.settings.values).keys().cloned().collect();
    assert_that!(keys).does_not_contain("dictation.model".to_owned());
}
