//! Envíos con Andreani: el paquete de cada producto (peso y medidas), la
//! cuenta de Andreani de la tienda y lo que se le manda al cotizar y al dar
//! de alta un envío.
//!
//! Andreani se conecta con el "Credential ID" de su cuenta PyME (el de la
//! opción WooCommerce, en pymes.andreani.com/integraciones).

use serde::{Deserialize, Serialize};

/// Peso y medidas de un producto empaquetado, como lo cotiza Andreani.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parcel {
    /// Gramos.
    pub weight_g: u32,
    /// Milímetros (se cargan en centímetros, con decimales).
    pub length_mm: u32,
    pub width_mm: u32,
    pub height_mm: u32,
}

impl Default for Parcel {
    /// Una caja chica: medio kilo, 20 × 15 × 10 cm.
    fn default() -> Self {
        Self {
            weight_g: 500,
            length_mm: 200,
            width_mm: 150,
            height_mm: 100,
        }
    }
}

/// Más que esto es seguro un error de tipeo (Andreani lleva hasta 1.000 kg).
pub const PARCEL_MAX_G: u32 = 1_000_000;
/// Ningún lado puede pasar de 3 metros.
pub const PARCEL_MAX_MM: u32 = 3_000;
/// Andreani cobra como "Bigger" (paquetes grandes) lo que pasa de esto.
pub const BIGGER_WEIGHT_G: u32 = 50_000;
pub const BIGGER_SIDE_MM: u32 = 1_650;
pub const BIGGER_SUM_MM: u32 = 3_000;

impl Parcel {
    /// "0,5 kg · 20 × 15 × 10 cm".
    pub fn summary(&self) -> String {
        format!(
            "{} kg · {} × {} × {} cm",
            kg_input(self.weight_g),
            cm_input(self.length_mm),
            cm_input(self.width_mm),
            cm_input(self.height_mm)
        )
    }

    /// Supera lo que Andreani lleva como paquete común.
    pub fn is_bigger(&self) -> bool {
        let sides = [self.length_mm, self.width_mm, self.height_mm];
        self.weight_g > BIGGER_WEIGHT_G
            || sides.iter().any(|s| *s > BIGGER_SIDE_MM)
            || sides.iter().sum::<u32>() > BIGGER_SUM_MM
    }
}

/// Un número escrito con coma o punto decimal ("0,5", "1.25", "2").
fn parse_decimal(text: &str) -> Option<f64> {
    let t = text.trim().replace(' ', "");
    if t.is_empty() {
        return None;
    }
    // "1.250,5": el punto separa miles. "0.5": es decimal.
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t
    };
    let n: f64 = t.parse().ok()?;
    (n.is_finite() && n > 0.0).then_some(n)
}

/// Kilos escritos por el dueño ("0,5") en gramos. `None` si no es un número.
pub fn parse_kg(text: &str) -> Option<u32> {
    let g = (parse_decimal(text)? * 1000.0).round();
    (g >= 1.0 && g <= PARCEL_MAX_G as f64).then_some(g as u32)
}

/// Centímetros escritos por el dueño ("12,5") en milímetros.
pub fn parse_cm(text: &str) -> Option<u32> {
    let mm = (parse_decimal(text)? * 10.0).round();
    (mm >= 1.0 && mm <= PARCEL_MAX_MM as f64).then_some(mm as u32)
}

/// Gramos como se escriben en kilos: 500 → "0,5"; 1250 → "1,25".
pub fn kg_input(grams: u32) -> String {
    decimal_text(grams, 1000)
}

/// Milímetros como se escriben en centímetros: 125 → "12,5".
pub fn cm_input(mm: u32) -> String {
    decimal_text(mm, 10)
}

fn decimal_text(n: u32, per: u32) -> String {
    let whole = n / per;
    let rest = n % per;
    if rest == 0 {
        return whole.to_string();
    }
    let digits = per.to_string().len() - 1;
    let frac = format!("{rest:0digits$}");
    format!("{whole},{}", frac.trim_end_matches('0'))
}

/// Los cuatro campos del paquete como los escribió el dueño. Todos vacíos:
/// `Ok(None)` (se usa el paquete de la tienda). Alguno vacío o mal: el
/// problema, en palabras simples.
pub fn parse_parcel(
    weight: &str,
    length: &str,
    width: &str,
    height: &str,
) -> Result<Option<Parcel>, String> {
    let all = [weight, length, width, height];
    if all.iter().all(|s| s.trim().is_empty()) {
        return Ok(None);
    }
    let weight_g = parse_kg(weight)
        .ok_or("Escribí el peso en kilos, por ejemplo 0,5 (medio kilo).")?;
    let cm = |s: &str| {
        parse_cm(s).ok_or("Escribí las tres medidas en centímetros, por ejemplo 20, 15 y 10.")
    };
    Ok(Some(Parcel {
        weight_g,
        length_mm: cm(length)?,
        width_mm: cm(width)?,
        height_mm: cm(height)?,
    }))
}

/// Los servicios de Andreani que puede ofrecer una tienda (según su cuenta).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AndreaniService {
    /// A domicilio ("estándar").
    Home,
    /// A una sucursal de Andreani, donde lo retira el comprador.
    Branch,
    /// Llega el mismo día (solo en algunas zonas).
    SameDay,
    /// Paquetes grandes, a domicilio.
    Bigger,
}

impl AndreaniService {
    pub const ALL: [AndreaniService; 4] = [
        AndreaniService::Home,
        AndreaniService::Branch,
        AndreaniService::SameDay,
        AndreaniService::Bigger,
    ];

    /// Cómo lo nombra Andreani en sus contratos y tarifas (ya normalizado).
    pub fn api_name(self) -> &'static str {
        match self {
            AndreaniService::Home => "estándar",
            AndreaniService::Branch => "sucursal",
            AndreaniService::SameDay => "llega hoy",
            AndreaniService::Bigger => "bigger",
        }
    }

    /// El servicio de un contrato o una tarifa de Andreani. Acepta los nombres
    /// de los contratos ("A domicilio", "A sucursal", "Llega hoy") y los de
    /// las tarifas ("estándar", "sucursal", "llega hoy", "bigger").
    pub fn from_api(name: &str) -> Option<Self> {
        let n = name.trim().to_lowercase();
        match n.as_str() {
            "estándar" | "estandar" | "a domicilio" | "domicilio" => Some(AndreaniService::Home),
            "sucursal" | "a sucursal" => Some(AndreaniService::Branch),
            "llega hoy" | "llega-hoy" => Some(AndreaniService::SameDay),
            "bigger" => Some(AndreaniService::Bigger),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AndreaniService::Home => "home",
            AndreaniService::Branch => "branch",
            AndreaniService::SameDay => "same_day",
            AndreaniService::Bigger => "bigger",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|x| x.as_str() == s)
    }

    /// Lo que ve el comprador.
    pub fn label(self) -> &'static str {
        match self {
            AndreaniService::Home => "Andreani a domicilio",
            AndreaniService::Branch => "Andreani a sucursal",
            AndreaniService::SameDay => "Andreani llega hoy",
            AndreaniService::Bigger => "Andreani a domicilio (paquete grande)",
        }
    }

    /// Lo que ve el dueño al elegir qué ofrecer.
    pub fn owner_hint(self) -> &'static str {
        match self {
            AndreaniService::Home => "Andreani lo lleva a la casa del comprador.",
            AndreaniService::Branch => "El comprador lo retira en la sucursal de Andreani que elija.",
            AndreaniService::SameDay => "Llega el mismo día, en las zonas donde Andreani lo ofrece.",
            AndreaniService::Bigger => "Para paquetes de más de 50 kg o muy grandes. Se usa solo cuando hace falta.",
        }
    }

    /// Necesita que el comprador elija una sucursal.
    pub fn needs_branch(self) -> bool {
        self == AndreaniService::Branch
    }
}

/// Un contrato de la cuenta de Andreani: cada servicio tiene el suyo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniContract {
    /// El `id` que pide el alta del envío.
    pub id: String,
    /// El número de contrato, para mostrar.
    pub number: String,
    pub service: AndreaniService,
}

/// Los servicios de los contratos, en el orden en que se muestran, sin repetir.
pub fn contract_services(contracts: &[AndreaniContract]) -> Vec<AndreaniService> {
    let mut v: Vec<_> = contracts.iter().map(|c| c.service).collect();
    v.sort();
    v.dedup();
    v
}

/// Desde dónde despacha la tienda (lo que va en la etiqueta como remitente).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct AndreaniOrigin {
    pub street: String,
    pub number: String,
    /// Piso y departamento (opcional).
    #[serde(default)]
    pub floor: String,
    pub city: String,
    pub province: String,
    pub postal_code: String,
    /// El nombre que sale en la etiqueta (la tienda).
    pub sender: String,
}

/// Lo que el dueño elige para Andreani en Entregas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniSettings {
    /// Si se ofrece en "Finalizar compra".
    pub enabled: bool,
    pub origin: AndreaniOrigin,
    /// Los servicios que se ofrecen (entre los que tiene la cuenta).
    pub services: Vec<AndreaniService>,
    /// El paquete de los productos que no tienen uno propio.
    pub parcel: Parcel,
}

/// La cuenta de Andreani conectada, para mostrarla.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniAccount {
    /// Titular, como lo tiene Andreani (puede venir vacío).
    pub holder: String,
    pub cuit: String,
    pub email: String,
    /// "pyme", "middle_market" o "corporativo".
    pub client_type: String,
    /// Ya guardó el origen (si no, lo que se muestra es una propuesta).
    pub configured: bool,
    /// Los servicios que tiene contratados.
    pub available: Vec<AndreaniService>,
    pub settings: AndreaniSettings,
}

/// Envíos con Andreani en la pantalla Entregas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniPanel {
    /// `None`: todavía no la conectó.
    pub account: Option<AndreaniAccount>,
    /// Para precargar el origen: la dirección de retiro y el nombre de la tienda.
    pub store_name: String,
    pub pickup_address: String,
}

/// Qué pasó al tocar "Conectar".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AndreaniConnect {
    Connected(AndreaniAccount),
    /// El código no sirve, o Andreani no responde: el mensaje para el dueño.
    Problem(String),
}

/// Lo que pasó al guardar Andreani.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniSaved {
    /// Problemas por campo: con alguno, no se guardó nada.
    pub errors: Vec<crate::FieldError>,
    /// Se guardó, pero Andreani no tomó el origen (se reintenta al guardar otra vez).
    pub warning: Option<String>,
}

/// Lo que cuesta cada servicio para un código postal y un carrito.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniQuote {
    /// El código postal cotizado (4 números).
    pub postal_code: String,
    pub rates: Vec<AndreaniRate>,
    /// Por qué no hay tarifas, en palabras para el comprador.
    pub problem: Option<String>,
}

impl AndreaniQuote {
    pub fn cents(&self, service: AndreaniService) -> Option<i64> {
        self.rates.iter().find(|r| r.service == service).map(|r| r.cents)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct AndreaniRate {
    pub service: AndreaniService,
    pub cents: i64,
}

/// Una sucursal de Andreani donde retirar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AndreaniBranch {
    pub code: String,
    pub name: String,
    /// "Av. Santa Fe 3200, CABA".
    pub address: String,
    /// Horario, si Andreani lo informa.
    #[serde(default)]
    pub hours: String,
}

/// En qué está el alta del envío en Andreani.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShipmentState {
    /// Todavía no se creó (el pedido no está pagado).
    #[default]
    Waiting,
    /// Se está creando ahora.
    Creating,
    /// Andreani lo tiene. Falta que el dueño lo pague en Andreani.
    Created,
    /// No se pudo crear: se ve el error y "Reintentar".
    Failed,
}

/// El envío con Andreani de un pedido.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Shipment {
    pub service: AndreaniService,
    /// Lo que se le cobró al comprador por el envío.
    pub quote_cents: i64,
    pub dni: String,
    /// Con retiro en sucursal.
    #[serde(default)]
    pub branch_code: String,
    #[serde(default)]
    pub branch_name: String,
    #[serde(default)]
    pub branch_address: String,
    #[serde(default)]
    pub state: ShipmentState,
    /// El número que da Andreani al crear el envío.
    #[serde(default)]
    pub pedido_id: String,
    /// El número de seguimiento (aparece cuando el dueño paga el envío).
    #[serde(default)]
    pub tracking: String,
    /// Estado según Andreani ("en camino", "entregado"…).
    #[serde(default)]
    pub status: String,
    /// Por qué falló el alta.
    #[serde(default)]
    pub error: String,
}

/// Las provincias, como las escribe Andreani.
pub const PROVINCES: [&str; 24] = [
    "Buenos Aires",
    "Ciudad Autónoma de Buenos Aires",
    "Catamarca",
    "Chaco",
    "Chubut",
    "Córdoba",
    "Corrientes",
    "Entre Ríos",
    "Formosa",
    "Jujuy",
    "La Pampa",
    "La Rioja",
    "Mendoza",
    "Misiones",
    "Neuquén",
    "Río Negro",
    "Salta",
    "San Juan",
    "San Luis",
    "Santa Cruz",
    "Santa Fe",
    "Santiago del Estero",
    "Tierra del Fuego",
    "Tucumán",
];

/// El código postal de 4 números: acepta "1425" y el CPA ("C1425ABC").
pub fn normalize_postal_code(text: &str) -> Option<String> {
    let t: String = text.trim().to_uppercase().chars().filter(|c| !c.is_whitespace()).collect();
    let b = t.as_bytes();
    if b.len() == 4 && b.iter().all(u8::is_ascii_digit) {
        return Some(t);
    }
    if (5..=8).contains(&b.len())
        && b[0].is_ascii_alphabetic()
        && b[1..5].iter().all(u8::is_ascii_digit)
        && b[5..].iter().all(u8::is_ascii_alphabetic)
    {
        return Some(t[1..5].to_string());
    }
    None
}

/// Nombre y apellido por separado (Andreani los pide así): el apellido es
/// la última palabra. Sin apellido, "-".
pub fn split_name(full: &str) -> (String, String) {
    let words: Vec<&str> = full.split_whitespace().collect();
    match words.as_slice() {
        [] => (String::new(), "-".to_string()),
        [one] => (one.to_string(), "-".to_string()),
        [first @ .., last] => (first.join(" "), last.to_string()),
    }
}

/// Calle y número por separado: "Av. Corrientes 1234" → ("Av. Corrientes",
/// "1234"). Sin número: `None`. Lo que sigue al número (piso, depto) se
/// devuelve aparte.
pub fn split_street(address: &str) -> Option<(String, String, String)> {
    let words: Vec<&str> = address.split_whitespace().collect();
    // El primer número de 1 o más cifras que no sea la primera palabra
    // ("25 de Mayo 123": el 25 es parte de la calle).
    for (i, w) in words.iter().enumerate().skip(1) {
        let w = w.trim_end_matches(',');
        let digits = w.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 {
            continue;
        }
        // "de" después de un número: es parte del nombre ("9 de Julio").
        if words.get(i + 1).is_some_and(|n| n.eq_ignore_ascii_case("de")) {
            continue;
        }
        let street = words[..i].join(" ").trim_end_matches(',').to_string();
        let rest = words[i + 1..].join(" ").trim_start_matches(',').trim().to_string();
        return Some((street, w.to_string(), rest));
    }
    None
}

/// Limpia el origen y devuelve los problemas de cada campo (vacío si está bien).
pub fn tidy_origin(o: &mut AndreaniOrigin) -> Vec<crate::FieldError> {
    fn clip(s: &str, max: usize) -> String {
        s.trim().chars().take(max).collect()
    }
    o.street = clip(&o.street, 80);
    o.number = clip(&o.number, 10);
    o.floor = clip(&o.floor, 30);
    o.city = clip(&o.city, 60);
    o.province = clip(&o.province, 40);
    o.postal_code = clip(&o.postal_code, 10);
    o.sender = clip(&o.sender, 60);
    let mut e = Vec::new();
    let mut err = |field: &str, message: &str| {
        e.push(crate::FieldError {
            field: field.to_string(),
            message: message.to_string(),
        })
    };
    if o.street.is_empty() {
        err("street", "Escribí la calle desde donde despachás.");
    }
    if o.number.is_empty() {
        err("number", "Escribí el número (o S/N).");
    }
    if o.city.is_empty() {
        err("city", "Escribí la localidad.");
    }
    if !PROVINCES.contains(&o.province.as_str()) {
        err("province", "Elegí la provincia.");
    }
    match normalize_postal_code(&o.postal_code) {
        Some(cp) => o.postal_code = cp,
        None => err("postal_code", "Escribí el código postal de 4 números, por ejemplo 1425."),
    }
    if o.sender.is_empty() {
        err("sender", "Escribí el nombre que sale en la etiqueta.");
    }
    e
}

/// Limpia lo que eligió el dueño: solo servicios que tiene la cuenta, sin
/// repetir. Devuelve los problemas por campo.
pub fn tidy_andreani(
    s: &mut AndreaniSettings,
    available: &[AndreaniService],
) -> Vec<crate::FieldError> {
    let mut services: Vec<_> = s
        .services
        .iter()
        .copied()
        .filter(|x| available.contains(x))
        .collect();
    services.sort();
    services.dedup();
    s.services = services;
    let mut e = tidy_origin(&mut s.origin);
    if s.enabled && s.services.is_empty() {
        e.push(crate::FieldError {
            field: "services".into(),
            message: "Elegí al menos un servicio para ofrecer.".into(),
        });
    }
    if s.parcel.weight_g == 0 || s.parcel.weight_g > PARCEL_MAX_G {
        e.push(crate::FieldError {
            field: "parcel".into(),
            message: "Revisá el peso del paquete.".into(),
        });
    }
    e
}

/// "En camino", "Entregado"…: el estado de un envío como lo informa
/// Andreani, en minúsculas y sin espacios de más.
pub fn tracking_is_delivered(status: &str) -> bool {
    status.trim().eq_ignore_ascii_case("entregado")
}

/// Dónde sigue el comprador su envío.
pub fn tracking_url(tracking: &str) -> String {
    format!("https://www.andreani.com/envio/{tracking}")
}

/// Adónde va el dueño a pagar los envíos de su cuenta PyME.
pub const ANDREANI_PAY_URL: &str = "https://pymes.andreani.com/ver-envios?tab=integraciones";
/// Adónde va el dueño a crear su cuenta.
pub const ANDREANI_SIGNUP_URL: &str = "https://pymes.andreani.com/";
/// Adónde va el dueño a copiar su código.
pub const ANDREANI_CODE_URL: &str = "https://pymes.andreani.com/integraciones/";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kilos_y_centimetros() {
        assert_eq!(parse_kg("0,5"), Some(500));
        assert_eq!(parse_kg("1.25"), Some(1250));
        assert_eq!(parse_kg("2"), Some(2000));
        assert_eq!(parse_kg("1.000,5"), Some(1_000_500).filter(|g| *g <= PARCEL_MAX_G));
        assert_eq!(parse_cm("1.250,5"), None);
        assert_eq!(parse_kg("999,5"), Some(999_500));
        assert_eq!(parse_kg("abc"), None);
        assert_eq!(parse_kg("0"), None);
        assert_eq!(parse_kg("-1"), None);
        assert_eq!(parse_cm("12,5"), Some(125));
        assert_eq!(parse_cm("20"), Some(200));
        assert_eq!(kg_input(500), "0,5");
        assert_eq!(kg_input(1250), "1,25");
        assert_eq!(kg_input(2000), "2");
        assert_eq!(kg_input(5), "0,005");
        assert_eq!(cm_input(125), "12,5");
        assert_eq!(Parcel::default().summary(), "0,5 kg · 20 × 15 × 10 cm");
    }

    #[test]
    fn paquete_todo_o_nada() {
        assert_eq!(parse_parcel("", " ", "", ""), Ok(None));
        assert!(parse_parcel("1", "", "10", "10").is_err());
        assert!(parse_parcel("", "10", "10", "10").is_err());
        assert_eq!(
            parse_parcel("1,5", "30", "20", "10,5"),
            Ok(Some(Parcel {
                weight_g: 1500,
                length_mm: 300,
                width_mm: 200,
                height_mm: 105
            }))
        );
    }

    #[test]
    fn bigger() {
        assert!(!Parcel::default().is_bigger());
        let mut p = Parcel::default();
        p.weight_g = 60_000;
        assert!(p.is_bigger());
        let p = Parcel {
            weight_g: 1000,
            length_mm: 1500,
            width_mm: 1000,
            height_mm: 600,
        };
        assert!(p.is_bigger());
    }

    #[test]
    fn servicios_de_andreani() {
        assert_eq!(AndreaniService::from_api("A domicilio"), Some(AndreaniService::Home));
        assert_eq!(AndreaniService::from_api("estándar"), Some(AndreaniService::Home));
        assert_eq!(AndreaniService::from_api("A sucursal"), Some(AndreaniService::Branch));
        assert_eq!(AndreaniService::from_api("Llega hoy"), Some(AndreaniService::SameDay));
        assert_eq!(AndreaniService::from_api("BIGGER"), Some(AndreaniService::Bigger));
        assert_eq!(AndreaniService::from_api("otro"), None);
        for s in AndreaniService::ALL {
            assert_eq!(AndreaniService::from_str(s.as_str()), Some(s));
        }
    }

    #[test]
    fn codigo_postal() {
        assert_eq!(normalize_postal_code("1425"), Some("1425".into()));
        assert_eq!(normalize_postal_code("c1425abc"), Some("1425".into()));
        assert_eq!(normalize_postal_code(" 5000 "), Some("5000".into()));
        assert_eq!(normalize_postal_code("142"), None);
        assert_eq!(normalize_postal_code("ABCD"), None);
    }

    #[test]
    fn nombre_y_calle() {
        assert_eq!(split_name("Ana María López"), ("Ana María".into(), "López".into()));
        assert_eq!(split_name("Ana"), ("Ana".into(), "-".into()));
        assert_eq!(
            split_street("Av. Corrientes 1234"),
            Some(("Av. Corrientes".into(), "1234".into(), String::new()))
        );
        assert_eq!(
            split_street("Corrientes 1234, piso 3 B"),
            Some(("Corrientes".into(), "1234".into(), "piso 3 B".into()))
        );
        assert_eq!(
            split_street("25 de Mayo 450"),
            Some(("25 de Mayo".into(), "450".into(), String::new()))
        );
        assert_eq!(
            split_street("Calle 9 de Julio 77"),
            Some(("Calle 9 de Julio".into(), "77".into(), String::new()))
        );
        assert_eq!(split_street("Corrientes sin número"), None);
    }

    #[test]
    fn origen() {
        let mut o = AndreaniOrigin {
            street: " Corrientes ".into(),
            number: "1234".into(),
            floor: String::new(),
            city: "CABA".into(),
            province: "Ciudad Autónoma de Buenos Aires".into(),
            postal_code: "C1043AAZ".into(),
            sender: "Mi tienda".into(),
        };
        assert!(tidy_origin(&mut o).is_empty());
        assert_eq!(o.street, "Corrientes");
        assert_eq!(o.postal_code, "1043");
        o.province = "Narnia".into();
        assert_eq!(tidy_origin(&mut o)[0].field, "province");
    }
}
