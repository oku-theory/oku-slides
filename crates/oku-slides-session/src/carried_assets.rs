//! Assets apportés par un élément collé depuis un autre onglet (t-1178).
//!
//! Au collage, l'onglet d'origine donne les octets des assets que cite l'élément
//! ([`Session::element_assets`]) ; l'onglet d'arrivée les garde en mémoire
//! (`Session::carry_assets`) ; ils passent dans son bundle, et n'y sont plus gardés à
//! part, quand une op les cite (`Session::embed_op_assets`). Les autres restent jusqu'à
//! la copie suivante.

use std::collections::BTreeMap;

use oku_slides_core::Element;

use crate::Session;

/// Les assets apportés par le dernier collage entre onglets, par identifiant.
pub(crate) type CarriedAssets = BTreeMap<String, Vec<u8>>;

impl Session {
    /// Les octets des assets que cite `element` et que le bundle tient.
    ///
    /// Un asset cité mais absent du bundle est omis : l'onglet d'arrivée le refusera
    /// comme tout asset inconnu.
    pub fn element_assets(&self, element: &Element) -> Vec<Vec<u8>> {
        let refs = oku_slides_core::element_asset_refs(element, &self.registry);
        refs.iter().filter_map(|id| self.doc.sources.assets.get(id).cloned()).collect()
    }

    /// Garde les assets d'un collage venu d'un autre onglet, à la place des précédents.
    ///
    /// Les octets ne sont pas fiables : leur identifiant est recalculé (un bundle lu d'un
    /// zip ne vérifie pas le sien), et ils ne sont dessinés que par le rendu borné.
    /// Un asset que le bundle tient déjà n'est pas gardé : aucun collage ne le
    /// consommerait. Renvoie le nombre d'assets distincts gardés.
    pub(crate) fn carry_assets(&mut self, assets: Vec<Vec<u8>>) -> usize {
        let bundle = &self.doc.sources.assets;
        self.carried = assets
            .into_iter()
            .map(|bytes| (oku_slides_core::asset_id(&bytes), bytes))
            .filter(|(id, _)| !bundle.contains_key(id))
            .collect();
        self.carried.len()
    }
}

#[cfg(test)]
#[path = "carried_assets_tests.rs"]
mod carried_assets_tests;
