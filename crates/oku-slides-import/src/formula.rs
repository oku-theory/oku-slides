//! Formules des guides DrawingML (ECMA-376, 20.1.9.11) : `*/ w adj 100000`.
//!
//! Angles en 60 000ᵉ de degré. Une division par zéro vaut 0, la racine d'un
//! négatif vaut 0 ; un résultat non fini refuse l'évaluation (`None`), comme
//! un nom que rien ne définit.

use std::collections::HashMap;
use std::f64::consts::PI;

/// Opérande : littéral ou nom (guide, ajustement, constante prédéfinie).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Operand {
    Lit(f64),
    Name(String),
}

impl Operand {
    /// Un littéral commence par un chiffre, un signe ou un point : un guide
    /// nommé `inf` ou `nan` reste un nom.
    pub(crate) fn parse(token: &str) -> Operand {
        let numeric = token.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'));
        match token.parse::<f64>() {
            Ok(v) if numeric && v.is_finite() => Operand::Lit(v),
            _ => Operand::Name(token.to_owned()),
        }
    }
}

/// Opérateur d'une formule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    /// `x * y / z`.
    MulDiv,
    /// `x + y - z`.
    AddSub,
    /// `(x + y) / z`.
    AddDiv,
    /// `x > 0 ? y : z`.
    IfElse,
    Abs,
    /// `atan2(y, x)`, en 60 000ᵉ de degré.
    At2,
    /// `x * cos(atan2(z, y))`.
    Cat2,
    /// `x * cos(y)`.
    Cos,
    Max,
    Min,
    /// `sqrt(x² + y² + z²)`.
    Mod,
    /// `y` borné à `[x, z]`.
    Pin,
    /// `x * sin(atan2(z, y))`.
    Sat2,
    /// `x * sin(y)`.
    Sin,
    Sqrt,
    /// `x * tan(y)`.
    Tan,
    Val,
}

impl Op {
    fn parse(token: &str) -> Option<(Op, usize)> {
        Some(match token {
            "*/" => (Op::MulDiv, 3),
            "+-" => (Op::AddSub, 3),
            "+/" => (Op::AddDiv, 3),
            "?:" => (Op::IfElse, 3),
            "abs" => (Op::Abs, 1),
            "at2" => (Op::At2, 2),
            "cat2" => (Op::Cat2, 3),
            "cos" => (Op::Cos, 2),
            "max" => (Op::Max, 2),
            "min" => (Op::Min, 2),
            "mod" => (Op::Mod, 3),
            "pin" => (Op::Pin, 3),
            "sat2" => (Op::Sat2, 3),
            "sin" => (Op::Sin, 2),
            "sqrt" => (Op::Sqrt, 1),
            "tan" => (Op::Tan, 2),
            "val" => (Op::Val, 1),
            _ => return None,
        })
    }
}

/// Formule analysée ; `None` si l'opérateur est inconnu ou s'il manque un
/// opérande. Un opérande de trop est ignoré : les définitions d'origine en
/// ont (`+- xH 0 dxB 0` dans `circularArrow`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Formula {
    op: Op,
    args: Vec<Operand>,
}

impl Formula {
    pub(crate) fn parse(fmla: &str) -> Option<Formula> {
        let mut tokens = fmla.split_whitespace();
        let (op, arity) = Op::parse(tokens.next()?)?;
        let args: Vec<Operand> = tokens.take(arity).map(Operand::parse).collect();
        (args.len() == arity).then_some(Formula { op, args })
    }

    /// Valeur de la formule dans `env`.
    #[expect(clippy::many_single_char_names, reason = "operands x, y, z of OOXML guide formulas")]
    pub(crate) fn eval(&self, env: &Env<'_>) -> Option<f64> {
        let mut v = [0.0; 3];
        for (slot, a) in v.iter_mut().zip(&self.args) {
            *slot = env.get(a)?;
        }
        let [x, y, z] = v;
        let r = match self.op {
            Op::MulDiv => div(x * y, z),
            Op::AddSub => x + y - z,
            Op::AddDiv => div(x + y, z),
            Op::IfElse => {
                if x > 0.0 {
                    y
                } else {
                    z
                }
            }
            Op::Abs => x.abs(),
            Op::At2 => y.atan2(x) / ANGLE,
            Op::Cat2 => x * z.atan2(y).cos(),
            Op::Cos => x * (y * ANGLE).cos(),
            Op::Max => x.max(y),
            Op::Min => x.min(y),
            Op::Mod => (x * x + y * y + z * z).sqrt(),
            Op::Pin => {
                if y < x {
                    x
                } else if y > z {
                    z
                } else {
                    y
                }
            }
            Op::Sat2 => x * z.atan2(y).sin(),
            Op::Sin => x * (y * ANGLE).sin(),
            Op::Sqrt => x.max(0.0).sqrt(),
            Op::Tan => x * (y * ANGLE).tan(),
            Op::Val => x,
        };
        r.is_finite().then_some(r)
    }
}

/// Une unité d'angle DrawingML (1/60 000ᵉ de degré), en radians.
pub(crate) const ANGLE: f64 = PI / 180.0 / 60_000.0;

fn div(n: f64, d: f64) -> f64 {
    if d == 0.0 { 0.0 } else { n / d }
}

/// Valeurs nommées d'une évaluation : guides et ajustements déjà calculés,
/// puis constantes prédéfinies tirées de la taille de la forme.
pub(crate) struct Env<'a> {
    w: f64,
    h: f64,
    names: HashMap<&'a str, f64>,
}

impl<'a> Env<'a> {
    pub(crate) fn new(w: f64, h: f64) -> Self {
        Env { w, h, names: HashMap::new() }
    }

    pub(crate) fn set(&mut self, name: &'a str, v: f64) {
        self.names.insert(name, v);
    }

    pub(crate) fn get(&self, operand: &Operand) -> Option<f64> {
        match operand {
            Operand::Lit(v) => Some(*v),
            Operand::Name(n) => self.names.get(n.as_str()).copied().or_else(|| self.builtin(n)),
        }
    }

    /// Constantes prédéfinies : bords et centre, `ss`/`ls` (petit et grand
    /// côté), `wdN`/`hdN`/`ssdN` (côté divisé par N), angles `cd2`… `7cd8`.
    fn builtin(&self, name: &str) -> Option<f64> {
        let (w, h) = (self.w, self.h);
        let ss = w.min(h);
        Some(match name {
            "l" | "t" => 0.0,
            "r" | "w" => w,
            "b" | "h" => h,
            "hc" => w / 2.0,
            "vc" => h / 2.0,
            "ss" => ss,
            "ls" => w.max(h),
            "cd2" => 10_800_000.0,
            "cd4" => 5_400_000.0,
            "cd8" => 2_700_000.0,
            "3cd4" => 16_200_000.0,
            "3cd8" => 8_100_000.0,
            "5cd8" => 13_500_000.0,
            "7cd8" => 18_900_000.0,
            _ => {
                let (side, n) = if let Some(n) = name.strip_prefix("ssd") {
                    (ss, n)
                } else if let Some(n) = name.strip_prefix("wd") {
                    (w, n)
                } else {
                    (h, name.strip_prefix("hd")?)
                };
                let n: u32 = n.parse().ok().filter(|n| *n > 0)?;
                side / f64::from(n)
            }
        })
    }
}

#[cfg(test)]
#[path = "formula_tests.rs"]
mod formula_tests;
