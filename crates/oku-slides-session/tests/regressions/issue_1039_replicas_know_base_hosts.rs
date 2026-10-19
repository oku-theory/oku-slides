//! t-1039 : un replica écrit avec le registre du noyau ne connaissait pas `base/table` : tableau en
//! valeur plate, fusion cellule par cellule perdue (ADR-0004).

use oku_slides_core::{HostType, kinds};
use oku_slides_session::Session;
use rstest::rstest;
use speculoos::prelude::*;

use crate::common::{Data, data_dir, fixture_path, session_in};
use crate::hifi_import::imported_hifi;

/// D'où vient le premier replica de la session.
#[derive(Debug, Clone, Copy)]
enum Open {
    New,
    Fixture,
    Help,
    Hifi,
}

impl Open {
    /// Une session ouverte ainsi, et son dossier de données.
    fn session(self) -> (Session, Data) {
        let data = data_dir();
        let s = match self {
            Self::New => session_in(&data),
            Self::Fixture => {
                let mut s = session_in(&data);
                s.open(&fixture_path()).unwrap_or_else(|e| panic!("{}", e.one_line()));
                s
            }
            Self::Help => {
                let mut s = session_in(&data);
                s.open_help("fr", None).unwrap_or_else(|e| panic!("{}", e.one_line()));
                s
            }
            Self::Hifi => imported_hifi(&data),
        };
        (s, data)
    }
}

#[rstest]
#[case::new_deck(Open::New)]
#[case::opened_fixture(Open::Fixture)]
#[case::help(Open::Help)]
#[case::hifi_import(Open::Hifi)]
fn issue_1039_every_first_replica_knows_the_base_table_host(#[case] open: Open) {
    let (s, _data) = open.session();
    let table = s
        .sources_for_tests()
        .registry()
        .get(kinds::base::TABLE)
        .and_then(|w| w.host_param(HostType::Table))
        .map(|p| p.name.clone());
    assert_that!(table).is_equal_to(Some("data".to_owned()));
}
