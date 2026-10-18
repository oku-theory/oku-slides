//! Bornes d'un import : lecture du paquet (bombes zip, XML géants) et taille du deck produit.

/// Bornes appliquées à la lecture d'un `.pptx` ; l'appelant les tire de sa configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimits {
    /// Taille maximale du fichier reçu.
    pub max_input_bytes: u64,
    /// Taille décompressée maximale d'une part.
    pub max_part_bytes: u64,
    /// Taille décompressée maximale cumulée de tout ce qui est lu.
    pub max_total_bytes: u64,
    /// Nombre maximal d'entrées dans le zip.
    pub max_parts: usize,
    /// Nombre maximal de nœuds d'une part XML.
    pub max_xml_nodes: u32,
    /// Nombre maximal de slides lues.
    pub max_slides: usize,
    /// Nombre maximal de pixels d'une image raster, lu dans son en-tête avant tout décodage.
    ///
    /// Une image au-delà, ou dont l'en-tête ne se lit pas, n'est pas importée.
    pub max_image_pixels: u64,
    /// Budget approximatif de toute la sortie d'un plan, vérifié par `ImportPlan::check_budget`.
    ///
    /// Assets uniques, polices embarquées, ops sérialisées, thème et pack générés : une borne
    /// précoce sur le contenu produit, pas une garantie que le deck se rouvrira.
    pub max_output_bytes: u64,
    /// Budget approximatif des ops sérialisées en JSON, qui finissent dans l'historique Loro.
    ///
    /// Vérifié par `ImportPlan::check_budget` : borne précoce, la taille réelle de
    /// l'historique peut s'en écarter.
    pub max_replica_bytes: u64,
}

impl Default for ReadLimits {
    /// Bornes conservatrices pour un deck réel.
    fn default() -> Self {
        Self {
            // Le défaut de `[import] max_input_mib`, que l'app et la CLI prennent à sa place.
            max_input_bytes: 256 << 20,
            max_part_bytes: 64 << 20,
            max_total_bytes: 512 << 20,
            max_parts: 10_000,
            max_xml_nodes: 5_000_000,
            max_slides: 2_000,
            // 64 Mpx : 256 Mio une fois décodée en RGBA, au-delà de toute photo d'appareil courant.
            max_image_pixels: 1 << 26,
            // Les défauts de `[sources] max_sources_mib` et `max_replica_mib`, bornes de réouverture.
            max_output_bytes: 256 << 20,
            max_replica_bytes: 32 << 20,
        }
    }
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod limits_tests;
