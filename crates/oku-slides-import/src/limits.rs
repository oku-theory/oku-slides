//! Bornes de lecture d'un paquet (bombes zip, XML géants).

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
}

impl Default for ReadLimits {
    /// Bornes conservatrices pour un deck réel.
    fn default() -> Self {
        Self {
            max_input_bytes: 256 << 20,
            max_part_bytes: 64 << 20,
            max_total_bytes: 512 << 20,
            max_parts: 10_000,
            max_xml_nodes: 5_000_000,
            max_slides: 2_000,
        }
    }
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod limits_tests;
