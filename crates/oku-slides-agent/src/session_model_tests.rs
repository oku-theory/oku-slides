use agent_client_protocol::schema::v1::{
    SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectGroup, SessionConfigSelectOption, SessionId,
};
use oku_slides_session::{AgentModelView, AgentModelsView};
use rstest::rstest;
use speculoos::prelude::*;

use super::{ModelBoard, ModelOptions, ModelValue, Offered, model_options};

fn value(id: &str) -> SessionConfigSelectOption {
    SessionConfigSelectOption::new(id.to_owned(), id.to_uppercase())
}

fn models(current: &str, ids: &[&str]) -> SessionConfigOption {
    let values = ids.iter().map(|i| value(i)).collect::<Vec<_>>();
    SessionConfigOption::select("model", "Model", current.to_owned(), values)
        .category(SessionConfigOptionCategory::Model)
}

fn options(current: &str, ids: &[&str]) -> ModelOptions {
    ModelOptions {
        config_id: "model".into(),
        current: current.to_owned(),
        values: ids
            .iter()
            .map(|i| ModelValue { id: (*i).to_owned(), name: i.to_uppercase(), description: None })
            .collect(),
    }
}

fn ids(o: &ModelOptions) -> Vec<&str> {
    o.values.iter().map(|v| v.id.as_str()).collect()
}

#[test]
fn model_options_ungrouped_model_select_returns_its_values() {
    let found = model_options(&[models("b", &["a", "b"])]);
    assert_that!(found).is_equal_to(Some(options("b", &["a", "b"])));
}

#[test]
fn model_options_grouped_select_flattens_groups_in_order() {
    let groups = vec![
        SessionConfigSelectGroup::new("fast", "Fast", vec![value("a")]),
        SessionConfigSelectGroup::new("deep", "Deep", vec![value("b"), value("c")]),
    ];
    let option = SessionConfigOption::select("m", "Model", "c", groups).category(SessionConfigOptionCategory::Model);
    let found = model_options(&[option]).unwrap_or_else(|| panic!("model option not found"));
    assert_that!(ids(&found)).is_equal_to(vec!["a", "b", "c"]);
}

#[test]
fn model_options_keeps_the_value_description() {
    let option = SessionConfigOption::select(
        "model",
        "Model",
        "a",
        vec![SessionConfigSelectOption::new("a", "A").description("quick".to_owned())],
    );
    let found = model_options(&[option]).unwrap_or_else(|| panic!("model option not found"));
    assert_that!(found.values[0].description.as_deref()).is_equal_to(Some("quick"));
}

#[test]
fn model_options_category_less_select_named_model_counts() {
    let option = SessionConfigOption::select("model", "Model", "a", vec![value("a")]);
    assert_that!(model_options(&[option]).map(|o| o.current)).is_equal_to(Some("a".to_owned()));
}

#[rstest]
#[case::other_category_named_model(
    SessionConfigOption::select("model", "Model", "a", vec![value("a")]).category(SessionConfigOptionCategory::Mode)
)]
#[case::category_less_other_id(SessionConfigOption::select("effort", "Effort", "a", vec![value("a")]))]
#[case::boolean_of_category_model(
    SessionConfigOption::boolean("model", "Model", true).category(SessionConfigOptionCategory::Model)
)]
fn model_options_without_a_model_select_returns_none(#[case] option: SessionConfigOption) {
    assert_that!(model_options(&[option])).is_none();
}

#[test]
fn model_options_takes_the_first_model_select() {
    let found = model_options(&[
        SessionConfigOption::boolean("model", "Model", true).category(SessionConfigOptionCategory::Model),
        models("x", &["x"]),
        models("y", &["y"]),
    ]);
    assert_that!(found.map(|o| o.current)).is_equal_to(Some("x".to_owned()));
}

#[test]
fn offered_view_unknown_shows_the_chosen_model() {
    let view = Offered::Unknown.view(Some("a".into()));
    assert_that!(view).is_equal_to(Some(AgentModelsView::Unknown { chosen: Some("a".into()) }));
}

#[test]
fn offered_view_no_choice_returns_none() {
    assert_that!(Offered::NoChoice.view(Some("a".into()))).is_none();
}

#[test]
fn offered_view_models_lists_them_with_current_and_chosen() {
    let view = Offered::Models(options("b", &["a", "b"])).view(None);
    let expected = AgentModelsView::Offered {
        models: vec![
            AgentModelView { id: "a".into(), name: "A".into(), hint: None },
            AgentModelView { id: "b".into(), name: "B".into(), hint: None },
        ],
        current: "b".into(),
        chosen: None,
    };
    assert_that!(view).is_equal_to(Some(expected));
}

#[test]
fn board_offered_before_any_session_is_unknown() {
    assert_that!(ModelBoard::default().offered(0)).is_equal_to(Offered::Unknown);
}

#[test]
fn board_offered_returns_what_the_last_session_of_the_epoch_told() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    board.told(1, 0, &SessionId::new("t"), Some(options("b", &["a", "b"])));
    assert_that!(board.offered(0)).is_equal_to(Offered::Models(options("b", &["a", "b"])));
}

#[test]
fn board_offered_of_another_epoch_is_unknown() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    assert_that!(board.offered(1)).is_equal_to(Offered::Unknown);
}

#[test]
fn board_told_late_by_an_older_epoch_keeps_the_newer_offer() {
    let board = ModelBoard::default();
    board.told(2, 1, &SessionId::new("s"), Some(options("b", &["b"])));
    board.told(1, 0, &SessionId::new("t"), Some(options("a", &["a"])));
    assert_that!(board.offered(1)).is_equal_to(Offered::Models(options("b", &["b"])));
}

#[test]
fn board_told_late_by_an_older_epoch_still_records_its_session() {
    let board = ModelBoard::default();
    board.told(2, 1, &SessionId::new("s"), Some(options("b", &["b"])));
    board.told(1, 0, &SessionId::new("t"), Some(options("a", &["a"])));
    assert_that!(board.session(1, &SessionId::new("t"))).is_equal_to(Some(options("a", &["a"])));
}

#[test]
fn board_told_without_options_offers_no_choice() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), None);
    assert_that!(board.offered(0)).is_equal_to(Offered::NoChoice);
}

#[test]
fn board_told_without_options_forgets_the_session_model() {
    let board = ModelBoard::default();
    let id = SessionId::new("s");
    board.told(1, 0, &id, Some(options("a", &["a"])));
    board.told(1, 0, &id, None);
    assert_that!(board.session(1, &id)).is_none();
}

#[test]
fn board_session_is_kept_per_generation() {
    let board = ModelBoard::default();
    let id = SessionId::new("s");
    board.told(1, 0, &id, Some(options("a", &["a"])));
    assert_that!(board.session(2, &id)).is_none();
}

#[test]
fn board_forget_drops_only_that_session() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    board.told(1, 0, &SessionId::new("t"), Some(options("a", &["a"])));
    board.forget(1, &SessionId::new("s"));
    let kept = (board.session(1, &SessionId::new("s")), board.session(1, &SessionId::new("t")).is_some());
    assert_that!(kept).is_equal_to((None, true));
}

#[test]
fn board_forget_generation_drops_its_sessions_only() {
    let board = ModelBoard::default();
    let id = SessionId::new("s");
    board.told(1, 0, &id, Some(options("a", &["a"])));
    board.told(2, 0, &id, Some(options("a", &["a"])));
    board.forget_generation(1);
    assert_that!((board.session(1, &id), board.session(2, &id).is_some())).is_equal_to((None, true));
}

#[test]
fn board_revision_advances_when_the_offer_changes() {
    let board = ModelBoard::default();
    let revision = board.subscribe();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    assert_that!(revision.has_changed().unwrap_or(false)).is_true();
}

#[test]
fn board_revision_stays_when_the_same_offer_is_told_again() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    let mut revision = board.subscribe();
    revision.mark_unchanged();
    board.told(1, 0, &SessionId::new("t"), Some(options("a", &["a"])));
    assert_that!(revision.has_changed().unwrap_or(true)).is_false();
}

#[test]
fn board_revision_advances_on_a_new_epoch_with_the_same_offer() {
    let board = ModelBoard::default();
    board.told(1, 0, &SessionId::new("s"), Some(options("a", &["a"])));
    let mut revision = board.subscribe();
    revision.mark_unchanged();
    board.told(2, 1, &SessionId::new("s"), Some(options("a", &["a"])));
    assert_that!(revision.has_changed().unwrap_or(false)).is_true();
}
