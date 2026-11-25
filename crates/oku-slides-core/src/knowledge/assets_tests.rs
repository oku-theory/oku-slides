use std::collections::BTreeSet;

use proptest::prelude::*;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::model::Deck;

const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn file(path: &str, text: &str) -> KnowledgeFile {
    KnowledgeFile { path: path.to_owned(), text: text.to_owned() }
}

fn id() -> BTreeSet<String> {
    BTreeSet::from([format!("sha256:{HEX}")])
}

#[rstest]
#[case::absolute("/a.md", format!("![x](/references/assets/{HEX}.png)"))]
#[case::relative("/references/a.md", format!("![x](assets/{HEX}.png)"))]
#[case::dot_relative("/references/a.md", format!("![x](./assets/{HEX}.png)"))]
#[case::parent("/notes/a.md", format!("![x](../references/assets/{HEX}.png)"))]
#[case::deep("/a/b/c.md", format!("![x](../../references/assets/{HEX}.webp)"))]
#[case::inside_a_link("/a.md", format!("[![x](/references/assets/{HEX}.png)](https://v)"))]
#[case::reference_style("/a.md", format!("![x][r]\n\n[r]: /references/assets/{HEX}.png"))]
fn cited_assets_keeps_an_image_of_the_asset_folder(#[case] path: &str, #[case] body: String) {
    assert_that!(cited_assets(&file(path, &body))).is_equal_to(id());
}

#[rstest]
#[case::past_root("/a.md", format!("![x](/../../references/assets/{HEX}.png)"))]
#[case::relative_past_root("/a.md", format!("![x](../references/assets/{HEX}.png)"))]
#[case::scheme("/a.md", format!("![x](https://x/references/assets/{HEX}.png)"))]
#[case::scheme_only("/a.md", format!("![x](data:/references/assets/{HEX}.png)"))]
#[case::host("/a.md", format!("![x](//h/references/assets/{HEX}.png)"))]
#[case::query("/a.md", format!("![x](/references/assets/{HEX}.png?q)"))]
#[case::fragment("/a.md", format!("![x](/references/assets/{HEX}.png#f)"))]
#[case::percent("/a.md", "![x](/%2e%2e/references/assets/0.png)".to_owned())]
#[case::percent_in_name("/a.md", format!("![x](/references/assets/{HEX}%2epng)"))]
#[case::uppercase("/a.md", format!("![x](/references/assets/{}.png)", HEX.to_uppercase()))]
#[case::short("/a.md", format!("![x](/references/assets/{}.png)", &HEX[1..]))]
#[case::no_extension("/a.md", format!("![x](/references/assets/{HEX})"))]
#[case::other_folder("/a.md", format!("![x](/other/assets/{HEX}.png)"))]
#[case::nested("/a.md", format!("![x](/references/assets/sub/{HEX}.png)"))]
#[case::empty_segment("/a.md", format!("![x](/references//assets/{HEX}.png)"))]
#[case::relative_empty_segment("/references/a.md", format!("![x](assets//{HEX}.png)"))]
#[case::trailing_slash("/a.md", format!("![x](/references/assets/{HEX}.png/)"))]
#[case::entity_percent("/a.md", format!("![x](/references/assets/{HEX}&#37;2epng)"))]
#[case::entity_dots("/a.md", format!("![x](/&#46;&#46;/references/assets/{HEX}.png)"))]
#[case::backslash("/a.md", format!("![x](\\references\\assets\\{HEX}.png)"))]
#[case::link("/a.md", format!("[x](/references/assets/{HEX}.png)"))]
#[case::html_img("/a.md", format!("<img src=\"/references/assets/{HEX}.png\">"))]
#[case::fenced("/a.md", format!("```\n![x](/references/assets/{HEX}.png)\n```"))]
#[case::frontmatter(
    "/a.md",
    format!("---\ntype: Note\nicon: \"![x](/references/assets/{HEX}.png)\"\n---\nno image\n")
)]
fn cited_assets_refuses(#[case] path: &str, #[case] text: String) {
    assert_that!(cited_assets(&file(path, &text))).is_equal_to(BTreeSet::new());
}

#[test]
fn cited_assets_scans_the_body_after_a_valid_frontmatter() {
    let text = format!("---\ntype: Note\n---\n![x](/references/assets/{HEX}.png)\n");
    assert_that!(cited_assets(&file("/a.md", &text))).is_equal_to(id());
}

#[test]
fn cited_assets_scans_the_whole_text_when_the_frontmatter_is_unterminated() {
    let text = format!("---\ntype: [\n![x](/references/assets/{HEX}.png)\n");
    assert_that!(cited_assets(&file("/a.md", &text))).is_equal_to(id());
}

#[test]
fn knowledge_asset_refs_is_the_union_over_the_files() {
    let other = "f".repeat(64);
    let mut deck = Deck::empty();
    for (n, text) in [format!("![x](/references/assets/{HEX}.png)"), format!("![y](/references/assets/{other}.jpg)")]
        .into_iter()
        .enumerate()
    {
        deck.knowledge.insert(format!("{n:032x}"), file(&format!("/n{n}.md"), &text));
    }
    let mut want = id();
    want.insert(format!("sha256:{other}"));
    assert_that!(crate::knowledge_asset_refs(&deck)).is_equal_to(want);
}

proptest! {
    #[test]
    fn cited_assets_never_panics(path in "[ -~]{0,40}", text in "\\PC{0,200}") {
        let _ = cited_assets(&file(&path, &text));
    }

    #[test]
    fn cited_assets_never_panics_on_image_urls(url in "[ -~]{0,80}") {
        let _ = cited_assets(&file("/a/b.md", &format!("![x]({url})")));
    }
}
