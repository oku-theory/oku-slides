//! Modèles de slides proposés par « Nouvelle slide ». Chaque modèle est une
//! slide complète ; l'interface la copie avec de nouveaux identifiants.

use serde::Serialize;
use serde_json::json;

use slider_core::Slide;

#[derive(Debug, Clone, Serialize)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub slide: Slide,
}

fn slide(v: serde_json::Value) -> Slide {
    serde_json::from_value(v).expect("modèle de slide valide")
}

fn para(t: &str) -> serde_json::Value {
    json!({ "runs": [{ "t": t }] })
}

fn bullet(t: &str) -> serde_json::Value {
    json!({ "list": "bullet", "runs": [{ "t": t }] })
}

/// Les modèles, dans l'ordre du carrousel.
pub fn templates() -> Vec<Template> {
    vec![
        Template {
            id: "texte",
            name: "Texte",
            description: "Un titre et une liste de points",
            slide: slide(json!({
                "id": "tpl-texte", "kind": "content", "kicker": "Surtitre", "title": "Titre de la slide",
                "blocks": [{ "id": "b", "kind": "text", "text": [
                    bullet("Premier point"), bullet("Deuxième point"), bullet("Troisième point")
                ] }]
            })),
        },
        Template {
            id: "cartes",
            name: "Trois cartes",
            description: "Trois idées côte à côte, la dernière mise en avant",
            slide: slide(json!({
                "id": "tpl-cartes", "kind": "content", "kicker": "Surtitre", "title": "Trois idées", "layout": "columns",
                "blocks": [
                    { "id": "c1", "kind": "card", "kicker": "Un", "title": "Première", "text": [bullet("Détail")] },
                    { "id": "c2", "kind": "card", "kicker": "Deux", "title": "Deuxième", "text": [bullet("Détail")] },
                    { "id": "c3", "kind": "card", "kicker": "Trois", "title": "Troisième", "highlight": true, "text": [bullet("Détail")] }
                ]
            })),
        },
        Template {
            id: "colonnes",
            name: "Deux colonnes",
            description: "Comparer deux choses, ou un texte et son illustration",
            slide: slide(json!({
                "id": "tpl-colonnes", "kind": "content", "title": "Avant, après", "layout": "split",
                "blocks": [
                    { "id": "g", "kind": "text", "text": [para("Avant"), bullet("Ce qui était"), bullet("Ce qui gênait")] },
                    { "id": "d", "kind": "text", "text": [para("Après"), bullet("Ce qui change"), bullet("Ce qu'on y gagne")] }
                ]
            })),
        },
        Template {
            id: "citation",
            name: "Citation",
            description: "Une phrase forte, mise en valeur",
            slide: slide(json!({
                "id": "tpl-citation", "kind": "content", "title": "Ce qu'il faut retenir", "layout": "quote",
                "blocks": [{ "id": "q", "kind": "text", "text": [para("« Une idée par slide. »")] }]
            })),
        },
        Template {
            id: "message",
            name: "Message central",
            description: "Un seul message, au centre",
            slide: slide(json!({
                "id": "tpl-message", "kind": "content", "layout": "center",
                "blocks": [{ "id": "m", "kind": "text", "style": { "size": "subtitle", "font": "display", "align": "center" },
                    "text": [para("Le message principal")] }]
            })),
        },
        Template {
            id: "libre",
            name: "Libre",
            description: "Des éléments à placer où vous voulez",
            slide: slide(json!({
                "id": "tpl-libre", "kind": "content", "title": "Composition libre",
                "elements": [
                    { "id": "f", "kind": "shape", "frame": { "x": 0, "y": 0, "w": 800, "h": 460 }, "style": { "fill": "surface", "radius": "md" } },
                    { "id": "t", "kind": "text", "frame": { "x": 40, "y": 40, "w": 720, "h": 160 },
                      "style": { "size": "lead", "font": "display" }, "text": [para("Un texte placé librement")] }
                ]
            })),
        },
        Template {
            id: "chapitre",
            name: "Chapitre",
            description: "Ouvre une partie, apparaît dans le sommaire",
            slide: slide(
                json!({ "id": "tpl-chapitre", "kind": "chapter", "title": "Nouvelle partie", "subtitle": "En une phrase" }),
            ),
        },
        Template {
            id: "sommaire",
            name: "Sommaire",
            description: "La liste des chapitres, mise à jour seule",
            slide: slide(json!({ "id": "tpl-sommaire", "kind": "toc" })),
        },
        Template {
            id: "couverture",
            name: "Couverture",
            description: "Titre, sous-titre, auteur et date du deck",
            slide: slide(json!({ "id": "tpl-couverture", "kind": "title" })),
        },
        Template {
            id: "fin",
            name: "Fin",
            description: "La dernière slide",
            slide: slide(json!({ "id": "tpl-fin", "kind": "end", "text": "Merci" })),
        },
    ]
}
