//! Styles de tableau intégrés de PowerPoint : ceux qu'un tableau désigne par
//! GUID sans que `tableStyles.xml` les définisse.
//!
//! PowerPoint n'écrit dans `tableStyles.xml` que les styles employés qu'il
//! veut bien y mettre ; les autres sont intégrés à l'application. Chaque
//! famille (thématique, clair, moyen, foncé 1 à 4) se décline sans accent
//! (sur `tx1` ou `dk1`) et pour les accents 1 à 6. [`builtin`] rend la
//! définition DrawingML (`a:tblStyle`) du style de GUID donné, que le lecteur
//! de tableaux lit comme un style du fichier.
//!
//! Les GUID et les remplissages suivent ceux que LibreOffice reproduit
//! (`oox/source/drawingml/table/predefined-table-styles.cxx`, MPL-2.0) : les
//! valeurs sont des faits du format, réécrits ici sans en copier le code.
//! Écarts voulus, comme PowerPoint : le style foncé 1 a un texte clair, et
//! sans accent des gris foncés (`tint` du noir) ; la ligne de total du moyen 4
//! prend la teinte de l'accent ;
//! les références de remplissage de thème (`fillRef`) des styles thématiques
//! deviennent des pleins de l'accent, qui ne dépendent pas du thème.

// `write!` into a `String` never fails: its result is ignored.
use std::fmt::Write as _;

/// Famille d'un style intégré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Themed1,
    Themed2,
    Light1,
    Light2,
    Light3,
    Medium1,
    Medium2,
    Medium3,
    Medium4,
    Dark1,
    Dark2,
}

use Family::{Dark1, Dark2, Light1, Light2, Light3, Medium1, Medium2, Medium3, Medium4, Themed1, Themed2};

/// Les styles intégrés : GUID, famille, accent (0 : sans accent ; le foncé 2
/// associe l'accent impair donné au pair suivant).
const BUILTIN: [(&str, Family, u8); 74] = [
    ("{2D5ABB26-0587-4C30-8999-92F81FD0307C}", Themed1, 0),
    ("{3C2FFA5D-87B4-456A-9821-1D502468CF0F}", Themed1, 1),
    ("{284E427A-3D55-4303-BF80-6455036E1DE7}", Themed1, 2),
    ("{69C7853C-536D-4A76-A0AE-DD22124D55A5}", Themed1, 3),
    ("{775DCB02-9BB8-47FD-8907-85C794F793BA}", Themed1, 4),
    ("{35758FB7-9AC5-4552-8A53-C91805E547FA}", Themed1, 5),
    ("{08FB837D-C827-4EFA-A057-4D05807E0F7C}", Themed1, 6),
    ("{5940675A-B579-460E-94D1-54222C63F5DA}", Themed2, 0),
    ("{D113A9D2-9D6B-4929-AA2D-F23B5EE8CBE7}", Themed2, 1),
    ("{18603FDC-E32A-4AB5-989C-0864C3EAD2B8}", Themed2, 2),
    ("{306799F8-075E-4A3A-A7F6-7FBC6576F1A4}", Themed2, 3),
    ("{E269D01E-BC32-4049-B463-5C60D7B0CCD2}", Themed2, 4),
    ("{327F97BB-C833-4FB7-BDE5-3F7075034690}", Themed2, 5),
    ("{638B1855-1B75-4FBE-930C-398BA8C253C6}", Themed2, 6),
    ("{9D7B26C5-4107-4FEC-AEDC-1716B250A1EF}", Light1, 0),
    ("{3B4B98B0-60AC-42C2-AFA5-B58CD77FA1E5}", Light1, 1),
    ("{0E3FDE45-AF77-4B5C-9715-49D594BDF05E}", Light1, 2),
    ("{C083E6E3-FA7D-4D7B-A595-EF9225AFEA82}", Light1, 3),
    ("{D27102A9-8310-4765-A935-A1911B00CA55}", Light1, 4),
    ("{5FD0F851-EC5A-4D38-B0AD-8093EC10F338}", Light1, 5),
    ("{68D230F3-CF80-4859-8CE7-A43EE81993B5}", Light1, 6),
    ("{7E9639D4-E3E2-4D34-9284-5A2195B3D0D7}", Light2, 0),
    ("{69012ECD-51FC-41F1-AA8D-1B2483CD663E}", Light2, 1),
    ("{72833802-FEF1-4C79-8D5D-14CF1EAF98D9}", Light2, 2),
    ("{F2DE63D5-997A-4646-A377-4702673A728D}", Light2, 3),
    ("{17292A2E-F333-43FB-9621-5CBBE7FDCDCB}", Light2, 4),
    ("{5A111915-BE36-4E01-A7E5-04B1672EAD32}", Light2, 5),
    ("{912C8C85-51F0-491E-9774-3900AFEF0FD7}", Light2, 6),
    ("{616DA210-FB5B-4158-B5E0-FEB733F419BA}", Light3, 0),
    ("{BC89EF96-8CEA-46FF-86C4-4CE0E7609802}", Light3, 1),
    ("{5DA37D80-6434-44D0-A028-1B22A696006F}", Light3, 2),
    ("{8799B23B-EC83-4686-B30A-512413B5E67A}", Light3, 3),
    ("{ED083AE6-46FA-4A59-8FB0-9F97EB10719F}", Light3, 4),
    ("{BDBED569-4797-4DF1-A0F4-6AAB3CD982D8}", Light3, 5),
    ("{E8B1032C-EA38-4F05-BA0D-38AFFFC7BED3}", Light3, 6),
    ("{793D81CF-94F2-401A-BA57-92F5A7B2D0C5}", Medium1, 0),
    ("{B301B821-A1FF-4177-AEE7-76D212191A09}", Medium1, 1),
    ("{9DCAF9ED-07DC-4A11-8D7F-57B35C25682E}", Medium1, 2),
    ("{1FECB4D8-DB02-4DC6-A0A2-4F2EBAE1DC90}", Medium1, 3),
    ("{1E171933-4619-4E11-9A3F-F7608DF75F80}", Medium1, 4),
    ("{FABFCF23-3B69-468F-B69F-88F6DE6A72F2}", Medium1, 5),
    ("{10A1B5D5-9B99-4C35-A422-299274C87663}", Medium1, 6),
    ("{073A0DAA-6AF3-43AB-8588-CEC1D06C72B9}", Medium2, 0),
    ("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", Medium2, 1),
    ("{21E4AEA4-8DFA-4A89-87EB-49C32662AFE0}", Medium2, 2),
    ("{F5AB1C69-6EDB-4FF4-983F-18BD219EF322}", Medium2, 3),
    ("{00A15C55-8517-42AA-B614-E9B94910E393}", Medium2, 4),
    ("{7DF18680-E054-41AD-8BC1-D1AEF772440D}", Medium2, 5),
    ("{93296810-A885-4BE3-A3E7-6D5BEEA58F35}", Medium2, 6),
    ("{8EC20E35-A176-4012-BC5E-935CFFF8708E}", Medium3, 0),
    ("{6E25E649-3F16-4E02-A733-19D2CDBF48F0}", Medium3, 1),
    ("{85BE263C-DBD7-4A20-BB59-AAB30ACAA65A}", Medium3, 2),
    ("{EB344D84-9AFB-497E-A393-DC336BA19D2E}", Medium3, 3),
    ("{EB9631B5-78F2-41C9-869B-9F39066F8104}", Medium3, 4),
    ("{74C1A8A3-306A-4EB7-A6B1-4F7E0EB9C5D6}", Medium3, 5),
    ("{2A488322-F2BA-4B5B-9748-0D474271808F}", Medium3, 6),
    ("{D7AC3CCA-C797-4891-BE02-D94E43425B78}", Medium4, 0),
    ("{69CF1AB2-1976-4502-BF36-3FF5EA218861}", Medium4, 1),
    ("{8A107856-5554-42FB-B03E-39F5DBC370BA}", Medium4, 2),
    ("{0505E3EF-67EA-436B-97B2-0124C06EBD24}", Medium4, 3),
    ("{C4B1156A-380E-4F78-BDF5-A606A8083BF9}", Medium4, 4),
    ("{22838BEF-8BB2-4498-84A7-C5851F593DF1}", Medium4, 5),
    ("{16D9F66E-5EB9-4882-86FB-DCBF35E3C3E4}", Medium4, 6),
    ("{E8034E78-7F5D-4C2E-B375-FC64B27BC917}", Dark1, 0),
    ("{125E5076-3810-47DD-B79F-674D7AD40C01}", Dark1, 1),
    ("{37CE84F3-28C3-443E-9E96-99CF82512B78}", Dark1, 2),
    ("{D03447BB-5D67-496B-8E87-E561075AD55C}", Dark1, 3),
    ("{E929F9F4-4A8F-4326-A1B4-22849713DDAB}", Dark1, 4),
    ("{8FD4443E-F989-4FC4-A0C8-D5A2AF1F390B}", Dark1, 5),
    ("{AF606853-7671-496A-8E4F-DF71F8EC918B}", Dark1, 6),
    ("{5202B0CA-FC54-4496-8BCA-5EF66A818D29}", Dark2, 0),
    ("{0660B408-B3CF-4A94-85FC-2B1E0A45F4A2}", Dark2, 1),
    ("{91EBBBCC-DAD2-459C-BE2E-F6DE35CF9A28}", Dark2, 3),
    ("{46F890A9-2807-4EBB-B81D-B2AA78EC7F39}", Dark2, 5),
];

/// Une couleur de schéma DrawingML, avec un modificateur (`tint`, `shade`,
/// `alpha`) en millièmes de pour cent.
fn clr(slot: &str, modifier: Option<(&str, u32)>) -> String {
    match modifier {
        Some((m, v)) => format!(r#"<a:schemeClr val="{slot}"><a:{m} val="{v}"/></a:schemeClr>"#),
        None => format!(r#"<a:schemeClr val="{slot}"/>"#),
    }
}

/// Un remplissage plein de la couleur `c`.
fn solid(c: &str) -> String {
    format!("<a:fill><a:solidFill>{c}</a:solidFill></a:fill>")
}

/// Pas de remplissage.
const NO_FILL: &str = "<a:fill><a:noFill/></a:fill>";

/// Un trait de `w` EMU de la couleur `c`.
fn ln(w: u32, c: &str) -> String {
    format!(r#"<a:ln w="{w}"><a:solidFill>{c}</a:solidFill></a:ln>"#)
}

/// Un trait double de `w` EMU de la couleur `c`.
fn dbl(w: u32, c: &str) -> String {
    format!(r#"<a:ln w="{w}" cmpd="dbl"><a:solidFill>{c}</a:solidFill></a:ln>"#)
}

/// Le trait `idx` (1..3) du thème, de la couleur `c`.
fn ln_ref(idx: u32, c: &str) -> String {
    format!(r#"<a:lnRef idx="{idx}">{c}</a:lnRef>"#)
}

/// Pas de trait.
const NO_LINE: &str = "<a:ln><a:noFill/></a:ln>";

/// Les bords `sides` (`left`, `insideH`…) d'une partie.
fn borders(sides: &[(&str, &str)]) -> String {
    let inner = sides.iter().fold(String::new(), |mut inner, (side, line)| {
        let _ = write!(inner, "<a:{side}>{line}</a:{side}>");
        inner
    });
    format!("<a:tcBdr>{inner}</a:tcBdr>")
}

/// Les six bords d'un tableau, tous du trait `line`.
fn grid(line: &str) -> String {
    borders(&[("left", line), ("right", line), ("top", line), ("bottom", line), ("insideH", line), ("insideV", line)])
}

/// Le texte d'une partie : gras (`b`), police du corps du thème, couleur `color`.
fn text(bold: bool, color: Option<&str>) -> String {
    let b = if bold { r#" b="on""# } else { "" };
    match color {
        Some(c) => format!(r#"<a:tcTxStyle{b}><a:fontRef idx="minor"/>{c}</a:tcTxStyle>"#),
        None => format!("<a:tcTxStyle{b}/>"),
    }
}

/// Une partie `name` : texte `tx`, bords `bdr`, remplissage `fill`.
fn part(name: &str, tx: &str, bdr: &str, fill: &str) -> String {
    format!("<a:{name}>{tx}<a:tcStyle>{bdr}{fill}</a:tcStyle></a:{name}>")
}

/// Les parties d'un style : le tableau entier, les bandes horizontale et
/// verticale, les colonnes extrêmes, les lignes extrêmes.
struct Parts {
    whole: String,
    bands: [String; 2],
    first_col: String,
    last_col: String,
    first_row: String,
    last_row: String,
    /// Fond du tableau (`tblBg`), sous les cellules.
    background: Option<String>,
}

impl Parts {
    /// Le `a:tblStyle` d'id `id`.
    fn xml(&self, id: &str) -> String {
        let bg = self.background.as_deref().map(|f| format!("<a:tblBg>{f}</a:tblBg>")).unwrap_or_default();
        let [h, v] = &self.bands;
        let (w, fc, lc, fr, lr) = (&self.whole, &self.first_col, &self.last_col, &self.first_row, &self.last_row);
        format!(r#"<a:tblStyle xmlns:a="{NS}" styleId="{id}">{bg}{w}{h}{v}{lc}{fc}{lr}{fr}</a:tblStyle>"#)
    }
}

/// Espace de noms DrawingML.
const NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

/// Les bandes horizontale et verticale de remplissage `fill`, sans bord.
fn bands(fill: &str) -> [String; 2] {
    ["band1H", "band1V"].map(|name| part(name, "", "<a:tcBdr/>", fill))
}

/// Colonne extrême en gras, sans autre effet.
fn bold_col(name: &str) -> String {
    part(name, &text(true, None), "<a:tcBdr/>", "")
}

/// Les parties de la famille `family` sur la couleur `a` (un accent, sinon la
/// couleur de base de la famille) et, pour le foncé 2, l'en-tête `head`.
#[expect(clippy::too_many_lines, reason = "lookup table of the built-in style families, one arm per family")]
fn parts(family: Family, a: &str, head: &str) -> Parts {
    let (lt1, dk1) = (clr("lt1", None), clr("dk1", None));
    let c = clr(a, None);
    let tint = |v| clr(a, Some(("tint", v)));
    match family {
        Themed1 => Parts {
            whole: part("wholeTbl", &text(false, Some(&dk1)), &grid(&ln_ref(1, &c)), NO_FILL),
            bands: bands(&solid(&clr(a, Some(("alpha", 40000))))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &c))]), NO_FILL),
            background: None,
        },
        Themed2 => Parts {
            whole: {
                let outer = ln_ref(1, &clr(a, Some(("tint", 50000))));
                let bdr = borders(&[
                    ("left", &outer),
                    ("right", &outer),
                    ("top", &outer),
                    ("bottom", &outer),
                    ("insideH", NO_LINE),
                    ("insideV", NO_LINE),
                ]);
                part("wholeTbl", &text(false, Some(&lt1)), &bdr, NO_FILL)
            },
            bands: bands(&solid(&clr("lt1", Some(("alpha", 20000))))),
            first_col: part("firstCol", &text(true, None), &borders(&[("right", &ln_ref(2, &lt1))]), ""),
            last_col: part("lastCol", &text(true, None), &borders(&[("left", &ln_ref(2, &lt1))]), ""),
            first_row: part("firstRow", &text(true, None), &borders(&[("bottom", &ln_ref(3, &lt1))]), ""),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &ln_ref(2, &lt1))]), NO_FILL),
            background: Some(solid(&c)),
        },
        Light1 => Parts {
            whole: {
                let line = ln(12700, &c);
                let bdr = borders(&[("top", &line), ("bottom", &line)]);
                part("wholeTbl", &text(false, Some(&clr("tx1", None))), &bdr, NO_FILL)
            },
            bands: bands(&solid(&clr(a, Some(("alpha", 20000))))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, None), &borders(&[("bottom", &ln(12700, &c))]), NO_FILL),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &ln(12700, &c))]), NO_FILL),
            background: None,
        },
        Light2 => Parts {
            whole: {
                let line = ln(12700, &c);
                let bdr = borders(&[("left", &line), ("right", &line), ("top", &line), ("bottom", &line)]);
                part("wholeTbl", &text(false, Some(&clr("tx1", None))), &bdr, NO_FILL)
            },
            bands: [
                part("band1H", "", &borders(&[("top", &ln(12700, &c)), ("bottom", &ln(12700, &c))]), ""),
                part("band1V", "", &borders(&[("left", &ln(12700, &c)), ("right", &ln(12700, &c))]), ""),
            ],
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&clr("bg1", None))), "<a:tcBdr/>", &solid(&c)),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &c))]), NO_FILL),
            background: None,
        },
        Light3 => Parts {
            whole: part("wholeTbl", &text(false, Some(&clr("tx1", None))), &grid(&ln(12700, &c)), NO_FILL),
            bands: bands(&solid(&clr(a, Some(("alpha", 20000))))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&c)), &borders(&[("bottom", &ln(25400, &c))]), NO_FILL),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &c))]), NO_FILL),
            background: None,
        },
        Medium1 => Parts {
            whole: {
                let line = ln(12700, &c);
                let bdr = borders(&[
                    ("left", &line),
                    ("right", &line),
                    ("top", &line),
                    ("bottom", &line),
                    ("insideH", &line),
                    ("insideV", NO_LINE),
                ]);
                part("wholeTbl", &text(false, Some(&dk1)), &bdr, &solid(&lt1))
            },
            bands: bands(&solid(&tint(20000))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &c))]), &solid(&lt1)),
            background: None,
        },
        Medium2 => Parts {
            whole: part("wholeTbl", &text(false, Some(&dk1)), &grid(&ln(12700, &lt1)), &solid(&tint(20000))),
            bands: bands(&solid(&tint(40000))),
            first_col: part("firstCol", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            last_col: part("lastCol", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            first_row: part("firstRow", &text(true, Some(&lt1)), &borders(&[("bottom", &ln(38100, &lt1))]), &solid(&c)),
            last_row: part("lastRow", &text(true, Some(&lt1)), &borders(&[("top", &ln(38100, &lt1))]), &solid(&c)),
            background: None,
        },
        Medium3 => Parts {
            whole: {
                let line = ln(25400, &dk1);
                let bdr = borders(&[
                    ("left", NO_LINE),
                    ("right", NO_LINE),
                    ("top", &line),
                    ("bottom", &line),
                    ("insideH", NO_LINE),
                    ("insideV", NO_LINE),
                ]);
                part("wholeTbl", &text(false, Some(&dk1)), &bdr, &solid(&lt1))
            },
            bands: bands(&solid(&clr("dk1", Some(("tint", 20000))))),
            first_col: part("firstCol", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            last_col: part("lastCol", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&c)),
            first_row: part("firstRow", &text(true, Some(&lt1)), &borders(&[("bottom", &ln(25400, &dk1))]), &solid(&c)),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &dk1))]), &solid(&lt1)),
            background: None,
        },
        Medium4 => Parts {
            whole: part("wholeTbl", &text(false, Some(&dk1)), &grid(&ln(12700, &c)), &solid(&tint(20000))),
            bands: bands(&solid(&tint(40000))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&c)), "<a:tcBdr/>", &solid(&tint(20000))),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &ln(25400, &c))]), &solid(&tint(20000))),
            background: None,
        },
        Dark1 => {
            // Sans accent, le noir s'éclaircit (`tint`) ; un accent s'assombrit (`shade`).
            let tone = |v| if a == "dk1" { clr(a, Some(("tint", 100_000 - v))) } else { clr(a, Some(("shade", v))) };
            let edge = |side| borders(&[(side, &ln(25400, &lt1))]);
            Parts {
                whole: part("wholeTbl", &text(false, Some(&lt1)), &grid(NO_LINE), &solid(&tone(20000))),
                bands: bands(&solid(&tone(40000))),
                first_col: part("firstCol", &text(true, None), &edge("right"), &solid(&tone(60000))),
                last_col: part("lastCol", &text(true, None), &edge("left"), &solid(&tone(60000))),
                first_row: part("firstRow", &text(true, None), &edge("bottom"), &solid(&dk1)),
                last_row: part("lastRow", &text(true, None), &edge("top"), &solid(&c)),
                background: None,
            }
        }
        Dark2 => Parts {
            whole: part("wholeTbl", &text(false, Some(&dk1)), &grid(NO_LINE), &solid(&tint(20000))),
            bands: bands(&solid(&tint(40000))),
            first_col: bold_col("firstCol"),
            last_col: bold_col("lastCol"),
            first_row: part("firstRow", &text(true, Some(&lt1)), "<a:tcBdr/>", &solid(&clr(head, None))),
            last_row: part("lastRow", &text(true, None), &borders(&[("top", &dbl(50800, &dk1))]), &solid(&tint(20000))),
            background: None,
        },
    }
}

/// La définition DrawingML (`a:tblStyle`) du style intégré de GUID `id`
/// (casse ignorée), `None` pour un GUID inconnu.
pub(crate) fn builtin(id: &str) -> Option<String> {
    let &(guid, family, accent) = BUILTIN.iter().find(|(g, ..)| g.eq_ignore_ascii_case(id))?;
    let base = match family {
        Themed1 | Themed2 | Light1 | Light2 | Light3 => "tx1",
        Medium1 | Medium2 | Medium3 | Medium4 | Dark1 | Dark2 => "dk1",
    };
    let a = if accent == 0 { base.to_owned() } else { format!("accent{accent}") };
    let head = if accent == 0 { base.to_owned() } else { format!("accent{}", accent + 1) };
    let parts = match (family, accent) {
        // Sans accent, les styles thématiques sont « sans style » : sans quadrillage, ou quadrillé de `tx1`.
        (Themed1, 0) => no_style(None),
        (Themed2, 0) => no_style(Some(&ln(12700, &clr("tx1", None)))),
        _ => parts(family, &a, &head),
    };
    Some(parts.xml(guid))
}

/// Un style « sans style » : texte `tx1`, quadrillage `grid_line` s'il y en a un.
fn no_style(grid_line: Option<&str>) -> Parts {
    let bdr = grid_line.map_or_else(|| "<a:tcBdr/>".to_owned(), grid);
    Parts {
        whole: part("wholeTbl", &text(false, Some(&clr("tx1", None))), &bdr, NO_FILL),
        bands: bands(""),
        first_col: bold_col("firstCol"),
        last_col: bold_col("lastCol"),
        first_row: bold_col("firstRow"),
        last_row: bold_col("lastRow"),
        background: None,
    }
}

#[cfg(test)]
#[path = "table_styles_tests.rs"]
mod table_styles_tests;
