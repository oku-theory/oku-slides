//! t-1171 : un asset ni dans le deck ni collé était dit par `session-asset-unknown`,
//! écrit pour l'agent (« donnez l'identifiant d'une image collée ») ; l'utilisateur qui
//! colle une image copiée dans un autre onglet lisait ce texte. La session parle
//! maintenant à l'utilisateur ; l'agent garde son message dans ses réponses d'outil.

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{call, client, start, text_of};

#[tokio::test(flavor = "multi_thread")]
async fn apply_citing_an_unknown_asset_points_the_agent_to_pasted_images() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let op = json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset", "value": "sha256:1234"
    });
    let r = call(&c, "apply", json!({ "ops": [op] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-asset-unknown"));
}
