//! Pantallas de la cuenta del dueño: Entrar (`/entrar`), "Mi cuenta"
//! (`/cuenta`) y la barra "Editar mi tienda" que el dueño ve arriba de su
//! tienda.

use crate::auth::{
    change_password, check_code, create_account, load_account, lookup_email, request_code,
    save_avatar, save_name, set_password_with_code, sign_in, sign_out,
};
use domain::{
    AccountCreation, CodeCheck, CodeRequest, EmailLookup, Me, PasswordChange, PasswordSignIn,
    SetPassword,
};
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

/// Para qué es el código que mandamos por email.
#[derive(Clone, Copy, PartialEq, Debug)]
enum CodeFor {
    /// Confirmar el email de una cuenta nueva (nombre y contraseña ya escritos).
    /// Solo si no está invitado: los invitados entran sin código.
    NewAccount,
    /// Elegir una contraseña nueva (se la olvidó, o la cuenta es de antes y no tenía).
    Reset,
    /// Entrar solo con el código, sin contraseña.
    Login,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Step {
    Email,
    Password,
    Create,
    Code(CodeFor),
}

/// Aviso chico debajo de un campo: rojo con un círculo si algo salió mal,
/// verde con un tilde si salió bien. Sin recuadro, para que no grite.
#[component]
fn FieldNote(ok: bool, #[prop(into)] text: String) -> impl IntoView {
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

/// `/entrar`: primero el email; si ya tiene cuenta, la contraseña; si es
/// nuevo e invitado, nombre y contraseña y ya está adentro.
#[component]
pub fn SignInPage() -> impl IntoView {
    let query = use_query_map();
    // Sin `volver`: en una tienda, al editor; en la plataforma, a tus tiendas.
    let next = move || match query.read().get("volver") {
        Some(v) => domain::safe_next(&v),
        None if crate::site::current_site() == domain::Site::Platform => "/".to_string(),
        None => "/editor".to_string(),
    };
    let me = Resource::new(|| (), |_| crate::auth::who_am_i());
    view! {
        <leptos_meta::Title text="Entrar · MarketMaker"/>
        <div data-theme="light" class="grid min-h-screen bg-base-100 text-base-content lg:grid-cols-2">
            <div class="flex flex-col px-6 py-8 sm:px-12 lg:px-16">
                <a href="/" class="self-start text-lg"><Logo/></a>
                <div class="mx-auto my-auto flex w-full max-w-sm flex-col gap-4 py-12">
                    <Suspense fallback=|| view! { <ui::Skeleton class="h-56 w-full"/> }>
                        {move || Suspend::new(async move {
                            match me.await {
                                Ok(Some(me)) => view! { <AlreadyIn me next=next()/> }.into_any(),
                                _ => view! { <SignInForm next=next()/> }.into_any(),
                            }
                        })}
                    </Suspense>
                </div>
                <a href="/" class="self-start text-sm text-base-content/60 hover:text-base-content">"← Volver al inicio"</a>
            </div>
            <SidePanel/>
        </div>
    }
}

/// La mitad oscura de Entrar: una tienda de muestra y una frase.
#[component]
fn SidePanel() -> impl IntoView {
    view! {
        <div class="relative hidden overflow-hidden bg-[#110e1c] p-12 text-white lg:flex lg:flex-col lg:justify-end" aria-hidden="true">
            <div class="pointer-events-none absolute -top-48 -right-48 size-[600px] rounded-full bg-[radial-gradient(closest-side,rgba(106,85,240,.6),transparent)]"></div>
            <div class="absolute top-16 right-14 left-14 -rotate-2 rounded-2xl bg-white p-4 text-[#16131f] shadow-2xl">
                <div class="mb-3 flex justify-between text-sm font-extrabold"><span>"Huerta Nativa"</span><span>"Carrito · 2"</span></div>
                <div class="flex h-28 items-end rounded-xl bg-gradient-to-r from-[#203a2c] to-[#3e6b52] p-4 text-xl font-extrabold text-white">"Plantas que duran"</div>
                <div class="mt-3 grid grid-cols-4 gap-2">
                    {[4, 2, 3, 6].into_iter().map(|n| view! {
                        <img src=format!("/img/product-{n}.webp") alt="" class="aspect-square w-full rounded-lg object-cover"/>
                    }).collect_view()}
                </div>
            </div>
            <p class="relative text-2xl leading-snug font-bold tracking-tight">"Tu tienda, tu dirección,"<br/>"tus ventas."</p>
            <p class="relative mt-2 text-white/60">"tunombre.mkr.com.ar"</p>
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
        "Ya entraste".to_string()
    } else {
        format!("Hola, {}", me.name)
    };
    view! {
        <h1 class="text-3xl font-extrabold tracking-tight">{hello}</h1>
        <p>"Estás con la cuenta " <strong>{me.email.clone()}</strong> "."</p>
        {(!me.can_edit && !platform).then(|| view! {
            <div role="alert" class="alert alert-warning">"Tu cuenta no puede editar esta tienda."</div>
        })}
        <div class="flex flex-wrap gap-2">
            {(me.can_edit || platform).then(|| view! { <a class="btn btn-primary" href=next.clone()>"Seguir"</a> })}
            {platform.then(|| view! { <a class="btn btn-ghost" href="/cuenta">"Mi cuenta"</a> })}
            {(!platform).then(|| view! { <a class="btn btn-ghost" href="/">"Ver la tienda"</a> })}
            <button class="btn btn-ghost" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
        </div>
    }
}

/// Campo de contraseña con "Mostrar" y, si `meter`, la barrita de qué tan
/// buena es.
#[component]
fn PasswordField(
    value: RwSignal<String>,
    #[prop(into)] label: String,
    #[prop(into)] autocomplete: String,
    #[prop(optional)] meter: bool,
    #[prop(optional)] autofocus: bool,
    #[prop(optional)] extra: Option<AnyView>,
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
            <div class="input flex w-full items-center gap-2 pr-1">
                <input id=id type=move || if show.get() { "text" } else { "password" }
                    class="grow" required minlength=domain::PASSWORD_MIN.to_string() maxlength=domain::PASSWORD_MAX.to_string()
                    autocomplete=autocomplete data-autofocus=autofocus.then_some("")
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))/>
                <button type="button" class="btn btn-ghost btn-xs text-base-content/60"
                    aria-pressed=move || show.get().to_string()
                    on:click=move |_| show.update(|s| *s = !*s)>
                    {move || if show.get() { "Ocultar" } else { "Mostrar" }}
                </button>
            </div>
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
                <p class="text-xs text-base-content/70" aria-live="polite">
                    {move || match score() {
                        0 => format!("{} caracteres o más.", domain::PASSWORD_MIN),
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

#[component]
fn SignInForm(next: String) -> impl IntoView {
    let next = StoredValue::new(next);
    let step = RwSignal::new(Step::Email);
    let email = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let code = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let info = RwSignal::new(None::<String>);
    // Para qué es el próximo código que se pide.
    let code_for = RwSignal::new(CodeFor::Login);

    let lookup = Action::new(|email: &String| lookup_email(email.clone()));
    let login = Action::new(|(e, p, n): &(String, String, String)| {
        sign_in(e.clone(), p.clone(), n.clone())
    });
    let ask = Action::new(|email: &String| request_code(email.clone()));
    let check = Action::new(|(e, c, n): &(String, String, String)| {
        check_code(e.clone(), c.clone(), n.clone())
    });
    let finish = Action::new(
        |(e, c, nm, p, n): &(String, String, String, String, String)| {
            set_password_with_code(e.clone(), c.clone(), nm.clone(), p.clone(), n.clone())
        },
    );
    let create = Action::new(|(e, nm, p, n): &(String, String, String, String)| {
        create_account(e.clone(), nm.clone(), p.clone(), n.clone())
    });
    let busy = move || {
        create.pending().get()
            || lookup.pending().get()
            || login.pending().get()
            || ask.pending().get()
            || check.pending().get()
            || finish.pending().get()
    };
    // Al escribir de nuevo, el aviso de error anterior se va.
    Effect::new(move |prev: Option<()>| {
        password.track();
        code.track();
        if prev.is_some() {
            problem.set(None);
        }
    });
    let goto = move |s: Step| {
        problem.set(None);
        step.set(s);
        focus_first();
    };
    let send_code = move |purpose: CodeFor| {
        code_for.set(purpose);
        problem.set(None);
        ask.dispatch(email.get_untracked());
    };

    Effect::new(move |_| {
        let Some(result) = lookup.value().get() else {
            return;
        };
        match result {
            Ok(EmailLookup::Existing {
                name: n,
                has_password: true,
            }) => {
                name.set(n);
                password.set(String::new());
                goto(Step::Password);
            }
            Ok(EmailLookup::Existing {
                name: n,
                has_password: false,
            }) => {
                name.set(n);
                info.set(Some("Tu cuenta todavía no tiene contraseña. Te mandamos un código para que elijas una; después entrás siempre con ella.".into()));
                send_code(CodeFor::Reset);
            }
            Ok(EmailLookup::New) => {
                password.set(String::new());
                goto(Step::Create);
            }
            Ok(EmailLookup::BadEmail) => problem.set(Some(
                "El email no es válido. Revisalo (por ejemplo, ana@correo.com).".into(),
            )),
            Ok(EmailLookup::NotAllowed) => problem.set(Some(NOT_ALLOWED.into())),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });
    Effect::new(move |_| {
        let Some(result) = login.value().get() else {
            return;
        };
        match result {
            Ok(PasswordSignIn::In { next }) => go(&next),
            Ok(PasswordSignIn::Wrong) => problem.set(Some("La contraseña no es correcta. Probá de nuevo o recuperala con \"¿Olvidaste tu contraseña?\".".into())),
            Ok(PasswordSignIn::Locked { minutes }) => problem.set(Some(format!(
                "Hubo demasiados intentos seguidos. Esperá {minutes} minutos o entrá con un código por email."
            ))),
            Ok(PasswordSignIn::NoPassword) => send_code(CodeFor::Reset),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });
    Effect::new(move |_| {
        let Some(result) = ask.value().get() else {
            return;
        };
        match result {
            Ok(CodeRequest::Sent) => {
                code.set(String::new());
                if code_for.get_untracked() == CodeFor::Reset {
                    password.set(String::new());
                }
                goto(Step::Code(code_for.get_untracked()));
            }
            Ok(CodeRequest::BadEmail) => {
                problem.set(Some("El email no es válido. Revisalo.".into()))
            }
            Ok(CodeRequest::NotAllowed) => problem.set(Some(NOT_ALLOWED.into())),
            Ok(CodeRequest::TooMany) => problem.set(Some(
                "Se pidieron demasiados códigos seguidos. Esperá unos minutos y probá de nuevo."
                    .into(),
            )),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });
    let wrong_code = move |left: u32| {
        if left == 1 {
            "El código no es correcto. Te queda 1 intento.".to_string()
        } else {
            format!("El código no es correcto. Te quedan {left} intentos.")
        }
    };
    let expired = "El código venció o ya no es válido. Pedí uno nuevo.";
    Effect::new(move |_| {
        let Some(result) = check.value().get() else {
            return;
        };
        match result {
            Ok(CodeCheck::In { next }) => go(&next),
            Ok(CodeCheck::Wrong { left }) => problem.set(Some(wrong_code(left))),
            Ok(CodeCheck::Expired) => problem.set(Some(expired.into())),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });
    Effect::new(move |_| {
        let Some(result) = create.value().get() else {
            return;
        };
        match result {
            Ok(AccountCreation::In { next }) => go(&next),
            Ok(AccountCreation::NeedsCode) => send_code(CodeFor::NewAccount),
            Ok(AccountCreation::Exists) => {
                password.set(String::new());
                goto(Step::Password);
                info.set(Some(
                    "Este email ya tiene una cuenta. Entrá con tu contraseña.".into(),
                ));
            }
            Ok(AccountCreation::Weak { reason }) => problem.set(Some(reason)),
            Ok(AccountCreation::NotAllowed) => problem.set(Some(NOT_ALLOWED.into())),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });
    Effect::new(move |_| {
        let Some(result) = finish.value().get() else {
            return;
        };
        match result {
            Ok(SetPassword::In { next }) => go(&next),
            Ok(SetPassword::Wrong { left }) => problem.set(Some(wrong_code(left))),
            Ok(SetPassword::Expired) => problem.set(Some(expired.into())),
            Ok(SetPassword::Weak { reason }) => problem.set(Some(reason)),
            Ok(SetPassword::NotAllowed) => problem.set(Some(NOT_ALLOWED.into())),
            Err(_) => problem.set(Some(CONNECTION.into())),
        }
    });

    let alert = move || {
        problem
            .get()
            .map(|p| view! { <FieldNote ok=false text=p/> })
    };
    let shown_email = move || email.get().trim().to_lowercase();
    // El email elegido, con "Cambiar" para volver al primer paso.
    let chip = move |initial: bool| {
        view! {
            <div class="flex items-center gap-3 rounded-xl bg-base-200 px-3 py-2.5 text-sm">
                <span class=if initial { "grid size-8 shrink-0 place-items-center rounded-full bg-gradient-to-br from-[#ffb547] to-[#ff7a59] font-bold text-white" } else { "grid size-8 shrink-0 place-items-center rounded-full bg-base-300 font-bold text-base-content/60" } aria-hidden="true">
                    {move || if initial { name.get().chars().next().or(shown_email().chars().next()).unwrap_or('?').to_uppercase().to_string() } else { "?".to_string() }}
                </span>
                <span class="min-w-0 grow truncate">{shown_email}</span>
                <button type="button" class="link link-primary font-semibold no-underline" on:click=move |_| { info.set(None); goto(Step::Email) }>"Cambiar"</button>
            </div>
        }
    };
    let submit_btn = move |label: &'static str, busy_label: &'static str| {
        view! {
            <button class="btn btn-primary h-12" type="submit" disabled=busy>
                {move || if busy() { busy_label } else { label }}
            </button>
        }
    };

    move || {
        match step.get() {
        Step::Email => view! {
            <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                problem.set(None);
                info.set(None);
                lookup.dispatch(email.get_untracked());
            }>
                <h1 class="text-3xl font-extrabold tracking-tight">"Entrá o creá tu cuenta"</h1>
                <p class="-mt-2 text-base-content/70">"Con el mismo email para las dos cosas."</p>
                <label class="flex flex-col gap-1.5">
                    <span class="text-sm font-semibold">"Email"</span>
                    <input type="email" name="email" required autocomplete="email" data-autofocus=""
                        class="input w-full" placeholder="ana@correo.com"
                        prop:value=move || email.get()
                        on:input=move |ev| email.set(event_target_value(&ev))/>
                </label>
                {alert}
                {submit_btn("Seguir", "Buscando…")}
                <p class="text-xs text-base-content/60">"Para comprar en una tienda no hace falta cuenta. Esto es para quien arma y maneja su tienda."</p>
            </form>
        }.into_any(),
        Step::Password => view! {
            <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                problem.set(None);
                login.dispatch((email.get_untracked(), password.get_untracked(), next.get_value()));
            }>
                <h1 class="text-3xl font-extrabold tracking-tight">
                    {move || if name.get().is_empty() { "Hola de nuevo".to_string() } else { format!("Hola de nuevo, {}", name.get()) }}
                </h1>
                {chip(true)}
                {move || info.get().map(|i| view! { <p role="status" class="-mt-1 text-sm text-base-content/70">{i}</p> })}
                <PasswordField value=password label="Contraseña" autocomplete="current-password" autofocus=true
                    extra=view! {
                        <button type="button" class="link link-primary text-sm font-semibold no-underline" disabled=busy
                            on:click=move |_| send_code(CodeFor::Reset)>"¿Olvidaste tu contraseña?"</button>
                    }.into_any()/>
                {alert}
                {submit_btn("Entrar", "Entrando…")}
                <button type="button" class="link text-center text-sm text-base-content/70 no-underline hover:text-base-content" disabled=busy
                    on:click=move |_| send_code(CodeFor::Login)>"Iniciar sesión con un código por email"</button>
            </form>
        }.into_any(),
        Step::Create => view! {
            <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                ev.prevent_default();
                if let Some(reason) = domain::password_problem(&password.get_untracked()) {
                    problem.set(Some(reason));
                    return;
                }
                problem.set(None);
                create.dispatch((email.get_untracked(), name.get_untracked(), password.get_untracked(), next.get_value()));
            }>
                <h1 class="text-3xl font-extrabold tracking-tight">"Creá tu cuenta"</h1>
                {chip(false)}
                <label class="flex flex-col gap-1.5">
                    <span class="text-sm font-semibold">"¿Cómo te llamás?"</span>
                    <input type="text" name="name" autocomplete="given-name" data-autofocus=""
                        maxlength=domain::ACCOUNT_NAME_MAX.to_string()
                        class="input w-full" placeholder="Ana"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))/>
                </label>
                <PasswordField value=password label="Elegí una contraseña" autocomplete="new-password" meter=true/>
                {alert}
                {submit_btn("Crear cuenta", "Creando cuenta…")}
                <p class="text-xs text-base-content/60">"Al crear la cuenta aceptás los términos y la privacidad."</p>
            </form>
        }.into_any(),
        Step::Code(purpose) => {
            let (title, button) = match purpose {
                CodeFor::NewAccount => ("Confirmá tu email", "Crear cuenta"),
                CodeFor::Reset => ("Elegí tu contraseña", "Guardar y entrar"),
                CodeFor::Login => ("Revisá tu email", "Entrar"),
            };
            view! {
                <form class="flex flex-col gap-4" on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    problem.set(None);
                    let (e, c, n) = (email.get_untracked(), code.get_untracked(), next.get_value());
                    match purpose {
                        CodeFor::Login => { check.dispatch((e, c, n)); }
                        _ => { finish.dispatch((e, c, name.get_untracked(), password.get_untracked(), n)); }
                    }
                }>
                    <h1 class="text-3xl font-extrabold tracking-tight">{title}</h1>
                    {move || info.get().map(|i| view! { <p role="status" class="text-sm text-base-content/70">{i}</p> })}
                    <p class="text-base-content/80">
                        "Te mandamos un código de 6 números a " <strong>{shown_email}</strong> ". Vence en 10 minutos."
                    </p>
                    {cfg!(debug_assertions).then(|| view! {
                        <p class="text-sm text-base-content/70">"En desarrollo el código no se manda por email: aparece en la terminal donde corre el servidor."</p>
                    })}
                    <label class="flex flex-col gap-1.5">
                        <span class="text-sm font-semibold">"Código"</span>
                        <input type="text" name="code" required data-autofocus=""
                            inputmode="numeric" autocomplete="one-time-code" maxlength="7"
                            class="input w-full font-mono text-2xl tracking-[0.4em]" placeholder="000000"
                            prop:value=move || code.get()
                            on:input=move |ev| code.set(event_target_value(&ev))/>
                    </label>
                    {(purpose == CodeFor::Reset).then(|| view! {
                        <PasswordField value=password label="Contraseña nueva" autocomplete="new-password" meter=true/>
                    })}
                    {alert}
                    {submit_btn(button, "Revisando…")}
                    <div class="flex flex-wrap gap-2 text-sm">
                        <button type="button" class="btn btn-ghost btn-sm" disabled=busy
                            on:click=move |_| send_code(purpose)>"Mandame otro código"</button>
                        <button type="button" class="btn btn-ghost btn-sm"
                            on:click=move |_| { info.set(None); goto(Step::Email) }>"Usar otro email"</button>
                    </div>
                </form>
            }.into_any()
        }
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
            <div class="mx-auto flex max-w-3xl flex-col gap-4 px-8 py-10">
                <div>
                    <h1 class="text-3xl font-extrabold tracking-tight">"Mi cuenta"</h1>
                    <p class="mt-1 text-base-content/70">"Tus datos para entrar a MarketMaker. Valen para todas tus tiendas."</p>
                </div>
                <Suspense fallback=|| view! { <ui::Skeleton class="h-80 w-full"/> }>
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
        <div class="flex items-center gap-4 border-t border-base-300 px-6 py-4">
            <ui::Icon class="size-[18px] shrink-0 text-base-content/60" name=icon/>
            <div class="w-32 shrink-0 text-sm font-semibold">{label}</div>
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
                action=view! { <span class="text-xs text-base-content/60">"No se puede cambiar"</span> }.into_any()>
                <div class="truncate">{account.email.clone()}</div>
                <div class="text-xs text-base-content/60">"Con este email entrás."</div>
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
        </section>
        <NameDialog dialog=name_dialog saved_name/>
        <PasswordDialog dialog=pw_dialog has_password notice/>
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
        Some(Ok(PasswordChange::Weak { reason })) => problem.set(Some(reason)),
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
