//! t-768 : un `set_param` sur un champ riche (`body`) donné en chaîne était
//! refusé (« body expects a list of paragraphs ») : ni la description d'`apply`,
//! ni `widgets` (`type: rich`), ni l'erreur ne montraient la forme attendue.
//! La description d'`apply` cite désormais un exemple de paragraphes ; cet
//! exemple, recopié tel quel par l'agent, doit être accepté.

use rust_mcp_sdk::McpClient;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{call, client, start};

/// Le premier tableau JSON de paragraphes cité dans `description`.
fn rich_example(description: &str) -> Option<Value> {
    let at = description.find("[{\"runs\"")?;
    serde_json::Deserializer::from_str(&description[at..]).into_iter::<Value>().next()?.ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_description_rich_example_is_accepted_by_set_param() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    let apply = tools.into_iter().find(|t| t.name == "apply").unwrap();
    let description = apply.description.unwrap_or_default();
    let example = rich_example(&description);
    assert_that!(example).named("exemple de paragraphes dans la description d'apply").is_some();

    let ops = json!([{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" },
        "param": "body", "value": example }]);
    let r = call(&c, "apply", json!({ "ops": ops })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let located = s.deck().slide("s-libre").unwrap().locate("e-texte").unwrap();
    assert_that!(located.element.params.rich("body").map(<[oku_slides_core::Paragraph]>::len)).is_greater_than(Some(0));
}
