//! Compras: cómo entrega cada tienda, el formulario de "Finalizar compra" y
//! los pedidos que quedan guardados.

use serde::{Deserialize, Serialize};

/// Cómo entrega sus pedidos una tienda (lo elige el dueño en Ventas).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeliverySettings {
    pub pickup_enabled: bool,
    /// Dónde se retira, por ejemplo "Av. Corrientes 1234, CABA".
    pub pickup_address: String,
    /// Cuándo, por ejemplo "Lun a Vie de 10 a 18 h".
    pub pickup_hours: String,
    pub shipping_enabled: bool,
    /// Costo fijo del envío: 0 es gratis; `None`, se coordina con el comprador.
    pub shipping_cents: Option<i64>,
    /// Lo que se le dice al comprador, por ejemplo "Lo coordinamos por WhatsApp".
    pub shipping_note: String,
    /// Los servicios de Andreani que se ofrecen (vacío: Andreani no está
    /// conectado o apagado). Se completa al cargar "Finalizar compra"; no se
    /// guarda con el resto.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub andreani: Vec<crate::AndreaniService>,
}

impl Default for DeliverySettings {
    fn default() -> Self {
        Self {
            pickup_enabled: false,
            pickup_address: String::new(),
            pickup_hours: String::new(),
            shipping_enabled: true,
            shipping_cents: None,
            shipping_note: "Lo coordinamos por WhatsApp".to_string(),
            andreani: Vec::new(),
        }
    }
}

pub const PICKUP_ADDRESS_MAX: usize = 120;
pub const PICKUP_HOURS_MAX: usize = 80;
pub const SHIPPING_NOTE_MAX: usize = 120;
/// Envío de más de esto es seguro un error de tipeo.
pub const SHIPPING_MAX_CENTS: i64 = 1_000_000_000;

/// Limpia los ajustes de entrega y devuelve los problemas (vacío si están bien).
pub fn tidy_delivery(d: &mut DeliverySettings) -> Vec<String> {
    d.pickup_address = clip(d.pickup_address.trim(), PICKUP_ADDRESS_MAX);
    d.pickup_hours = clip(d.pickup_hours.trim(), PICKUP_HOURS_MAX);
    d.shipping_note = clip(d.shipping_note.trim(), SHIPPING_NOTE_MAX);
    let mut errors = Vec::new();
    if !d.pickup_enabled && !d.shipping_enabled {
        errors.push("Elegí al menos una forma de entrega: retiro o envío.".to_string());
    }
    if d.pickup_enabled && d.pickup_address.is_empty() {
        errors.push("Poné la dirección donde se retiran los pedidos.".to_string());
    }
    if d.shipping_cents
        .is_some_and(|c| !(0..=SHIPPING_MAX_CENTS).contains(&c))
    {
        errors.push("El costo del envío no parece correcto.".to_string());
    }
    errors
}

/// Cómo paga el comprador.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PayMethod {
    /// En Mercado Pago, al confirmar: el pedido entra recién con el pago aprobado.
    #[default]
    MercadoPago,
    /// Transferencia a la cuenta de la tienda; el dueño marca cuando la recibe.
    Transfer,
    /// La tienda le escribe al comprador para coordinar el pago.
    Coordinate,
}

impl PayMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            PayMethod::MercadoPago => "mp",
            PayMethod::Transfer => "transfer",
            PayMethod::Coordinate => "coordinate",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "mp" => PayMethod::MercadoPago,
            "transfer" => PayMethod::Transfer,
            _ => PayMethod::Coordinate,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PayMethod::MercadoPago => "Mercado Pago",
            PayMethod::Transfer => "Transferencia",
            PayMethod::Coordinate => "Coordinar con la tienda",
        }
    }
}

/// Las formas de cobro de una tienda además de Mercado Pago (que depende de
/// tener la cuenta vinculada).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PaymentOptions {
    pub transfer: bool,
    /// Alias, CBU o CVU adonde le transfieren.
    #[serde(default)]
    pub transfer_alias: String,
    /// Titular de la cuenta (opcional).
    #[serde(default)]
    pub transfer_holder: String,
    pub coordinate: bool,
}

impl PaymentOptions {
    /// Adónde transferir, como lo ve el comprador (una línea por dato).
    pub fn transfer_text(&self) -> String {
        if self.transfer_holder.is_empty() {
            self.transfer_alias.clone()
        } else {
            format!("{}\nTitular: {}", self.transfer_alias, self.transfer_holder)
        }
    }
}

pub const TRANSFER_ALIAS_MAX: usize = 300;
pub const TRANSFER_HOLDER_MAX: usize = 80;

/// Limpia las formas de cobro y devuelve el problema para mostrar, si hay.
pub fn tidy_payment_options(p: &mut PaymentOptions) -> Option<String> {
    p.transfer_alias = clip(p.transfer_alias.trim(), TRANSFER_ALIAS_MAX);
    p.transfer_holder = clip(
        &p.transfer_holder
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        TRANSFER_HOLDER_MAX,
    );
    (p.transfer && p.transfer_alias.is_empty())
        .then(|| "Escribí tu alias, CBU o CVU para activar la transferencia.".to_string())
}

/// Las formas de pago que puede elegir el comprador, en orden. Vacío: la
/// tienda todavía no puede vender.
pub fn pay_methods(mp_linked: bool, p: &PaymentOptions) -> Vec<PayMethod> {
    let mut v = Vec::new();
    if mp_linked {
        v.push(PayMethod::MercadoPago);
    }
    if p.transfer {
        v.push(PayMethod::Transfer);
    }
    if p.coordinate {
        v.push(PayMethod::Coordinate);
    }
    v
}

/// Cómo recibe el pedido el comprador.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Delivery {
    Pickup,
    /// El envío propio de la tienda (costo fijo o a coordinar).
    #[default]
    Shipping,
    /// Con Andreani, cotizado al comprar.
    Andreani(crate::AndreaniService),
}

impl Delivery {
    pub fn as_str(self) -> &'static str {
        use crate::AndreaniService as A;
        match self {
            Delivery::Pickup => "pickup",
            Delivery::Shipping => "shipping",
            Delivery::Andreani(A::Home) => "andreani:home",
            Delivery::Andreani(A::Branch) => "andreani:branch",
            Delivery::Andreani(A::SameDay) => "andreani:same_day",
            Delivery::Andreani(A::Bigger) => "andreani:bigger",
        }
    }

    pub fn from_str(s: &str) -> Self {
        if s == "pickup" {
            Delivery::Pickup
        } else if let Some(svc) = s
            .strip_prefix("andreani:")
            .and_then(crate::AndreaniService::from_str)
        {
            Delivery::Andreani(svc)
        } else {
            Delivery::Shipping
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Delivery::Pickup => "Retiro en el local",
            Delivery::Shipping => "Envío a domicilio",
            Delivery::Andreani(s) => s.label(),
        }
    }

    /// Hace falta la dirección del comprador.
    pub fn needs_address(self) -> bool {
        match self {
            Delivery::Pickup => false,
            Delivery::Shipping => true,
            Delivery::Andreani(s) => !s.needs_branch(),
        }
    }

    pub fn andreani(self) -> Option<crate::AndreaniService> {
        match self {
            Delivery::Andreani(s) => Some(s),
            _ => None,
        }
    }
}

/// Lo que completa el comprador en "Finalizar compra".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CheckoutForm {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub delivery: Delivery,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub notes: String,
    /// Producto y cantidad, como están en el carrito.
    pub lines: Vec<(String, u32)>,
    /// Cómo paga (una de las que ofrece la tienda).
    #[serde(default)]
    pub pay: PayMethod,
    /// Con Andreani a sucursal: el código de la sucursal elegida.
    #[serde(default)]
    pub branch: String,
    /// Con Andreani: el DNI de quien recibe (lo pide Andreani).
    #[serde(default)]
    pub dni: String,
}

/// Un campo del formulario con problema, para mostrarlo al lado del campo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FieldError {
    /// `name`, `email`, `phone`, `delivery`, `address`, `postal_code`, `city` o `cart`.
    pub field: String,
    pub message: String,
}

pub const NAME_MAX: usize = 80;
pub const EMAIL_MAX: usize = 120;
pub const ADDRESS_MAX: usize = 120;
pub const CITY_MAX: usize = 60;
pub const NOTES_MAX: usize = 300;
/// Más renglones que esto en un pedido es seguro un error (o un abuso).
pub const MAX_ORDER_LINES: usize = 50;

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn err(field: &str, message: &str) -> FieldError {
    FieldError {
        field: field.to_string(),
        message: message.to_string(),
    }
}

/// ¿Parece un email? (algo@algo.algo, sin espacios)
pub fn looks_like_email(s: &str) -> bool {
    let Some((user, host)) = s.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && !host.contains('@')
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && !s.chars().any(char::is_whitespace)
}

/// Los dígitos de un teléfono: acepta espacios, guiones, paréntesis y +.
pub fn phone_digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Limpia el formulario y devuelve los problemas de cada campo (vacío si
/// está todo bien). Se usa igual en el navegador y en el servidor.
pub fn tidy_checkout(f: &mut CheckoutForm, d: &DeliverySettings) -> Vec<FieldError> {
    f.name = clip(
        &f.name.split_whitespace().collect::<Vec<_>>().join(" "),
        NAME_MAX,
    );
    f.email = clip(f.email.trim(), EMAIL_MAX).to_lowercase();
    f.phone = clip(f.phone.trim(), 30);
    f.address = clip(f.address.trim(), ADDRESS_MAX);
    f.postal_code = clip(f.postal_code.trim(), 10).to_uppercase();
    f.city = clip(f.city.trim(), CITY_MAX);
    f.notes = clip(f.notes.trim(), NOTES_MAX);
    f.lines.retain(|(id, q)| !id.is_empty() && *q > 0);

    let mut e = Vec::new();
    if f.name.chars().count() < 2 {
        e.push(err("name", "Escribí tu nombre y apellido."));
    }
    if f.email.is_empty() {
        e.push(err(
            "email",
            "Escribí tu email: ahí te llega el comprobante.",
        ));
    } else if !looks_like_email(&f.email) {
        e.push(err(
            "email",
            "Revisá el email, por ejemplo: nombre@correo.com",
        ));
    }
    let digits = phone_digits(&f.phone).len();
    if f.phone.is_empty() {
        e.push(err(
            "phone",
            "Escribí un teléfono para coordinar la entrega.",
        ));
    } else if digits == 0 {
        e.push(err(
            "phone",
            "Escribí solo números, por ejemplo 11 5555-1234.",
        ));
    } else if !(8..=15).contains(&digits) {
        e.push(err(
            "phone",
            "Revisá el teléfono, con característica: por ejemplo 11 5555-1234.",
        ));
    }
    let allowed = match f.delivery {
        Delivery::Pickup => d.pickup_enabled,
        Delivery::Shipping => d.shipping_enabled,
        Delivery::Andreani(s) => d.andreani.contains(&s),
    };
    if !allowed {
        e.push(err("delivery", "Elegí cómo querés recibir tu pedido."));
    }
    if f.delivery.needs_address() {
        if f.address.is_empty() {
            e.push(err("address", "Escribí la calle y el número."));
        } else if f.delivery.andreani().is_some() && crate::split_street(&f.address).is_none() {
            e.push(err(
                "address",
                "Escribí la calle y el número, por ejemplo Av. Corrientes 1234.",
            ));
        }
        if f.city.is_empty() {
            e.push(err("city", "Escribí la ciudad o localidad."));
        }
    } else {
        f.address.clear();
        f.city.clear();
    }
    match f.delivery {
        Delivery::Andreani(svc) => {
            match crate::normalize_postal_code(&f.postal_code) {
                Some(cp) => f.postal_code = cp,
                None if f.postal_code.is_empty() => {
                    e.push(err("postal_code", "Escribí tu código postal, por ejemplo 1425."))
                }
                None => e.push(err(
                    "postal_code",
                    "Revisá el código postal: son 4 números, por ejemplo 1425.",
                )),
            }
            f.dni = f.dni.chars().filter(char::is_ascii_digit).collect();
            if !(7..=8).contains(&f.dni.len()) {
                e.push(err("dni", "Escribí tu DNI, solo los números."));
            }
            f.branch = clip(f.branch.trim(), 20);
            if svc.needs_branch() {
                if f.branch.is_empty() {
                    e.push(err("branch", "Elegí la sucursal donde lo retirás."));
                }
            } else {
                f.branch.clear();
            }
        }
        Delivery::Shipping => {
            // El código postal ya no se pide; si llega, tiene que tener forma de uno.
            let pc_ok = (4..=8).contains(&f.postal_code.len())
                && f.postal_code.chars().all(|c| c.is_ascii_alphanumeric());
            if !f.postal_code.is_empty() && !pc_ok {
                e.push(err(
                    "postal_code",
                    "Revisá el código postal, por ejemplo 1123.",
                ));
            }
            f.dni.clear();
            f.branch.clear();
        }
        Delivery::Pickup => {
            f.postal_code.clear();
            f.dni.clear();
            f.branch.clear();
        }
    }
    if f.lines.is_empty() {
        e.push(err("cart", "Tu carrito está vacío."));
    } else if f.lines.len() > MAX_ORDER_LINES {
        e.push(err(
            "cart",
            "Hay demasiados productos distintos en el carrito.",
        ));
    }
    e
}

/// Costo del envío para esta forma de entrega (lo que se suma al total; si
/// se coordina después, 0).
/// Con Andreani, la cotización va aparte (ver [`crate::AndreaniQuote`]).
pub fn shipping_for(delivery: Delivery, d: &DeliverySettings) -> i64 {
    match delivery {
        Delivery::Pickup | Delivery::Andreani(_) => 0,
        Delivery::Shipping => d.shipping_cents.unwrap_or(0),
    }
}

/// Cómo se muestra el costo del envío: "$ 4.500,00", "Gratis" o "A coordinar".
/// La dirección de envío en una línea: "calle, localidad" y el código
/// postal entre paréntesis si lo hay (los pedidos viejos lo tienen).
pub fn ship_address(address: &str, city: &str, postal_code: &str) -> String {
    let mut s = [address, city]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if !postal_code.is_empty() {
        s.push_str(&format!(" ({postal_code})"));
    }
    s
}

pub fn shipping_label(d: &DeliverySettings, format_price: impl Fn(i64) -> String) -> String {
    match d.shipping_cents {
        None => "A coordinar".to_string(),
        Some(0) => "Gratis".to_string(),
        Some(c) => format_price(c),
    }
}

/// Lo que necesita la página "Finalizar compra" además de los productos.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CheckoutInfo {
    pub store_name: String,
    pub delivery: DeliverySettings,
    /// Las formas de pago que ofrece la tienda (vacío: todavía no vende).
    #[serde(default)]
    pub pay_methods: Vec<PayMethod>,
}

/// Un producto dentro de un pedido, como estaba al comprarlo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OrderItem {
    pub product_id: String,
    pub name: String,
    pub image_url: String,
    pub unit_cents: i64,
    pub qty: u32,
}

impl OrderItem {
    pub fn total_cents(&self) -> i64 {
        self.unit_cents * i64::from(self.qty)
    }
}

/// En qué está un pedido.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    /// Recibido, el pago se coordina con la tienda.
    New,
    PendingPayment,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrderStatus::New => "new",
            OrderStatus::PendingPayment => "pending_payment",
            OrderStatus::Paid => "paid",
            OrderStatus::Shipped => "shipped",
            OrderStatus::Delivered => "delivered",
            OrderStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending_payment" => OrderStatus::PendingPayment,
            "paid" => OrderStatus::Paid,
            "shipped" => OrderStatus::Shipped,
            "delivered" => OrderStatus::Delivered,
            "cancelled" => OrderStatus::Cancelled,
            _ => OrderStatus::New,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OrderStatus::New => "Esperando pago",
            OrderStatus::PendingPayment => "Esperando pago",
            OrderStatus::Paid => "Pagado",
            OrderStatus::Shipped => "Enviado",
            OrderStatus::Delivered => "Entregado",
            OrderStatus::Cancelled => "Cancelado",
        }
    }
}

/// Un pedido, para mostrarlo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Order {
    pub id: String,
    pub number: i32,
    pub status: OrderStatus,
    pub customer_name: String,
    pub email: String,
    pub phone: String,
    pub delivery: Delivery,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub notes: String,
    pub items: Vec<OrderItem>,
    pub subtotal_cents: i64,
    pub shipping_cents: i64,
    pub total_cents: i64,
    /// Cuándo se hizo, RFC 3339.
    pub created_at: String,
    #[serde(default)]
    pub pay: PayMethod,
    /// Código del pedido de arrepentimiento, si el comprador lo pidió.
    #[serde(default)]
    pub regret_code: String,
    /// El envío con Andreani, si va por Andreani.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipment: Option<crate::Shipment>,
}

impl Order {
    /// Adónde va el pedido, en una línea: la dirección, o la sucursal de
    /// Andreani. `None` si lo retira en el local.
    pub fn destination(&self) -> Option<String> {
        match (self.delivery, &self.shipment) {
            (Delivery::Pickup, _) => None,
            (Delivery::Andreani(s), Some(sh)) if s.needs_branch() => Some(format!(
                "Sucursal {} · {}",
                sh.branch_name, sh.branch_address
            )),
            _ => Some(ship_address(&self.address, &self.city, &self.postal_code)),
        }
    }

    /// El costo del envío como se muestra: "$ 4.500,00", "Gratis" o "Gratis o a coordinar".
    pub fn shipping_text(&self, format_price: impl Fn(i64) -> String) -> String {
        match self.delivery {
            Delivery::Pickup => "Gratis".to_string(),
            Delivery::Andreani(_) => format_price(self.shipping_cents),
            Delivery::Shipping if self.shipping_cents == 0 => "Gratis o a coordinar".to_string(),
            Delivery::Shipping => format_price(self.shipping_cents),
        }
    }
}

/// La página de un pedido ya hecho, en la tienda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OrderReceipt {
    pub store_name: String,
    pub order: Order,
    pub delivery: DeliverySettings,
    /// Se puede pagar con Mercado Pago (la tienda lo vinculó y falta pagar).
    #[serde(default)]
    pub can_pay: bool,
    /// Adónde transferir, si el pedido se paga con transferencia.
    #[serde(default)]
    pub transfer_info: String,
}

/// Qué pasó al tocar "Confirmar pedido".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlaceOrder {
    /// Listo: el pedido quedó guardado.
    Placed { id: String, number: i32 },
    /// Guardado: falta pagar en Mercado Pago, en `url`.
    Pay { id: String, url: String },
    /// Hay campos para corregir.
    Invalid { errors: Vec<FieldError> },
    /// Algo del carrito cambió (un producto se agotó o ya no está).
    CartChanged { message: String },
}

/// La pantalla Cobros de una tienda, en el panel.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PaymentsInfo {
    pub store_name: String,
    /// MarketMaker tiene su aplicación de Mercado Pago configurada.
    pub configured: bool,
    pub link: Option<MpLink>,
    #[serde(default)]
    pub options: PaymentOptions,
}

/// La cuenta de Mercado Pago vinculada a una tienda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MpLink {
    pub user_id: i64,
    /// Cuándo se vinculó, RFC 3339.
    pub linked_at: String,
    /// `false`: cuenta de prueba (los pagos no son de verdad).
    pub live: bool,
}

/// Las columnas del tablero de Pedidos. Los cancelados van aparte.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderColumn {
    AwaitingPayment,
    ToPrepare,
    Shipped,
    Delivered,
}

impl OrderColumn {
    pub const ALL: [OrderColumn; 4] = [
        OrderColumn::AwaitingPayment,
        OrderColumn::ToPrepare,
        OrderColumn::Shipped,
        OrderColumn::Delivered,
    ];

    /// En qué columna va un pedido. `None` si está cancelado.
    pub fn of(status: OrderStatus) -> Option<Self> {
        match status {
            // Sin pago online, el pedido nuevo espera que la tienda cobre.
            OrderStatus::New | OrderStatus::PendingPayment => Some(OrderColumn::AwaitingPayment),
            OrderStatus::Paid => Some(OrderColumn::ToPrepare),
            OrderStatus::Shipped => Some(OrderColumn::Shipped),
            OrderStatus::Delivered => Some(OrderColumn::Delivered),
            OrderStatus::Cancelled => None,
        }
    }

    /// El estado que toma un pedido al llevarlo a esta columna.
    pub fn status(self) -> OrderStatus {
        match self {
            OrderColumn::AwaitingPayment => OrderStatus::New,
            OrderColumn::ToPrepare => OrderStatus::Paid,
            OrderColumn::Shipped => OrderStatus::Shipped,
            OrderColumn::Delivered => OrderStatus::Delivered,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            OrderColumn::AwaitingPayment => "Esperando pago",
            OrderColumn::ToPrepare => "Para preparar",
            OrderColumn::Shipped => "Enviados",
            OrderColumn::Delivered => "Entregados",
        }
    }

    /// La columna que sigue, para el botón principal del detalle.
    pub fn next(self) -> Option<Self> {
        match self {
            OrderColumn::AwaitingPayment => Some(OrderColumn::ToPrepare),
            OrderColumn::ToPrepare => Some(OrderColumn::Shipped),
            OrderColumn::Shipped => Some(OrderColumn::Delivered),
            OrderColumn::Delivered => None,
        }
    }

    /// Texto del botón que lleva un pedido a esta columna.
    pub fn action(self, delivery: Delivery) -> &'static str {
        match (self, delivery) {
            (OrderColumn::AwaitingPayment, _) => "Volver a esperando pago",
            (OrderColumn::ToPrepare, _) => "Ya me pagó",
            (OrderColumn::Shipped, Delivery::Pickup) => "Marcar listo para retirar",
            (OrderColumn::Shipped, Delivery::Shipping | Delivery::Andreani(_)) => "Marcar como enviado",
            (OrderColumn::Delivered, _) => "Marcar como entregado",
        }
    }
}

/// Fecha y hora de un pedido en hora de Argentina (UTC−3, sin horario de
/// verano), por ejemplo "15/11 14:32". `created_at` viene como
/// `2026-11-15T17:32:00Z`.
pub fn order_time_ar(created_at: &str) -> String {
    let num = |a: usize, b: usize| created_at.get(a..b).and_then(|t| t.parse::<i64>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi)) =
        (num(0, 4), num(5, 7), num(8, 10), num(11, 13), num(14, 16))
    else {
        return String::new();
    };
    // Minutos desde 1970, menos 3 horas, y de vuelta a fecha.
    let days = days_from_civil(y, mo, d);
    let total = days * 1440 + h * 60 + mi - 180;
    let (days, mins) = (total.div_euclid(1440), total.rem_euclid(1440));
    let (_, mo, d) = civil_from_days(days);
    format!("{d:02}/{mo:02} {:02}:{:02}", mins / 60, mins % 60)
}

// Algoritmos de Howard Hinnant para pasar de fecha a días y al revés.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> CheckoutForm {
        CheckoutForm {
            name: "  Lucía   Fernández ".into(),
            email: " Lucia@Correo.com ".into(),
            phone: "(11) 5555-1234".into(),
            delivery: Delivery::Shipping,
            address: "Av. Santa Fe 2050".into(),
            postal_code: "c1123aaa".into(),
            city: "CABA".into(),
            notes: String::new(),
            lines: vec![("a".into(), 2), ("b".into(), 0)],
            pay: PayMethod::default(),
            branch: String::new(),
            dni: String::new(),
        }
    }

    #[test]
    fn andreani_needs_postal_code_dni_and_branch() {
        use crate::AndreaniService as A;
        let d = DeliverySettings {
            andreani: vec![A::Home, A::Branch],
            ..DeliverySettings::default()
        };
        let mut f = good();
        f.delivery = Delivery::Andreani(A::Home);
        f.dni = "30.123.456".into();
        assert!(tidy_checkout(&mut f, &d).is_empty());
        assert_eq!(f.postal_code, "1123");
        assert_eq!(f.dni, "30123456");

        let mut f = good();
        f.delivery = Delivery::Andreani(A::Branch);
        f.postal_code = String::new();
        let fields: Vec<_> = tidy_checkout(&mut f, &d).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["postal_code", "dni", "branch"]);
        assert!(f.address.is_empty());

        let mut f = good();
        f.delivery = Delivery::Andreani(A::Home);
        f.address = "Corrientes".into();
        f.dni = "30123456".into();
        let fields: Vec<_> = tidy_checkout(&mut f, &d).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["address"]);

        let mut f = good();
        f.delivery = Delivery::Andreani(A::SameDay);
        assert_eq!(tidy_checkout(&mut f, &d)[0].field, "delivery");

        for x in [Delivery::Pickup, Delivery::Shipping, Delivery::Andreani(A::Branch)] {
            assert_eq!(Delivery::from_str(x.as_str()), x);
        }
    }

    #[test]
    fn tidies_a_good_form() {
        let mut f = good();
        assert!(tidy_checkout(&mut f, &DeliverySettings::default()).is_empty());
        assert_eq!(f.name, "Lucía Fernández");
        assert_eq!(f.email, "lucia@correo.com");
        assert_eq!(f.postal_code, "C1123AAA");
        assert_eq!(f.lines, vec![("a".to_string(), 2)]);
    }

    #[test]
    fn points_at_each_bad_field() {
        let mut f = CheckoutForm {
            email: "lucia@correo".into(),
            phone: "123".into(),
            ..CheckoutForm::default()
        };
        let fields: Vec<_> = tidy_checkout(&mut f, &DeliverySettings::default())
            .into_iter()
            .map(|e| e.field)
            .collect();
        assert_eq!(
            fields,
            ["name", "email", "phone", "address", "city", "cart"]
        );
    }

    #[test]
    fn pickup_needs_no_address_but_must_be_offered() {
        let mut f = good();
        f.delivery = Delivery::Pickup;
        let d = DeliverySettings::default(); // sin retiro
        let errors = tidy_checkout(&mut f, &d);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "delivery");
        assert!(f.address.is_empty());
        let d = DeliverySettings {
            pickup_enabled: true,
            pickup_address: "Corrientes 1234".into(),
            ..DeliverySettings::default()
        };
        assert!(tidy_checkout(&mut f, &d).is_empty());
        assert_eq!(shipping_for(Delivery::Pickup, &d), 0);
    }

    #[test]
    fn delivery_settings_need_one_option() {
        let mut d = DeliverySettings {
            shipping_enabled: false,
            ..DeliverySettings::default()
        };
        assert_eq!(tidy_delivery(&mut d).len(), 1);
        d.pickup_enabled = true;
        assert_eq!(tidy_delivery(&mut d).len(), 1); // falta la dirección
        d.pickup_address = " Corrientes 1234 ".into();
        assert!(tidy_delivery(&mut d).is_empty());
        assert_eq!(d.pickup_address, "Corrientes 1234");
    }

    #[test]
    fn emails() {
        assert!(looks_like_email("a@b.co"));
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("a b@c.com"));
        assert!(!looks_like_email("@c.com"));
        assert!(!looks_like_email("a@.com"));
    }

    #[test]
    fn columns_and_times() {
        assert_eq!(
            OrderColumn::of(OrderStatus::New),
            Some(OrderColumn::AwaitingPayment)
        );
        assert_eq!(OrderColumn::of(OrderStatus::Cancelled), None);
        for c in OrderColumn::ALL {
            assert_eq!(OrderColumn::of(c.status()), Some(c));
        }
        assert_eq!(order_time_ar("2026-11-15T17:32:05Z"), "15/11 14:32");
        assert_eq!(order_time_ar("2026-03-01T01:10:00Z"), "28/02 22:10");
        assert_eq!(order_time_ar("2028-03-01T02:00:00Z"), "29/02 23:00");
        assert_eq!(order_time_ar("2027-01-01T00:00:00Z"), "31/12 21:00");
        assert_eq!(order_time_ar("roto"), "");
    }
}

/// Botón de arrepentimiento: lo que escribe el comprador.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct RegretForm {
    pub name: String,
    pub email: String,
    /// El número del pedido, como lo escribió ("#1001" o "1001").
    pub number: String,
    pub reason: String,
}

/// Qué pasó al mandar el pedido de arrepentimiento.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RegretOutcome {
    /// Quedó registrado; `code` es el código para el comprador.
    Sent { code: String, number: i32 },
    Invalid { errors: Vec<FieldError> },
    /// Mensaje general (por ejemplo, demasiados intentos).
    Problem(String),
}

/// Limpia el formulario de arrepentimiento y devuelve el número del pedido o
/// los problemas por campo (`name`, `email`, `number`).
pub fn tidy_regret(f: &mut RegretForm) -> Result<i32, Vec<FieldError>> {
    f.name = f.name.split_whitespace().collect::<Vec<_>>().join(" ");
    f.name = f.name.chars().take(80).collect();
    f.email = f.email.trim().to_lowercase().chars().take(120).collect();
    f.reason = f.reason.trim().chars().take(500).collect();
    let digits: String = f.number.chars().filter(char::is_ascii_digit).collect();
    let mut e = Vec::new();
    let err = |field: &str, message: &str| FieldError {
        field: field.into(),
        message: message.into(),
    };
    if f.name.is_empty() {
        e.push(err("name", "Escribí tu nombre."));
    }
    if !looks_like_email(&f.email) {
        e.push(err(
            "email",
            "Escribí el email con el que hiciste la compra (por ejemplo, ana@correo.com).",
        ));
    }
    let number = digits.parse::<i32>().ok().filter(|n| *n > 0);
    if number.is_none() {
        e.push(err(
            "number",
            "Escribí el número de pedido que te llegó por email, por ejemplo 1001.",
        ));
    }
    match number {
        Some(n) if e.is_empty() => Ok(n),
        _ => Err(e),
    }
}

#[cfg(test)]
mod regret_tests {
    use super::*;

    #[test]
    fn the_regret_form_takes_the_number_with_or_without_hash() {
        let mut f = RegretForm {
            name: "  Ana   Pérez ".into(),
            email: " Ana@Correo.com ".into(),
            number: "#1001".into(),
            reason: String::new(),
        };
        assert_eq!(tidy_regret(&mut f), Ok(1001));
        assert_eq!((f.name.as_str(), f.email.as_str()), ("Ana Pérez", "ana@correo.com"));
        let mut bad = RegretForm::default();
        let fields: Vec<_> = tidy_regret(&mut bad).unwrap_err().into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["name", "email", "number"]);
    }
}
