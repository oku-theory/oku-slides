use speculoos::prelude::*;

use super::*;

/// Cleaning once before escaping each line gives what cleaning each line again would.
#[test]
fn escape_text_equals_clean_then_escape() {
    for text in ["a\r\nb*\u{7}c\n# d", "\u{2066}x_y\u{2069}\r- z", "plain\n\n  1. item"] {
        let twice: Vec<String> = clean(text).split('\n').map(|line| line_start(&escape_inline(&clean(line)))).collect();
        assert_that!(escape_text(text)).named(text).is_equal_to(twice.join("\n"));
    }
}

#[test]
fn escape_text_specials_and_line_starts_escaped() {
    assert_that!(escape_text("*a* [b](c) <d> `e` a|b x_y"))
        .is_equal_to(r"\*a\* \[b\](c) \<d\> \`e\` a\|b x\_y".to_owned());
    assert_that!(escape_text("# h\n- i\n  + j\n12. k\n3) l\n==\n4.5 m"))
        .is_equal_to("\\# h\n\\- i\n\\+ j\n12\\. k\n3\\) l\n\\==\n4\\.5 m".to_owned());
}

#[test]
fn escape_text_rendered_as_plain_text() {
    let raw = "# not a heading\n- not a list\n**not bold** <b>not html</b> ![x](https://e.org/i.png)";
    let escaped = escape_text(raw);
    let markup: Vec<Event<'_>> = Parser::new(&escaped)
        .filter(|e| {
            !matches!(
                e,
                Event::Text(_) | Event::SoftBreak | Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph)
            )
        })
        .collect();
    assert_that!(markup).is_empty();
}

#[test]
fn escape_text_indented_lines_not_code() {
    let escaped = escape_text("para\n\n    code_like\n\n\ttabbed *x*");
    let blocks: Vec<Event<'_>> =
        Parser::new(&escaped).filter(|e| matches!(e, Event::Start(Tag::CodeBlock(_)) | Event::Code(_))).collect();
    assert_that!(blocks).is_empty();
    assert_that!(escaped).is_equal_to("para\n\ncode\\_like\n\ntabbed \\*x\\*".to_owned());
}

#[test]
fn escape_cell_newline_becomes_space_and_pipe_escaped() {
    assert_that!(escape_cell("a|b\nc\td")).is_equal_to(r"a\|b c d".to_owned());
    assert_that!(escape_cell("# 1. x")).is_equal_to("# 1. x".to_owned());
}

#[test]
fn link_url_keeps_web_and_mail_schemes_only() {
    assert_that!(link_url("HTTPS://e.org/a b")).is_equal_to(Some("HTTPS://e.org/a%20b".to_owned()));
    assert_that!(link_url("mailto:a@b.c")).is_equal_to(Some("mailto:a@b.c".to_owned()));
    assert_that!(link_url("javascript:alert(1)")).is_none();
    assert_that!(link_url("file:///etc/passwd")).is_none();
    assert_that!(link_url("./local.md")).is_none();
}

#[test]
fn table_first_row_header_and_rows_padded() {
    let rows = vec![vec!["a".to_owned(), "b".to_owned()], vec!["1".to_owned()]];
    assert_that!(table(&rows)).is_equal_to("| a | b |\n| --- | --- |\n| 1 |  |\n".to_owned());
}

#[test]
fn cut_line_sheets_state_the_spent_budget() {
    assert_that!(cut_line(Cut { what: CutKind::Sheets, count: 3 }, Bound::Budget))
        .is_equal_to("… 3 sheets not imported (text budget spent)\n".to_owned());
    assert_that!(cut_line(Cut { what: CutKind::Rows, count: 3 }, Bound::Limit(10)))
        .is_equal_to("… 3 rows not imported (limit 10)\n".to_owned());
}

#[test]
fn fence_for_longer_than_backtick_runs() {
    assert_that!(fence_for("plain")).is_equal_to("```".to_owned());
    assert_that!(fence_for("a ```` b")).is_equal_to("`````".to_owned());
}

#[test]
fn fit_within_max_kept_whole() {
    assert_that!(fit("<", "body", ">", 6, 0)).is_equal_to(Some(("<body>".to_owned(), None)));
}

#[test]
fn fit_past_max_cut_at_line_end_and_stated() {
    let body = "line one\nline two\nline three\n".repeat(10);
    let (out, cut) = fit("", &body, "", 120, 0).unwrap();
    assert_that!(out.len()).is_less_than_or_equal_to(120);
    let cut = cut.unwrap();
    assert_that!(cut.what).is_equal_to(CutKind::Text);
    let kept = out.split("\n\n…").next().unwrap();
    assert_that!(body.starts_with(kept)).is_true();
    assert_that!(cut.count).is_equal_to(count_u64(body.len() - kept.len()));
    assert_that!(out.as_str()).ends_with(format!("… {} bytes not imported (limit 120)\n", cut.count));
}

#[test]
fn fit_cut_inside_fence_closed_before_notice() {
    let body = format!("intro\n\n~~~~ rust\n{}~~~~\n\nafter\n", "let a = 1;\n".repeat(30));
    let (out, cut) = fit("", &body, "", 200, 0).unwrap();
    assert_that!(out.len()).is_less_than_or_equal_to(200);
    assert_that!(cut).is_some();
    let notice = out.rfind('…').unwrap();
    let inside: Vec<_> = Parser::new(&out)
        .into_offset_iter()
        .filter(|(e, r)| matches!(e, Event::Start(Tag::CodeBlock(_))) && r.contains(&notice))
        .collect();
    assert_that!(inside).is_empty();
    assert_that!(out.as_str()).contains("let a = 1;\n~~~~\n\n… ");
}

#[test]
fn fit_no_room_for_the_notice_is_none() {
    assert_that!(fit("", &"x".repeat(100), "", 10, 0)).is_none();
}

#[test]
fn sanitize_markdown_reference_definition_removed() {
    let out = sanitize_markdown("[a][r]\n\n[r]: javascript:alert(1)\n");
    assert_that!(out.as_str()).does_not_contain("javascript");
    assert_that!(out.trim()).is_equal_to("a");
}

#[test]
fn sanitize_markdown_nested_html_in_link_removed_over_passes() {
    let out = sanitize_markdown("[a <b>x</b>](https://e.org)\n");
    assert_that!(out.as_str()).is_equal_to("[a x](https://e.org)\n");
}

#[test]
fn sanitize_markdown_email_autolink_kept_as_mailto() {
    assert_that!(sanitize_markdown("<a@b.org>\n")).is_equal_to("[a@b.org](mailto:a@b.org)\n".to_owned());
}

#[test]
fn sanitize_markdown_html_in_code_block_kept() {
    let text = "```\n<script>x</script>\n```\n";
    assert_that!(sanitize_markdown(text)).is_equal_to(text.to_owned());
}
