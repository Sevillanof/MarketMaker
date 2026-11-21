//! Pantallas de la cuenta del dueño: Entrar (`/entrar`), "Mi cuenta"
//! (`/cuenta`) y la barra "Editar mi tienda" que el dueño ve arriba de su
//! tienda.

use crate::auth::{
    change_password, load_account, request_password_link, save_avatar, save_name,
    set_password_with_code, sign_in, sign_out,
};
use domain::{CodeRequest, Me, PasswordChange, PasswordSignIn, SetPassword};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

/// Va a `path` recargando la página, para que el servidor vea la sesión nueva.
fn go(path: &str) {
    let _ = window().location().set_href(path);
}

/// Pone el foco en el campo marcado con `data-autofocus` del paso que se
/// acaba de dibujar.
fn focus_first() {
    request_animation_frame(|| {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = document()
            .query_selector("[data-autofocus]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    });
}

const CONNECTION: &str =
    "No pudimos comunicarnos con el servidor. Revisá tu conexión y probá de nuevo.";
const NOT_ALLOWED: &str = "Por ahora MarketMaker es solo por invitación y este email no está en la lista. Si creés que es un error, escribinos a hola@mkr.com.ar.";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Step {
    /// Email y contraseña juntos.
    SignIn,
    /// Nombre, email y contraseña (solo invitados).
    Create,
    /// Le mandamos el enlace para elegir contraseña; `new_account` si es para
    /// confirmar una cuenta del registro abierto.
    LinkSent { new_account: bool },
    /// Llegó desde el enlace del email: elige la contraseña (y el nombre, si
    /// la cuenta es nueva).
    NewPassword { new_account: bool },
}

/// El enlace del email trae `recuperar` (el email) y `codigo`.
#[derive(Clone, PartialEq, Debug)]
struct Link {
    email: String,
    code: String,
    new_account: bool,
}

/// Aviso chico debajo de un campo: rojo con un círculo si algo salió mal,
/// verde con un tilde si salió bien. Sin recuadro, para que no grite.
#[component]
pub(crate) fn FieldNote(ok: bool, #[prop(into)] text: String) -> impl IntoView {
    let (role, color, icon) = if ok {
        ("status", "text-green-700", "M5 12.5l4 4 10-10")
    } else {
        ("alert", "text-red-700", "M12 8v5m0 3.5v.01")
    };
    view! {
        <p role=role class=format!("-mt-1 flex items-start gap-1.5 text-sm leading-snug {color}")>
            <svg class="mt-px size-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                {(!ok).then(|| view! { <circle cx="12" cy="12" r="9"/> })}
                <path d=icon/>
            </svg>
            <span>{text}</span>
        </p>
    }
}

/// El logo de la plataforma.
#[component]
pub fn Logo(#[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <span class=format!("inline-flex items-center gap-2.5 font-extrabold tracking-tight {class}")>
            <span class="grid size-8 place-items-center rounded-[9px] bg-gradient-to-br from-[#4330d6] to-[#6a55f0] text-[15px] text-white" aria-hidden="true">"m"</span>
            "MarketMaker"
        </span>
    }
}

/// `/entrar`: una tarjeta sobre el fondo oscuro de la plataforma, con email y
/// contraseña en la misma pantalla (así el navegador completa los dos). Crear
/// cuenta (solo invitados) y elegir una contraseña nueva (con un enlace por
/// email) son pasos aparte.
#[component]
pub fn SignInPage() -> impl IntoView {
    let query = use_query_map();
    // Sin `volver`: en una tienda, al editor; en la plataforma, a tus tiendas.
    let next = move || match query.read().get("volver") {
        Some(v) => domain::safe_next(&v),
        None if crate::site::current_site() == domain::Site::Platform => "/".to_string(),
        None => "/editor".to_string(),
    };
    // `/entrar?crear`: los botones "Crear cuenta" abren directo ese paso.
    let create = query.read_untracked().get("crear").is_some();
    // `/entrar?recuperar=…&codigo=…`: viene del enlace del email.
    let link = query.with_untracked(|q| {
        Some(Link {
            email: q.get("recuperar")?,
            code: q.get("codigo")?,
            new_account: q.get("nuevo").is_some(),
        })
    });
    let from_link = link.is_some();
    let me = Resource::new(|| (), |_| crate::auth::who_am_i());
    view! {
        <leptos_meta::Title text="Entrar · MarketMaker"/>
        <div class="relative isolate flex min-h-dvh flex-col overflow-x-hidden bg-[#110e1c] text-white">
            <div class="pointer-events-none absolute inset-x-0 -top-72 -z-10 h-[800px] bg-[radial-gradient(closest-side,rgba(106,85,240,.45),transparent)]" aria-hidden="true"></div>
            <header class="flex items-center justify-between px-5 py-5 sm:px-10 sm:py-6">
                <a href="/" class="text-lg"><Logo/></a>
                <a href="/" class="text-sm text-white/65 hover:text-white">"← Volver al inicio"</a>
            </header>
            <main class="flex flex-1 flex-col items-center px-4 pt-2 pb-12 sm:justify-center sm:pt-6">
                <div data-theme="light" class="w-full max-w-[420px] rounded-[20px] bg-base-100 px-6 pt-7 pb-6 text-base-content shadow-[0_30px_80px_-20px_rgba(0,0,0,.6)] sm:px-9 sm:pt-9 sm:pb-7">
                    <Suspense fallback=|| view! { <ui::Skeleton class="h-80 w-full"/> }>
                        {move || {
                            let link = link.clone();
                            Suspend::new(async move {
                            match me.await {
                                Ok(Some(me)) if !from_link => view! { <AlreadyIn me next=next()/> }.into_any(),
                                _ => view! { <SignInForm next=next() create link/> }.into_any(),
                            }
                        })}}
                    </Suspense>
                </div>
                <p class="mt-6 max-w-[480px] text-center text-sm text-white/60">
                    <b class="font-semibold text-white/85">"¿Venís a comprar?"</b>
                    " No necesitás cuenta: comprá desde el enlace de la tienda que te pasaron."
                </p>
            </main>
        </div>
    }
}

#[component]
fn AlreadyIn(me: Me, next: String) -> impl IntoView {
    let platform = crate::site::current_site() == domain::Site::Platform;
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/entrar");
        }
    });
    let hello = if me.name.is_empty() {
        "Ya iniciaste sesión".to_string()
    } else {
        format!("Hola, {}", me.name)
    };
    view! {
        <div class="flex flex-col gap-4">
            <h1 class="text-[28px] leading-tight font-extrabold tracking-tight">{hello}</h1>
            <p class="-mt-2 text-base-content/70">"Estás con la cuenta " <strong class="text-base-content">{me.email.clone()}</strong> "."</p>
            {(!me.can_edit && !platform).then(|| view! {
                <FieldNote ok=false text="Tu cuenta no puede editar esta tienda."/>
            })}
            {(me.can_edit || platform).then(|| view! {
                <a class="btn btn-primary h-12" href=next.clone()>{if platform { "Ir a mis tiendas" } else { "Ir al editor" }}</a>
            })}
            <div class="flex flex-wrap justify-center gap-2">
                {platform.then(|| view! { <a class="btn btn-ghost btn-sm" href="/cuenta">"Mi cuenta"</a> })}
                {(!platform).then(|| view! { <a class="btn btn-ghost btn-sm" href="/">"Ver la tienda"</a> })}
                <button class="btn btn-ghost btn-sm" on:click=move |_| { out.dispatch(()); }>"Cerrar sesión"</button>
            </div>
        </div>
    }
}

/// Campo de contraseña con "Mostrar" y, si `meter`, la barrita de qué tan
/// buena es. `extra` va a la derecha de la etiqueta; `note`, debajo del campo.
#[component]
fn PasswordField(
    value: RwSignal<String>,
    #[prop(into)] label: String,
    #[prop(into)] autocomplete: String,
    #[prop(optional)] meter: bool,
    #[prop(optional)] autofocus: bool,
    #[prop(optional)] extra: Option<AnyView>,
    #[prop(optional, into)] bad: Signal<bool>,
    #[prop(optional)] note: Option<AnyView>,
) -> impl IntoView {
    let show = RwSignal::new(false);
    let id = format!("pw-{}", label.to_lowercase().replace(' ', "-"));
    let score = move || domain::password_score(&value.get());
    view! {
        <div class="flex flex-col gap-1.5">
            <div class="flex items-baseline justify-between gap-2">
                <label for=id.clone() class="text-sm font-semibold">{label}</label>
                {extra}
            </div>
            <div class=move || format!("input h-12 flex w-full items-center gap-2 pr-1 {}", if bad.get() { "input-error" } else { "" })>
                <input id=id name=autocomplete.clone() type=move || if show.get() { "text" } else { "password" }
                    class="grow text-base" required minlength=domain::PASSWORD_MIN.to_string() maxlength=domain::PASSWORD_MAX.to_string()
                    autocomplete=autocomplete enterkeyhint="go" data-autofocus=autofocus.then_some("")
                    aria-invalid=move || bad.get().then_some("true")
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))/>
                <button type="button" class="btn btn-ghost btn-sm text-base-content/60"
                    aria-pressed=move || show.get().to_string()
                    on:click=move |_| show.update(|s| *s = !*s)>
                    {move || if show.get() { "Ocultar" } else { "Mostrar" }}
                </button>
            </div>
            {note}
            {meter.then(|| view! {
                <div class="mt-1 flex gap-1" aria-hidden="true">
                    {(1..=4u8).map(|i| view! {
                        <i class=move || {
                            let s = score();
                            let color = if s < i { "bg-base-300" } else if s <= 1 { "bg-error" } else if s == 2 { "bg-warning" } else { "bg-success" };
                            format!("h-1 flex-1 rounded-full {color}")
                        }></i>
                    }).collect_view()}
                </div>
                // Con un error debajo del campo, el error ya dice lo que falta:
                // este texto se calla para no repetirlo (ni contradecirlo).
                <p class="text-xs text-base-content/70" aria-live="polite" class:hidden=move || bad.get()>
                    {move || match score() {
                        0 => format!("{} caracteres o más.", domain::PASSWORD_MIN),
                        1 if value.with(|v| v.chars().count() >= domain::PASSWORD_MIN) && domain::is_common_password(&value.get()) => "Muy común: se adivina fácil. Elegí otra.".to_string(),
                        1 => format!("Muy corta: {} caracteres o más.", domain::PASSWORD_MIN),
                        2 => "Aceptable. Con mayúsculas, números o símbolos es más segura.".to_string(),
                        3 => "Buena.".to_string(),
                        _ => "Muy buena.".to_string(),
                    }}
                </p>
            })}
        </div>
    }
}

/// Qué campo tiene el aviso de error, para mostrarlo justo debajo.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Spot {
    Email,
    Name,
    Password,
    /// Al pie del formulario (problemas de conexión, límites).
    Form,
}

#[component]
fn SignInForm(next: String, create: bool, link: Option<Link>) -> impl IntoView {
    let next = StoredValue::new(next);
    let first = match &link {
        Some(l) => Step::NewPassword {
            new_account: l.new_account,
        },
        None if create => Step::Create,
        None => Step::SignIn,
    };
    let step = RwSignal::new(first);
    let email = RwSignal::new(link.as_ref().map(|l| l.email.clone()).unwrap_or_default());
    // El código viaja en el enlace; nadie lo escribe.
    let code = StoredValue::new(link.map(|l| l.code).unwrap_or_default());
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<(Spot, String)>);
    let info = RwSignal::new(None::<String>);

    let login = Action::new(|(e, p, n): &(String, String, String)| {
        sign_in(e.clone(), p.clone(), n.clone())
    });
    // El `bool` es solo para saber qué pantalla mostrar después.
    let ask = Action::new(|(e, _new_account): &(String, bool)| request_password_link(e.clone()));
    let finish = Action::new(
        |(e, c, nm, p, n): &(String, String, String, String, String)| {
            set_password_with_code(e.clone(), c.clone(), nm.clone(), p.clone(), n.clone())
        },
    );
    let busy = move || login.pending().get() || ask.pending().get() || finish.pending().get();
    let fail = move |spot: Spot, text: &str| problem.set(Some((spot, text.to_string())));
    // Al escribir de nuevo, el aviso de error anterior se va.
    Effect::new(move |prev: Option<()>| {
        email.track();
        name.track();
        password.track();
        if prev.is_some() {
            problem.set(None);
        }
    });
    let goto = move |s: Step| {
        problem.set(None);
        step.set(s);
        focus_first();
    };
    // Revisa el email antes de mandarlo; si no sirve, deja el aviso debajo.
    let email_ok = move || {
        let e = email.get_untracked();
        if e.trim().is_empty() {
            fail(Spot::Email, "Escribí tu email.");
            false
        } else if !domain::looks_like_email(e.trim()) {
            fail(
                Spot::Email,
                "El email no es válido. Revisalo (por ejemplo, ana@correo.com).",
            );
            false
        } else {
            true
        }
    };
    // Si el último enlace pedido es para crear la cuenta. `ask.input()` ya está
    // vacío cuando llega la respuesta, y la pantalla mostraba el texto de
    // "contraseña nueva" a quien se estaba registrando.
    let asked_new = StoredValue::new(false);
    let send_link = move |new_account: bool| {
        info.set(None);
        if email.get_untracked().trim().is_empty() {
            fail(
                Spot::Email,
                "Escribí tu email y te mandamos un enlace para elegir una contraseña nueva.",
            );
            return;
        }
        if !email_ok() {
            return;
        }
        problem.set(None);
        asked_new.set_value(new_account);
        ask.dispatch((email.get_untracked(), new_account));
    };

    // Contraseña no aceptada: el mensaje es el mismo exista o no la cuenta,
    // para que Entrar no sirva para averiguar qué emails están en la lista.
    let wrong_password = "El email o la contraseña no son correctos. Probá de nuevo o elegí otra con \"¿La olvidaste?\".";
    Effect::new(move |_| {
        let Some(result) = login.value().get() else {
            return;
        };
        match result {
            Ok(PasswordSignIn::In { next }) => go(&next),
            Ok(PasswordSignIn::Wrong) => fail(Spot::Password, wrong_password),
            Ok(PasswordSignIn::Locked { minutes }) => problem.set(Some((Spot::Password, format!(
                "Hubo demasiados intentos seguidos. Esperá {minutes} minutos o elegí una contraseña nueva con \"¿La olvidaste?\"."
            )))),
            Ok(PasswordSignIn::Weak { reason }) => problem.set(Some((
                Spot::Password,
                format!("Tu cuenta todavía no tiene contraseña: la que escribas acá va a ser la tuya. {reason}"),
            ))),
            Ok(PasswordSignIn::NoPassword) => {
                send_link(false);
                info.set(Some(
                    "Tu cuenta todavía no tiene contraseña. Confirmá tu email con el enlace y elegila: es solo esta vez, después entrás directo con tu contraseña.".into(),
                ));
            }
            Err(_) => fail(Spot::Form, CONNECTION),
        }
    });
    Effect::new(move |_| {
        let Some(result) = ask.value().get() else {
            return;
        };
        let new_account = asked_new.get_value();
        match result {
            Ok(CodeRequest::Sent) => goto(Step::LinkSent { new_account }),
            Ok(CodeRequest::BadEmail) => fail(Spot::Email, "El email no es válido. Revisalo."),
            Ok(CodeRequest::NotAllowed) => fail(Spot::Email, NOT_ALLOWED),
            Ok(CodeRequest::TooMany) => fail(
                Spot::Form,
                "Se pidieron demasiados enlaces seguidos. Esperá unos minutos y probá de nuevo.",
            ),
            Err(_) => fail(Spot::Form, CONNECTION),
        }
    });
    let expired = "Este enlace venció o ya se usó. Pedí uno nuevo con \"¿La olvidaste?\".";
    let expired_invite =
        "Este enlace venció o ya se usó. Pedí uno nuevo desde Entrar, con \"Creá tu cuenta\".";
    Effect::new(move |_| {
        let Some(result) = finish.value().get() else {
            return;
        };
        match result {
            Ok(SetPassword::In { next }) => go(&next),
            Ok(SetPassword::Wrong { .. }) | Ok(SetPassword::Expired) => {
                let new_account = matches!(
                    step.get_untracked(),
                    Step::NewPassword { new_account: true }
                );
                fail(
                    Spot::Form,
                    if new_account { expired_invite } else { expired },
                )
            }
            Ok(SetPassword::Weak { reason }) => problem.set(Some((Spot::Password, reason))),
            Ok(SetPassword::NotAllowed) => fail(Spot::Form, NOT_ALLOWED),
            Err(_) => fail(Spot::Form, CONNECTION),
        }
    });

    // El aviso de error, si es para este lugar.
    let note_at = move |spot: Spot| {
        move || {
            problem
                .get()
                .filter(|(s, _)| *s == spot)
                .map(|(_, p)| view! { <FieldNote ok=false text=p/> })
        }
    };
    let bad = move |spot: Spot| {
        Signal::derive(move || problem.with(|p| matches!(p, Some((s, _)) if *s == spot)))
    };
    let shown_email = move || email.get().trim().to_lowercase();
    let submit_btn = move |label: &'static str, busy_label: &'static str| {
        view! {
            <button class="btn btn-primary h-12 text-[15px] shadow-[0_8px_20px_-8px_rgba(67,48,214,.7)]" type="submit" disabled=busy>
                {move || if busy() { busy_label } else { label }}
            </button>
        }
    };
    let heading = |title: &'static str, sub: &'static str| {
        view! {
            <div>
                <h1 class="text-[28px] leading-tight font-extrabold tracking-tight">{title}</h1>
                <p class="mt-1.5 text-[15px] text-base-content/70">{sub}</p>
            </div>
        }
    };
    let email_field = move |autofocus: bool| {
        view! {
            <div class="flex flex-col gap-1.5">
                <label for="email" class="text-sm font-semibold">"Email"</label>
                <input id="email" type="email" name="email" required autocomplete="username"
                    spellcheck="false" enterkeyhint="next" data-autofocus=autofocus.then_some("")
                    class=move || format!("input h-12 w-full text-base {}", if bad(Spot::Email).get() { "input-error" } else { "" })
                    aria-invalid=move || bad(Spot::Email).get().then_some("true")
                    placeholder="ana@correo.com"
                    prop:value=move || email.get()
                    on:input=move |ev| email.set(event_target_value(&ev))/>
                {note_at(Spot::Email)}
            </div>
        }
    };
    let link = "text-sm font-semibold text-primary hover:underline disabled:opacity-50";
    let name_field = move || {
        view! {
            <div class="flex flex-col gap-1.5">
                <label for="name" class="text-sm font-semibold">"Tu nombre"</label>
                <input id="name" type="text" name="name" autocomplete="given-name" data-autofocus=""
                    enterkeyhint="next" maxlength=domain::ACCOUNT_NAME_MAX.to_string()
                    class=move || format!("input h-12 w-full text-base {}", if bad(Spot::Name).get() { "input-error" } else { "" })
                    placeholder="Ana"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))/>
                {note_at(Spot::Name)}
            </div>
        }
    };

    move || {
        match step.get() {
        Step::SignIn => view! {
            <form class="flex flex-col gap-6" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                info.set(None);
                if !email_ok() {
                    return;
                }
                if password.get_untracked().is_empty() {
                    fail(Spot::Password, "Escribí tu contraseña.");
                    return;
                }
                problem.set(None);
                login.dispatch((email.get_untracked(), password.get_untracked(), next.get_value()));
            }>
                {heading("Entrá a tu cuenta", "Para manejar tus tiendas, productos y pedidos.")}
                <div class="flex flex-col gap-4">
                    {move || info.get().map(|i| view! { <FieldNote ok=true text=i/> })}
                    {email_field(true)}
                    <PasswordField value=password label="Contraseña" autocomplete="current-password"
                        bad=bad(Spot::Password) note=view! { {note_at(Spot::Password)} }.into_any()
                        extra=view! {
                            <button type="button" class=link disabled=busy
                                on:click=move |_| send_link(false)>"¿La olvidaste?"</button>
                        }.into_any()/>
                    {note_at(Spot::Form)}
                    {submit_btn("Entrar", "Entrando…")}
                </div>
                <p class="border-t border-base-300 pt-5 text-center text-sm text-base-content/70">
                    "¿Te invitaron a MarketMaker? "
                    <button type="button" class=link on:click=move |_| { info.set(None); password.set(String::new()); goto(Step::Create) }>"Creá tu cuenta"</button>
                </p>
            </form>
        }.into_any(),
        Step::Create => view! {
            <form class="flex flex-col gap-6" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                if email.get_untracked().trim().is_empty() {
                    fail(Spot::Email, "Escribí tu email.");
                    return;
                }
                send_link(true);
            }>
                <div class="flex flex-col gap-3">
                    <button type="button" class="self-start text-sm text-base-content/70 hover:text-base-content"
                        on:click=move |_| { password.set(String::new()); goto(Step::SignIn) }>"← Ya tengo cuenta"</button>
                    {heading("Creá tu cuenta", "Escribí el email al que te llegó la invitación y te mandamos un enlace para crearla.")}
                </div>
                <div class="flex flex-col gap-4">
                    {email_field(true)}
                    {note_at(Spot::Form)}
                    {submit_btn("Mandarme el enlace", "Mandando…")}
                </div>
            </form>
        }.into_any(),
        Step::LinkSent { new_account } => view! {
            <div class="flex flex-col items-center text-center">
                <div class="grid size-16 place-items-center rounded-full bg-primary/10 text-primary" aria-hidden="true">
                    <ui::Icon name="mail" class="size-7"/>
                </div>
                <h1 class="mt-5 text-[28px] leading-tight font-extrabold tracking-tight">"Revisá tu email"</h1>
                // No decimos si la dirección está en la lista: la pantalla es
                // la misma para cualquier email.
                <p role="status" class="mt-2.5 text-[15px] leading-relaxed text-base-content/70">
                    {if new_account {
                        "Si esta dirección tiene una invitación, le mandamos un enlace para crear la cuenta."
                    } else {
                        "Si esta dirección tiene una cuenta, le mandamos un enlace para elegir una contraseña nueva."
                    }}
                </p>
                <p class="mt-5 inline-flex max-w-full items-center gap-2 rounded-full bg-base-200 px-4 py-2 text-[15px] font-semibold">
                    <ui::Icon name="mail" class="size-4 shrink-0 text-base-content/50"/>
                    <span class="truncate">{shown_email}</span>
                </p>
                <p class="mt-3 flex items-center gap-1.5 text-[13px] text-base-content/60">
                    <ui::Icon name="clock" class="size-4"/>
                    "El enlace vence en 30 minutos."
                </p>
                {move || info.get().map(|i| view! { <p class="mt-4 text-sm text-base-content/70">{i}</p> })}
                {cfg!(debug_assertions).then(|| view! {
                    <p class="mt-4 text-sm text-base-content/70">"En desarrollo el email no se manda: el enlace aparece en la terminal donde corre el servidor."</p>
                })}
                <div class="mt-7 flex w-full flex-col gap-1.5 border-t border-base-300 pt-5 text-sm leading-relaxed text-base-content/70">
                    <p>
                        "¿No te llegó? Revisá el correo no deseado o "
                        <button type="button" class=link disabled=busy
                            on:click=move |_| send_link(new_account)>"mandalo de nuevo"</button>
                        "."
                    </p>
                    <p>
                        "¿Te equivocaste de dirección? "
                        <button type="button" class=link
                            on:click=move |_| {
                                info.set(None);
                                password.set(String::new());
                                goto(if new_account { Step::Create } else { Step::SignIn })
                            }>"Usá otro email"</button>
                        "."
                    </p>
                    {note_at(Spot::Form)}
                </div>
                <button type="button" class=format!("{link} mt-5")
                    on:click=move |_| { info.set(None); password.set(String::new()); goto(Step::SignIn) }>"← Volver a Entrar"</button>
            </div>
        }.into_any(),
        Step::NewPassword { new_account } => view! {
            <form class="flex flex-col gap-6" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                if new_account && name.get_untracked().trim().is_empty() {
                    fail(Spot::Name, "Escribí tu nombre.");
                    return;
                }
                if let Some(reason) = domain::password_problem(&password.get_untracked()) {
                    problem.set(Some((Spot::Password, reason)));
                    return;
                }
                problem.set(None);
                finish.dispatch((email.get_untracked(), code.get_value(), name.get_untracked(), password.get_untracked(), next.get_value()));
            }>
                <div>
                    <h1 class="text-[28px] leading-tight font-extrabold tracking-tight">
                        {if new_account { "Creá tu cuenta" } else { "Elegí tu contraseña nueva" }}
                    </h1>
                    <p class="mt-1.5 text-[15px] text-base-content/70">"Para la cuenta " <strong class="text-base-content">{shown_email}</strong> "."</p>
                </div>
                <div class="flex flex-col gap-4">
                    // Para que el gestor de contraseñas la guarde con el email correcto.
                    <input type="email" name="email" autocomplete="username" prop:value=move || email.get() hidden/>
                    {new_account.then(name_field)}
                    <PasswordField value=password label="Contraseña nueva" autocomplete="new-password" meter=true autofocus=!new_account
                        bad=bad(Spot::Password) note=view! { {note_at(Spot::Password)} }.into_any()/>
                    {note_at(Spot::Form)}
                    {if new_account { submit_btn("Crear cuenta y entrar", "Creando cuenta…") } else { submit_btn("Guardar y entrar", "Guardando…") }}
                    {new_account.then(|| view! {
                        <p class="text-xs text-base-content/60">"Al crear la cuenta aceptás los "<a class="link" href="/terminos" target="_blank">"términos"</a>" y la "<a class="link" href="/privacidad" target="_blank">"política de privacidad"</a>"."</p>
                    })}
                    <button type="button" class=format!("{link} self-center")
                        on:click=move |_| { password.set(String::new()); goto(Step::SignIn) }>"Volver a Entrar"</button>
                </div>
            </form>
        }.into_any(),
    }
    }
}

/// `/cuenta`: nombre, email, contraseña y salir, dentro del panel. Se abre
/// desde el menú de tu nombre (abajo a la izquierda); no está en el menú.
#[component]
pub fn AccountPage() -> impl IntoView {
    let store = use_query_map()
        .read_untracked()
        .get("tienda")
        .unwrap_or_default();
    let account = Resource::new(|| (), |_| load_account());
    view! {
        <leptos_meta::Title text="Mi cuenta · MarketMaker"/>
        <crate::panel::StoreShell slug=store section=crate::panel::Section::Account>
            <div class="flex w-full max-w-[1440px] flex-col gap-4 px-8 py-8 max-sm:px-4 max-sm:py-5">
                <div>
                    <h1 class="text-3xl font-extrabold tracking-tight max-sm:text-2xl">"Mi cuenta"</h1>
                    <p class="mt-1 text-base-content/70">"Tus datos para entrar a MarketMaker. Valen para todas tus tiendas."</p>
                </div>
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Account/> }>
                    {move || Suspend::new(async move {
                        match account.await {
                            Ok(Some(a)) => view! { <AccountRows account=a/> }.into_any(),
                            Ok(None) => view! { <leptos_router::components::Redirect path="/entrar?volver=/cuenta"/> }.into_any(),
                            Err(_) => view! { <FieldNote ok=false text=CONNECTION/> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
}

/// Una fila de Mi cuenta: ícono, qué es, el valor y una acción.
#[component]
fn AccountRow(
    icon: &'static str,
    label: &'static str,
    children: Children,
    #[prop(optional)] action: Option<AnyView>,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-4 border-t border-base-300 px-6 py-4 max-sm:flex-wrap max-sm:gap-x-3 max-sm:gap-y-1 max-sm:px-4">
            <ui::Icon class="size-[18px] shrink-0 text-base-content/60 max-sm:hidden" name=icon/>
            <div class="w-32 shrink-0 text-sm font-semibold max-sm:w-full max-sm:text-xs max-sm:font-medium max-sm:text-base-content/60">{label}</div>
            <div class="min-w-0 grow">{children()}</div>
            {action}
        </div>
    }
}

fn open_modal(d: NodeRef<leptos::html::Dialog>) {
    if let Some(d) = d.get() {
        _ = d.show_modal();
    }
}

fn close_modal(d: NodeRef<leptos::html::Dialog>) {
    if let Some(d) = d.get() {
        d.close();
    }
}

#[component]
fn AccountRows(account: domain::AccountInfo) -> impl IntoView {
    let saved_name = RwSignal::new(account.name.clone());
    let has_password = RwSignal::new(account.has_password);
    let notice = RwSignal::new(None::<String>);
    let name_dialog = NodeRef::<leptos::html::Dialog>::new();
    let pw_dialog = NodeRef::<leptos::html::Dialog>::new();
    let delete_dialog = NodeRef::<leptos::html::Dialog>::new();
    let account_email = account.email.clone();
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/");
        }
    });
    let avatar = account.avatar.clone();
    let first_of_email = account.email.chars().next();
    let initial = move || {
        saved_name
            .get()
            .chars()
            .next()
            .or(first_of_email)
            .unwrap_or('?')
            .to_uppercase()
            .to_string()
    };
    let stores = match account.stores {
        0 => "Todavía sin tiendas".to_string(),
        1 => "1 tienda".to_string(),
        n => format!("{n} tiendas"),
    };
    let change_btn = |label: Signal<&'static str>, dialog: NodeRef<leptos::html::Dialog>| {
        view! {
            <button type="button" class="btn btn-outline btn-sm border-base-300 bg-base-100"
                on:click=move |_| { notice.set(None); open_modal(dialog) }>{label}</button>
        }
        .into_any()
    };
    view! {
        <section class="overflow-hidden rounded-2xl border border-base-300 bg-base-100">
            <div class="flex items-center gap-4 px-6 py-5">
                <ProfilePhoto letter=Signal::derive(initial) photo=avatar.clone()>
                    <div class="min-w-0 grow">
                        <div class="truncate text-lg font-bold">{move || if saved_name.get().is_empty() { "Sin nombre".to_string() } else { saved_name.get() }}</div>
                        <div class="text-sm text-base-content/70">{stores}</div>
                    </div>
                </ProfilePhoto>
            </div>
            <AccountRow icon="user" label="Nombre" action=change_btn(Signal::stored("Cambiar"), name_dialog)>
                <span class=move || if saved_name.get().is_empty() { "text-base-content/50" } else { "" }>
                    {move || if saved_name.get().is_empty() { "Sin nombre".to_string() } else { saved_name.get() }}
                </span>
            </AccountRow>
            <AccountRow icon="mail" label="Email"
                action=view! {
                    <a class="link link-hover text-xs" href="mailto:hola@mkr.com.ar?subject=Cambiar%20mi%20email">"Pedir cambio"</a>
                }.into_any()>
                <div class="truncate">{account.email.clone()}</div>
                <div class="text-xs text-base-content/60">"Con este email entrás. Para cambiarlo, escribinos a hola@mkr.com.ar."</div>
            </AccountRow>
            <AccountRow icon="key" label="Contraseña"
                action=change_btn(Signal::derive(move || if has_password.get() { "Cambiar" } else { "Elegir" }), pw_dialog)>
                {move || if has_password.get() {
                    view! {
                        <div class="tracking-widest">"••••••••"</div>
                    }.into_any()
                } else {
                    view! {
                        <div class="text-base-content/50">"Sin contraseña"</div>
                        <div class="text-xs text-base-content/60">"Elegí una para iniciar sesión sin esperar un código por email."</div>
                    }.into_any()
                }}
            </AccountRow>
        </section>
        {move || notice.get().map(|n| view! { <div class="pl-1"><FieldNote ok=true text=n/></div> })}
        <section class="rounded-2xl border border-base-300 bg-base-100">
            <AccountRow icon="log-out" label="Salir"
                action=view! {
                    <button type="button" class="btn btn-outline btn-sm border-red-200 bg-base-100 text-red-700 hover:border-red-300 hover:bg-red-50"
                        on:click=move |_| { out.dispatch(()); }>"Salir"</button>
                }.into_any()>
                "Cerrar sesión en este dispositivo"
            </AccountRow>
            <AccountRow icon="trash" label="Borrar mi cuenta"
                action=view! {
                    <button type="button" class="btn btn-outline btn-sm border-red-200 bg-base-100 text-red-700 hover:border-red-300 hover:bg-red-50"
                        on:click=move |_| open_modal(delete_dialog)>"Borrar…"</button>
                }.into_any()>
                <span class="text-sm text-base-content/70">"Se borran tu cuenta, tus fotos y las tiendas que son solo tuyas."</span>
            </AccountRow>
        </section>
        <DeleteAccountDialog dialog=delete_dialog email=account_email/>
        <NameDialog dialog=name_dialog saved_name/>
        <PasswordDialog dialog=pw_dialog has_password notice/>
    }
}

/// "¿Borrar tu cuenta?": hay que escribir el email para confirmar.
#[component]
fn DeleteAccountDialog(dialog: NodeRef<leptos::html::Dialog>, email: String) -> impl IntoView {
    let typed = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let delete = Action::new(|t: &String| crate::auth::delete_my_account(t.clone()));
    Effect::new(move |_| match delete.value().get() {
        Some(Ok(Ok(()))) => go("/"),
        Some(Ok(Err(p))) => problem.set(Some(p)),
        Some(Err(_)) => problem.set(Some(CONNECTION.to_string())),
        None => {}
    });
    view! {
        <dialog node_ref=dialog class="modal" aria-labelledby="borrar-cuenta">
            <div class="modal-box max-w-md">
                <h2 id="borrar-cuenta" class="text-lg font-bold">"¿Borrar tu cuenta?"</h2>
                <p class="mt-1 text-sm text-base-content/70">"No se puede deshacer. Escribí "<b>{email}</b>" para confirmar."</p>
                <form class="mt-4 flex flex-col gap-3"
                    on:submit=move |ev| { ev.prevent_default(); problem.set(None); delete.dispatch(typed.get_untracked()); }>
                    <input class="input w-full" type="email" aria-label="Tu email" autocomplete="off"
                        prop:value=move || typed.get()
                        on:input=move |ev| typed.set(event_target_value(&ev))/>
                    {move || problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                    <div class="flex justify-end gap-2">
                        <button type="button" class="btn btn-ghost" autofocus on:click=move |_| close_modal(dialog)>"Cancelar"</button>
                        <button class="btn btn-error" disabled=move || delete.pending().get() || typed.with(|t| t.trim().is_empty())>"Borrar mi cuenta"</button>
                    </div>
                </form>
            </div>
            <form method="dialog" class="modal-backdrop"><button aria-label="Cerrar">"cerrar"</button></form>
        </dialog>
    }
}

/// La foto de perfil: se toca para elegir otra (o subir la primera) y
/// "Quitar" vuelve a la inicial. La foto nueva se sube a Mis fotos (así se
/// convierte y se achica como las demás). Después recarga, para que el menú
/// del costado la muestre. `children` va entre la foto y "Quitar".
#[component]
fn ProfilePhoto(
    letter: Signal<String>,
    photo: Option<String>,
    children: Children,
) -> impl IntoView {
    let has_photo = photo.is_some();
    let photos = editor::MyPhotos::new();
    let problem = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let apply = move |id: Option<String>| {
        saving.set(true);
        problem.set(None);
        leptos::task::spawn_local(async move {
            match save_avatar(id).await {
                Ok(()) => {
                    let _ = window().location().reload();
                }
                Err(e) => {
                    saving.set(false);
                    problem.set(Some(server_message(e)));
                }
            }
        });
    };
    let busy = move || saving.get() || photos.progress.get().is_some();
    let hint = if has_photo {
        "Cambiar foto de perfil"
    } else {
        "Subir foto de perfil"
    };
    view! {
        <label for="foto-perfil" title=hint
            class="group relative shrink-0 cursor-pointer rounded-full has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-primary">
            {move || view! { <crate::panel::Avatar letter=letter.get() size="size-13 text-xl" photo=photo.clone()/> }}
            <span aria-hidden="true" class=move || format!(
                "absolute inset-0 grid place-items-center rounded-full bg-black/45 text-white transition-opacity {}",
                if busy() { "opacity-100" } else { "opacity-0 group-hover:opacity-100" }
            )>
                {move || if busy() {
                    view! { <span class="loading loading-spinner loading-sm"></span> }.into_any()
                } else {
                    view! { <ui::Icon class="size-5" name="camera"/> }.into_any()
                }}
            </span>
            <span aria-hidden="true" class="absolute -right-0.5 -bottom-0.5 grid size-6 place-items-center rounded-full border-2 border-base-100 bg-base-content text-base-100">
                <ui::Icon class="size-3" name="camera"/>
            </span>
            <input id="foto-perfil" type="file" class="sr-only" aria-label=hint disabled=busy
                accept="image/jpeg,image/png,image/webp,image/gif"
                on:change=move |ev| {
                    problem.set(None);
                    photos.upload(
                        editor::take_files(&ev).into_iter().take(1).collect(),
                        Callback::new(move |item: domain::MediaItem| apply(Some(item.id))),
                    )
                }/>
        </label>
        {children()}
        <div class="flex shrink-0 flex-col items-end gap-1">
            {has_photo.then(|| view! {
                <button type="button" class="btn btn-ghost btn-sm text-base-content/70" disabled=busy
                    on:click=move |_| apply(None)>"Quitar foto"</button>
            })}
            {move || {
                let errors = photos.upload_errors.get();
                problem.get().or_else(|| errors.into_iter().next())
                    .map(|text| view! { <FieldNote ok=false text/> })
            }}
        </div>
    }
}

/// El texto de un error del servidor, sin el prefijo técnico.
fn server_message(e: ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => msg,
        _ => CONNECTION.to_string(),
    }
}

#[component]
fn NameDialog(
    dialog: NodeRef<leptos::html::Dialog>,
    saved_name: RwSignal<String>,
) -> impl IntoView {
    let name = RwSignal::new(saved_name.get_untracked());
    let problem = RwSignal::new(None::<String>);
    let save = Action::new(|n: &String| save_name(n.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(n)) => {
            saved_name.set(n.clone());
            name.set(n);
            close_modal(dialog);
            // Recarga para que el nombre nuevo se vea también en el menú del costado.
            let _ = window().location().reload();
        }
        Some(Err(_)) => problem.set(Some(CONNECTION.into())),
        None => {}
    });
    view! {
        <dialog node_ref=dialog class="modal" aria-labelledby="nombre-titulo"
            on:close=move |_| { problem.set(None); name.set(saved_name.get_untracked()); }>
            <div class="modal-box max-w-md">
                <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    problem.set(None);
                    if domain::clean_name(&name.get_untracked()).is_empty() {
                        problem.set(Some("Escribí tu nombre.".into()));
                        return;
                    }
                    save.dispatch(name.get_untracked());
                }>
                    <h2 id="nombre-titulo" class="text-xl font-extrabold">"Cambiar tu nombre"</h2>
                    <label class="flex flex-col gap-1.5">
                        <span class="text-sm font-semibold">"¿Cómo te llamás?"</span>
                        <input type="text" class="input w-full" autocomplete="given-name"
                            maxlength=domain::ACCOUNT_NAME_MAX.to_string()
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))/>
                    </label>
                    {move || problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                    <div class="flex justify-end gap-2">
                        <button type="button" class="btn btn-ghost" on:click=move |_| close_modal(dialog)>"Cancelar"</button>
                        <button type="submit" class="btn btn-primary" disabled=move || save.pending().get()>
                            {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                        </button>
                    </div>
                </form>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}

#[component]
fn PasswordDialog(
    dialog: NodeRef<leptos::html::Dialog>,
    has_password: RwSignal<bool>,
    notice: RwSignal<Option<String>>,
) -> impl IntoView {
    let current = RwSignal::new(String::new());
    let new = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let change = Action::new(|(c, n): &(String, String)| change_password(c.clone(), n.clone()));
    Effect::new(move |prev: Option<()>| {
        current.track();
        new.track();
        if prev.is_some() {
            problem.set(None);
        }
    });
    Effect::new(move |_| match change.value().get() {
        Some(Ok(PasswordChange::Done)) => {
            has_password.set(true);
            notice.set(Some(
                "Tu contraseña se actualizó. Cerramos tu sesión en todos tus otros dispositivos."
                    .into(),
            ));
            close_modal(dialog);
        }
        Some(Ok(PasswordChange::WrongCurrent)) => {
            problem.set(Some("La contraseña actual no es correcta.".into()))
        }
        // Si el medidor de abajo del campo ya lo dice, no se repite.
        Some(Ok(PasswordChange::Weak { reason })) => {
            if domain::password_problem(&new.get_untracked()).as_deref() != Some(reason.as_str()) {
                problem.set(Some(reason));
            }
        }
        Some(Err(_)) => problem.set(Some(CONNECTION.into())),
        None => {}
    });
    view! {
        <dialog node_ref=dialog class="modal" aria-labelledby="clave-titulo"
            on:close=move |_| { current.set(String::new()); new.set(String::new()); problem.set(None); }>
            <div class="modal-box max-w-md">
                <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    problem.set(None);
                    change.dispatch((current.get_untracked(), new.get_untracked()));
                }>
                    <h2 id="clave-titulo" class="text-xl font-extrabold">
                        {move || if has_password.get() { "Cambiar contraseña" } else { "Elegir contraseña" }}
                    </h2>
                    {move || has_password.get().then(|| view! {
                        <PasswordField value=current label="Contraseña actual" autocomplete="current-password"/>
                    })}
                    <PasswordField value=new label="Contraseña nueva" autocomplete="new-password" meter=true/>
                    {move || problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                    <p class="text-sm text-base-content/60">"Al cambiarla, cerramos tu sesión en todos tus otros dispositivos."</p>
                    <div class="flex justify-end gap-2">
                        <button type="button" class="btn btn-ghost" on:click=move |_| close_modal(dialog)>"Cancelar"</button>
                        <button type="submit" class="btn btn-primary" disabled=move || change.pending().get()>
                            {move || if change.pending().get() { "Guardando…" } else { "Guardar" }}
                        </button>
                    </div>
                </form>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}

/// Barra fina arriba de la tienda, solo para quien puede editarla.
#[component]
pub fn OwnerBar(me: Option<Me>) -> impl IntoView {
    let hidden = RwSignal::new(false);
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/");
        }
    });
    let can_edit = me.as_ref().is_some_and(|m| m.can_edit);
    can_edit.then(|| view! {
        <Show when=move || !hidden.get()>
            <div data-theme="light" role="region" aria-label="Tu tienda"
                class="bg-neutral text-neutral-content text-sm px-4 py-2 flex flex-wrap items-center gap-x-4 gap-y-2">
                <span class="grow">"Estás viendo tu tienda como la ven tus clientes."</span>
                <a class="btn btn-sm btn-primary" href="/editor">"Editar mi tienda"</a>
                <a class="btn btn-sm btn-ghost text-neutral-content hover:bg-white/10" href=crate::site::platform_url()>"Tus tiendas"</a>
                <button class="btn btn-sm btn-ghost text-neutral-content hover:bg-white/10" title="Esconde esta barra hasta que recargues la página"
                    on:click=move |_| hidden.set(true)>"Ver como cliente"</button>
                <button class="btn btn-sm btn-ghost text-neutral-content hover:bg-white/10" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
            </div>
        </Show>
    })
}
