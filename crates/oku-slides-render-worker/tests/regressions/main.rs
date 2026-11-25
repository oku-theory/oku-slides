//! Regression tests: one file per fixed bug.
//! Numbered `issue_NNNN` after the Oku task.

mod issue_1471_parent_gone;
// `cfg(test)`: its helpers fall under the test rules (`unwrap` allowed).
#[cfg(test)]
mod issue_1690_one_cell_edit_after_render;
