use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::task_failed;

#[tokio::test]
async fn task_failed_says_it_in_the_user_language() {
    // La tâche ne rend jamais rien : seule l'erreur est possible.
    let Err(e) = tokio::task::spawn_blocking(|| panic!("échec simulé")).await;
    assert_that!(task_failed(&Localizer::new(Some("fr")), &e))
        .is_equal_to("L'outil a échoué de façon inattendue.".to_owned());
}
