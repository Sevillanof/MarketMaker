//! Términos y condiciones (`/terminos`) y política de privacidad
//! (`/privacidad`) de MarketMaker. Borrador en lenguaje simple: lo tiene que
//! revisar un abogado antes de darlo por final.

use leptos::prelude::*;

const UPDATED: &str = "Última actualización: 16 de noviembre de 2026";

#[component]
fn LegalShell(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <leptos_meta::Title text=format!("{title} · MarketMaker")/>
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg"><crate::account::Logo/></a>
            </header>
            <article class="mx-auto flex max-w-[68ch] flex-col gap-4 px-4 py-10 leading-relaxed [&_h2]:mt-4 [&_h2]:text-xl [&_h2]:font-bold [&_ul]:list-disc [&_ul]:pl-6">
                <h1 class="text-3xl font-extrabold tracking-tight">{title}</h1>
                <p class="text-sm text-base-content/70">{UPDATED}</p>
                {children()}
                <p class="mt-6 text-sm text-base-content/70">
                    <a class="link" href="/terminos">"Términos y condiciones"</a>" · "
                    <a class="link" href="/privacidad">"Política de privacidad"</a>" · "
                    <a class="link" href="mailto:hola@mkr.com.ar">"hola@mkr.com.ar"</a>
                </p>
            </article>
        </div>
    }
}

/// `/terminos` en la plataforma.
#[component]
pub fn TermsPage() -> impl IntoView {
    if crate::site::current_site() != domain::Site::Platform {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    view! {
        <LegalShell title="Términos y condiciones">
            <p>"Estos términos explican cómo funciona MarketMaker (mkr.com.ar) y qué esperamos de quienes lo usan. Al crear una cuenta, los aceptás."</p>

            <h2>"1. Qué es MarketMaker"</h2>
            <p>"MarketMaker es una herramienta para armar y administrar una tienda online propia: diseñar sus páginas, cargar productos, recibir pedidos y cobrarlos. Cada tienda tiene su dirección (por ejemplo, tutienda.mkr.com.ar)."</p>
            <p>"MarketMaker no vende los productos de las tiendas ni es parte de las compras. Cada tienda es la vendedora y responde ante sus compradores por lo que ofrece, sus precios, entregas, cambios y devoluciones."</p>

            <h2>"2. Tu cuenta"</h2>
            <ul>
                <li>"Por ahora las cuentas son solo con invitación."</li>
                <li>"Tenés que ser mayor de 18 años y dar datos verdaderos."</li>
                <li>"Cuidá tu contraseña. Lo que se haga desde tu cuenta es tu responsabilidad."</li>
                <li>"Podés pedir que borremos tu cuenta escribiendo a hola@mkr.com.ar."</li>
            </ul>

            <h2>"3. Tu tienda y lo que publicás"</h2>
            <ul>
                <li>"Lo que subís (textos, fotos, logos, productos) sigue siendo tuyo. Nos das permiso para guardarlo y mostrarlo en tu tienda, que es para lo que existe."</li>
                <li>"Tenés que tener derecho a usar lo que publicás."</li>
                <li>"No se pueden vender productos prohibidos por la ley argentina, ni publicar contenido engañoso, ofensivo o que viole derechos de otros."</li>
                <li>"Como vendedor, cumplís la Ley de Defensa del Consumidor (24.240): precios claros, información de los productos, el botón de arrepentimiento (cada tienda lo tiene al pie) y la garantía que corresponda."</li>
                <li>"También te ocupás de tus obligaciones impositivas, como emitir las facturas de tus ventas."</li>
            </ul>

            <h2>"4. Cobros"</h2>
            <p>"Los pagos con Mercado Pago van directo a la cuenta de Mercado Pago de la tienda, con sus condiciones y comisiones. Las transferencias y los pagos coordinados los recibe la tienda. MarketMaker no guarda ni maneja el dinero de las ventas."</p>
            <p>"Un pedido entra a la tienda cuando está pagado o, si se paga por transferencia o coordinando, cuando la tienda lo marca como pagado. Los pedidos sin pagar se cancelan a las 48 horas."</p>

            <h2>"5. Lo que podemos hacer"</h2>
            <ul>
                <li>"Pausar o cerrar una tienda o una cuenta que no cumpla estos términos o la ley, avisando cuando sea posible."</li>
                <li>"Cambiar o mejorar MarketMaker. Si un cambio te afecta, te avisamos antes."</li>
                <li>"Cambiar estos términos. Te avisamos por email con al menos 15 días de anticipación."</li>
            </ul>

            <h2>"6. Responsabilidad"</h2>
            <p>"Hacemos lo posible para que MarketMaker funcione bien y sin cortes, y para cuidar tus datos. Aun así, puede haber fallas. No respondemos por las ventas entre las tiendas y sus compradores, ni por daños indirectos o ganancias perdidas, en la medida en que la ley lo permita."</p>

            <h2>"7. Ley y contacto"</h2>
            <p>"Estos términos se rigen por las leyes de la República Argentina. Para cualquier consulta o reclamo, escribinos a hola@mkr.com.ar."</p>
        </LegalShell>
    }
    .into_any()
}

/// `/privacidad` en la plataforma.
#[component]
pub fn PrivacyPage() -> impl IntoView {
    if crate::site::current_site() != domain::Site::Platform {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    view! {
        <LegalShell title="Política de privacidad">
            <p>"Acá contamos qué datos guarda MarketMaker, para qué, con quién los compartimos y qué podés hacer con ellos, según la Ley 25.326 de Protección de los Datos Personales."</p>

            <h2>"1. Qué datos guardamos"</h2>
            <ul>
                <li><b>"De quienes tienen una tienda: "</b>"nombre, email, contraseña (cifrada, nunca la vemos), foto de perfil, los datos de la tienda (nombre, logo, contacto, datos para transferencias) y lo que suben: fotos, textos y productos."</li>
                <li><b>"De quienes compran: "</b>"nombre, email, teléfono, la dirección de entrega si eligen envío, lo que compraron y cómo pagaron."</li>
                <li><b>"De todos: "</b>"datos técnicos necesarios para que el sitio funcione y sea seguro, como la dirección IP y una cookie de sesión. No usamos cookies de publicidad ni de seguimiento."</li>
            </ul>

            <h2>"2. Para qué los usamos"</h2>
            <ul>
                <li>"Para que funcionen las cuentas, las tiendas y las compras."</li>
                <li>"Para mandar los emails necesarios: códigos para entrar, confirmaciones de pedidos y avisos de ventas."</li>
                <li>"Para frenar abusos (por ejemplo, intentos de entrar en serie o pedidos falsos)."</li>
            </ul>
            <p>"No vendemos datos personales ni los usamos para publicidad."</p>

            <h2>"3. Quién más los ve"</h2>
            <ul>
                <li>"La tienda donde comprás recibe tus datos de compra para preparar y entregar tu pedido. Es responsable de usarlos solo para eso."</li>
                <li>"Usamos estos servicios para funcionar: Contabo (servidor), Cloudflare (dominio y almacenamiento de fotos), Resend (envío de emails) y Mercado Pago (pagos, con sus propias políticas). Algunos están fuera de Argentina; solo reciben lo necesario para su tarea."</li>
                <li>"Una autoridad, si la ley nos obliga."</li>
            </ul>

            <h2>"4. Cuánto tiempo"</h2>
            <p>"Guardamos los datos mientras la cuenta o la tienda existan. Los intentos de compra con Mercado Pago que no se pagan se borran a las 48 horas. Si borrás tu cuenta, borramos tus datos, salvo lo que la ley nos obligue a guardar."</p>

            <h2>"5. Tus derechos"</h2>
            <p>"Podés pedir ver, corregir, actualizar o borrar tus datos escribiendo a hola@mkr.com.ar. Te respondemos dentro de los 10 días corridos (acceso) o 5 días hábiles (corrección o borrado), como indica la ley. Tenés derecho a pedir el acceso en forma gratuita cada seis meses, salvo que acredites un interés legítimo para hacerlo antes (art. 14, inc. 3, Ley 25.326)."</p>
            <p class="rounded-box bg-base-100 p-4 text-sm">"La Agencia de Acceso a la Información Pública, en su carácter de Órgano de Control de la Ley N° 25.326, tiene la atribución de atender las denuncias y reclamos que interpongan quienes resulten afectados en sus derechos por incumplimiento de las normas vigentes en materia de protección de datos personales."</p>

            <h2>"6. Seguridad"</h2>
            <p>"Todo viaja cifrado (HTTPS), las contraseñas se guardan cifradas y el acceso al servidor está restringido. Si pasara algo que afecte tus datos, te avisamos."</p>

            <h2>"7. Cambios"</h2>
            <p>"Si cambiamos esta política, te avisamos por email antes de que el cambio rija."</p>
        </LegalShell>
    }
    .into_any()
}
