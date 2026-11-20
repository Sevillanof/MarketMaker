//! Cupones de descuento de una tienda: el dueño los crea en el panel y el
//! comprador escribe el código al pagar. Se usan igual en el navegador y en
//! el servidor.

use serde::{Deserialize, Serialize};

use crate::{parse_price, FieldError};

/// El código más largo.
pub const COUPON_CODE_MAX: usize = 20;
/// El descuento en por ciento más grande (más que eso no tiene sentido).
pub const COUPON_PERCENT_MAX: i64 = 90;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CouponKind {
    /// Un por ciento de los productos.
    #[default]
    Percent,
    /// Un monto fijo en pesos.
    Amount,
}

impl CouponKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CouponKind::Percent => "percent",
            CouponKind::Amount => "amount",
        }
    }

    pub fn from_str(s: &str) -> Self {
        if s == "amount" {
            CouponKind::Amount
        } else {
            CouponKind::Percent
        }
    }
}

/// Un cupón guardado.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Coupon {
    pub id: String,
    /// En mayúsculas, sin espacios ni tildes.
    pub code: String,
    pub kind: CouponKind,
    /// Por ciento (`Percent`) o centavos (`Amount`).
    pub value: i64,
    /// Compra mínima en centavos (0: sin mínimo).
    pub min_cents: i64,
    /// Último día en que sirve, `AAAA-MM-DD` (vacío: no vence).
    pub expires: String,
    /// Usos máximos (0: sin tope).
    pub max_uses: u32,
    pub uses: u32,
    pub active: bool,
}

/// Lo que ve y puede hacer el comprador con un cupón que sirve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CouponOffer {
    pub code: String,
    pub kind: CouponKind,
    pub value: i64,
    pub min_cents: i64,
}

/// La respuesta de «Aplicar»: el cupón, o por qué no sirve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CouponCheck {
    pub offer: Option<CouponOffer>,
    pub problem: String,
}

/// En qué estado está un cupón hoy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CouponStatus {
    Active,
    Paused,
    Expired,
    Exhausted,
}

impl CouponOffer {
    /// El descuento sobre `subtotal_cents` (0 si no llega al mínimo).
    pub fn discount_cents(&self, subtotal_cents: i64) -> i64 {
        if subtotal_cents <= 0 || subtotal_cents < self.min_cents {
            return 0;
        }
        match self.kind {
            CouponKind::Percent => {
                // El precio final queda en pesos enteros: el descuento absorbe
                // los centavos (a favor del comprador).
                let pct = self.value.clamp(0, COUPON_PERCENT_MAX);
                let final_cents = (subtotal_cents - subtotal_cents * pct / 100) / 100 * 100;
                subtotal_cents - final_cents
            }
            CouponKind::Amount => self.value.clamp(0, subtotal_cents),
        }
    }

    /// Si el pedido no llega a la compra mínima, el aviso para el comprador.
    pub fn min_problem(&self, subtotal_cents: i64) -> Option<String> {
        (subtotal_cents < self.min_cents).then(|| {
            format!(
                "Este cupón sirve en compras desde {}.",
                peso_text(self.min_cents)
            )
        })
    }

    /// «10 % de descuento» o «$ 5.000 de descuento».
    pub fn label(&self) -> String {
        match self.kind {
            CouponKind::Percent => format!("{} % de descuento", self.value),
            CouponKind::Amount => format!("{} de descuento", peso_text(self.value)),
        }
    }
}

impl Coupon {
    /// El descuento, para la tabla: «10 %», «$ 5.000».
    pub fn value_text(&self) -> String {
        match self.kind {
            CouponKind::Percent => format!("{} %", self.value),
            CouponKind::Amount => peso_text(self.value),
        }
    }

    /// El formulario con los datos de este cupón, para editarlo.
    pub fn to_form(&self) -> CouponForm {
        CouponForm {
            id: self.id.clone(),
            code: self.code.clone(),
            kind: self.kind,
            value: match self.kind {
                CouponKind::Percent => self.value.to_string(),
                CouponKind::Amount => (self.value / 100).to_string(),
            },
            min: if self.min_cents > 0 {
                (self.min_cents / 100).to_string()
            } else {
                String::new()
            },
            expires: self.expires.clone(),
            max_uses: if self.max_uses > 0 {
                self.max_uses.to_string()
            } else {
                String::new()
            },
        }
    }

    pub fn offer(&self) -> CouponOffer {
        CouponOffer {
            code: self.code.clone(),
            kind: self.kind,
            value: self.value,
            min_cents: self.min_cents,
        }
    }

    /// `today` es la fecha de hoy (`AAAA-MM-DD`, hora argentina).
    pub fn status(&self, today: &str) -> CouponStatus {
        if !self.active {
            CouponStatus::Paused
        } else if !self.expires.is_empty() && self.expires.as_str() < today {
            CouponStatus::Expired
        } else if self.max_uses > 0 && self.uses >= self.max_uses {
            CouponStatus::Exhausted
        } else {
            CouponStatus::Active
        }
    }

    /// Por qué no sirve hoy (sin mirar el monto del pedido), dicho al
    /// comprador. `None`: sirve.
    pub fn status_problem(&self, today: &str) -> Option<String> {
        match self.status(today) {
            CouponStatus::Paused => Some("Este cupón no está disponible.".into()),
            CouponStatus::Expired => Some("Este cupón venció. Probá con otro.".into()),
            CouponStatus::Exhausted => Some("Este cupón ya no tiene usos disponibles.".into()),
            CouponStatus::Active => None,
        }
    }

    /// Por qué no sirve ahora para un pedido de `subtotal_cents`.
    pub fn problem(&self, subtotal_cents: i64, today: &str) -> Option<String> {
        self.status_problem(today)
            .or_else(|| self.offer().min_problem(subtotal_cents))
    }
}

/// La pantalla Cupones de una tienda, en el panel.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CouponsInfo {
    pub store_name: String,
    /// Hoy (`AAAA-MM-DD`, hora argentina), para ver qué cupones vencieron.
    pub today: String,
    pub coupons: Vec<Coupon>,
}

/// Qué pasó al guardar un cupón: los problemas de cada campo (vacío si se
/// guardó) y los cupones como quedaron.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CouponSaved {
    pub errors: Vec<FieldError>,
    pub coupons: Vec<Coupon>,
}

/// `2026-12-31` → `31/12/2026`.
pub fn date_text(iso: &str) -> String {
    match (iso.get(0..4), iso.get(5..7), iso.get(8..10)) {
        (Some(y), Some(m), Some(d)) => format!("{d}/{m}/{y}"),
        _ => String::new(),
    }
}

/// Lo que escribe el dueño en «Nuevo cupón».
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CouponForm {
    /// Vacío: cupón nuevo.
    #[serde(default)]
    pub id: String,
    pub code: String,
    pub kind: CouponKind,
    pub value: String,
    pub min: String,
    pub expires: String,
    pub max_uses: String,
}

/// `$ 20.000` (pesos enteros, con puntos de miles).
pub fn peso_text(cents: i64) -> String {
    let units = (cents / 100).to_string();
    let mut out = String::new();
    for (i, ch) in units.chars().enumerate() {
        if i > 0 && (units.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    format!("$ {out}")
}

/// Deja un código como se guarda: sin espacios, sin tildes, en mayúsculas,
/// solo letras y números.
pub fn normalize_coupon_code(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            let c = match c {
                'á' | 'Á' => 'A',
                'é' | 'É' => 'E',
                'í' | 'Í' => 'I',
                'ó' | 'Ó' => 'O',
                'ú' | 'Ú' | 'ü' | 'Ü' => 'U',
                'ñ' | 'Ñ' => 'N',
                c => c.to_ascii_uppercase(),
            };
            c.is_ascii_alphanumeric().then_some(c)
        })
        .take(COUPON_CODE_MAX)
        .collect()
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |a: usize, z: usize| s.get(a..z).and_then(|t| t.parse::<u32>().ok());
    matches!((num(0, 4), num(5, 7), num(8, 10)),
        (Some(y), Some(m), Some(d)) if y >= 2000 && (1..=12).contains(&m) && (1..=31).contains(&d))
}

/// Revisa lo que escribió el dueño y arma el cupón (sin `id` ni usos). Los
/// problemas van al lado de cada campo: `code`, `value`, `min`, `expires` y
/// `max_uses`.
pub fn tidy_coupon(f: &CouponForm) -> Result<Coupon, Vec<FieldError>> {
    let mut errors = Vec::new();
    let mut err = |field: &str, message: &str| {
        errors.push(FieldError {
            field: field.into(),
            message: message.into(),
        })
    };
    let code = normalize_coupon_code(&f.code);
    if code.len() < 3 {
        err("code", "Escribí un código de al menos 3 letras o números.");
    }
    let value = f.value.trim();
    let mut v = 0;
    match f.kind {
        CouponKind::Percent => match value.parse::<i64>() {
            Ok(n) if (1..=COUPON_PERCENT_MAX).contains(&n) => v = n,
            _ => err("value", "Escribí un porcentaje entre 1 y 90."),
        },
        CouponKind::Amount => match parse_price(value) {
            Some(c) if c >= 100 => v = c / 100 * 100,
            _ => err("value", "Escribí un monto en pesos, por ejemplo 5.000."),
        },
    }
    let mut min_cents = 0;
    if !f.min.trim().is_empty() {
        match parse_price(&f.min) {
            Some(c) => min_cents = c / 100 * 100,
            None => err(
                "min",
                "Escribí la compra mínima en pesos, por ejemplo 20.000.",
            ),
        }
    }
    let expires = f.expires.trim().to_string();
    if !expires.is_empty() && !valid_date(&expires) {
        err("expires", "Elegí una fecha de vencimiento válida.");
    }
    let mut max_uses = 0;
    if !f.max_uses.trim().is_empty() {
        match f.max_uses.trim().parse::<u32>() {
            Ok(n) if (1..=100_000).contains(&n) => max_uses = n,
            _ => err(
                "max_uses",
                "Escribí cuántas veces se puede usar (un número desde 1).",
            ),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Coupon {
        id: f.id.clone(),
        code,
        kind: f.kind,
        value: v,
        min_cents,
        expires,
        max_uses,
        uses: 0,
        active: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(kind: CouponKind, value: i64, min: i64) -> CouponOffer {
        CouponOffer {
            code: "X".into(),
            kind,
            value,
            min_cents: min,
        }
    }

    #[test]
    fn percent_and_amount_discounts() {
        assert_eq!(
            offer(CouponKind::Percent, 10, 0).discount_cents(17_999_800),
            1_800_000
        );
        assert_eq!(
            offer(CouponKind::Amount, 500_000, 0).discount_cents(300_000),
            300_000
        );
        assert_eq!(
            offer(CouponKind::Amount, 500_000, 0).discount_cents(900_000),
            500_000
        );
        // No llega al mínimo: no descuenta.
        assert_eq!(
            offer(CouponKind::Percent, 10, 2_000_000).discount_cents(1_000_000),
            0
        );
    }

    #[test]
    fn code_is_normalized() {
        assert_eq!(normalize_coupon_code(" bienvenida-10 "), "BIENVENIDA10");
        assert_eq!(normalize_coupon_code("Envío Ñandú"), "ENVIONANDU");
    }

    #[test]
    fn status_and_problems() {
        let c = Coupon {
            code: "A".into(),
            expires: "2026-12-31".into(),
            max_uses: 2,
            uses: 1,
            active: true,
            min_cents: 100_000,
            ..Coupon::default()
        };
        assert_eq!(c.status("2026-12-31"), CouponStatus::Active);
        assert_eq!(c.status("2027-01-01"), CouponStatus::Expired);
        assert!(c.problem(50_000, "2026-11-20").unwrap().contains("$ 1.000"));
        assert_eq!(c.problem(100_000, "2026-11-20"), None);
        let used = Coupon {
            uses: 2,
            ..c.clone()
        };
        assert_eq!(used.status("2026-11-20"), CouponStatus::Exhausted);
        let off = Coupon { active: false, ..c };
        assert_eq!(off.status("2026-11-20"), CouponStatus::Paused);
    }

    #[test]
    fn form_is_checked() {
        let ok = CouponForm {
            code: "verano 15".into(),
            value: "15".into(),
            ..CouponForm::default()
        };
        let c = tidy_coupon(&ok).unwrap();
        assert_eq!((c.code.as_str(), c.value), ("VERANO15", 15));
        let bad = CouponForm {
            code: "a".into(),
            value: "150".into(),
            expires: "ayer".into(),
            max_uses: "0".into(),
            ..CouponForm::default()
        };
        let fields: Vec<_> = tidy_coupon(&bad)
            .unwrap_err()
            .into_iter()
            .map(|e| e.field)
            .collect();
        assert_eq!(fields, ["code", "value", "expires", "max_uses"]);
    }
}
