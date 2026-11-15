//! Pantallas de la cuenta del dueño: Entrar (`/entrar`) y la barra
//! "Editar mi tienda" que el dueño ve arriba de su tienda.

use crate::auth::{check_code, request_code, sign_out};
use domain::{CodeCheck, CodeRequest, Me};
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

/// Va a `path` recargando la página, para que el servidor vea la sesión nueva.
fn go(path: &str) {
    let _ = window().location().set_href(path);
}

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Email,
    Code,
}

/// `/entrar`: email → código de 6 dígitos → adentro. Sin contraseña.
#[component]
pub fn SignInPage() -> impl IntoView {
    let query = use_query_map();
    let next = move || domain::safe_next(&query.read().get("volver").unwrap_or_default());
    let me = Resource::new(|| (), |_| crate::auth::who_am_i());

    view! {
        <leptos_meta::Title text="Entrar · MarketMaker"/>
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content flex items-start justify-center px-4 py-16">
            <div class="card bg-base-100 shadow-sm w-full max-w-md">
                <div class="card-body gap-4">
                    <Suspense fallback=|| view! { <ui::Skeleton class="h-40 w-full"/> }>
                        {move || Suspend::new(async move {
                            match me.await {
                                Ok(Some(me)) => view! { <AlreadyIn me next=next()/> }.into_any(),
                                _ => view! { <SignInForm next=next()/> }.into_any(),
                            }
                        })}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

#[component]
fn AlreadyIn(me: Me, next: String) -> impl IntoView {
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/entrar");
        }
    });
    view! {
        <h1 class="card-title text-2xl">"Ya entraste"</h1>
        <p>"Estás con la cuenta " <strong>{me.email.clone()}</strong> "."</p>
        {(!me.can_edit).then(|| view! {
            <div role="alert" class="alert alert-warning">"Tu cuenta no puede editar esta tienda."</div>
        })}
        <div class="flex flex-wrap gap-2">
            {me.can_edit.then(|| view! { <a class="btn btn-primary" href=next.clone()>"Seguir"</a> })}
            <a class="btn btn-ghost" href="/">"Ver la tienda"</a>
            <button class="btn btn-ghost" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
        </div>
    }
}

#[component]
fn SignInForm(next: String) -> impl IntoView {
    let step = RwSignal::new(Step::Email);
    let email = RwSignal::new(String::new());
    let code = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let email_ref = NodeRef::<html::Input>::new();
    let code_ref = NodeRef::<html::Input>::new();

    let ask = Action::new(|email: &String| request_code(email.clone()));
    let check = Action::new(|(email, code, next): &(String, String, String)| {
        check_code(email.clone(), code.clone(), next.clone())
    });
    let busy = move || ask.pending().get() || check.pending().get();
    let connection =
        "No pudimos comunicarnos con el servidor. Revisá tu conexión y probá de nuevo.";

    Effect::new(move |_| {
        let Some(result) = ask.value().get() else {
            return;
        };
        match result {
            Ok(CodeRequest::Sent) => {
                problem.set(None);
                code.set(String::new());
                step.set(Step::Code);
            }
            Ok(CodeRequest::BadEmail) => problem.set(Some("Revisá el email: parece que le falta algo (por ejemplo, ana@correo.com).".into())),
            Ok(CodeRequest::NotAllowed) => problem.set(Some("Por ahora MarketMaker es solo por invitación y este email no está en la lista. Si creés que es un error, avisanos.".into())),
            Ok(CodeRequest::TooMany) => problem.set(Some("Pediste muchos códigos seguidos. Esperá un rato y probá de nuevo.".into())),
            Err(_) => problem.set(Some(connection.into())),
        }
    });
    Effect::new(move |_| {
        let Some(result) = check.value().get() else {
            return;
        };
        match result {
            Ok(CodeCheck::In { next }) => go(&next),
            Ok(CodeCheck::Wrong { left }) => problem.set(Some(if left == 1 {
                "Ese código no es. Te queda 1 intento.".into()
            } else {
                format!("Ese código no es. Te quedan {left} intentos.")
            })),
            Ok(CodeCheck::Expired) => problem.set(Some(
                "Ese código venció o ya no sirve. Pedí uno nuevo.".into(),
            )),
            Err(_) => problem.set(Some(connection.into())),
        }
    });
    // Al cambiar de paso, el foco va a la caja nueva.
    Effect::new(move |_| {
        let input = match step.get() {
            Step::Email => email_ref.get(),
            Step::Code => code_ref.get(),
        };
        if let Some(input) = input {
            let _ = input.focus();
        }
    });

    let on_email = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        problem.set(None);
        ask.dispatch(email.get_untracked());
    };
    let next_for_code = next.clone();
    let on_code = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        problem.set(None);
        check.dispatch((
            email.get_untracked(),
            code.get_untracked(),
            next_for_code.clone(),
        ));
    };
    let alert = move || {
        problem
            .get()
            .map(|p| view! { <div role="alert" class="alert alert-error text-sm">{p}</div> })
    };

    view! {
        <Show
            when=move || step.get() == Step::Code
            fallback=move || view! {
                <h1 class="card-title text-2xl">"Entrá a tu tienda"</h1>
                <p class="text-base-content/80">"Escribí tu email y te mandamos un código. No hace falta contraseña."</p>
                <form class="flex flex-col gap-3" on:submit=on_email>
                    <label class="flex flex-col gap-1">
                        <span class="font-medium">"Tu email"</span>
                        <input node_ref=email_ref type="email" name="email" required autocomplete="email"
                            class="input w-full" placeholder="ana@correo.com"
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))/>
                    </label>
                    {alert}
                    <button class="btn btn-primary" type="submit" disabled=busy>
                        {move || if ask.pending().get() { "Mandando…" } else { "Seguir" }}
                    </button>
                </form>
            }
        >
            <h1 class="card-title text-2xl">"Revisá tu email"</h1>
            <p class="text-base-content/80">
                "Te mandamos un código de 6 números a " <strong>{move || email.get().trim().to_lowercase()}</strong> ". Vence en 10 minutos."
            </p>
            {cfg!(debug_assertions).then(|| view! {
                <p class="text-sm text-base-content/70">"En desarrollo el código no se manda por email: aparece en la terminal donde corre el servidor."</p>
            })}
            <form class="flex flex-col gap-3" on:submit=on_code.clone()>
                <label class="flex flex-col gap-1">
                    <span class="font-medium">"Código"</span>
                    <input node_ref=code_ref type="text" name="code" required
                        inputmode="numeric" autocomplete="one-time-code" maxlength="7"
                        class="input w-full text-2xl tracking-[0.4em] font-mono" placeholder="000000"
                        prop:value=move || code.get()
                        on:input=move |ev| code.set(event_target_value(&ev))/>
                </label>
                {alert}
                <button class="btn btn-primary" type="submit" disabled=busy>
                    {move || if check.pending().get() { "Revisando…" } else { "Entrar" }}
                </button>
                <div class="flex flex-wrap gap-2 text-sm">
                    <button type="button" class="btn btn-ghost btn-sm" disabled=busy
                        on:click=move |_| { problem.set(None); ask.dispatch(email.get_untracked()); }>
                        "Mandame otro código"
                    </button>
                    <button type="button" class="btn btn-ghost btn-sm"
                        on:click=move |_| { problem.set(None); step.set(Step::Email); }>
                        "Usar otro email"
                    </button>
                </div>
            </form>
        </Show>
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
                <button class="btn btn-sm btn-ghost text-neutral-content hover:bg-white/10" title="Esconde esta barra hasta que recargues la página"
                    on:click=move |_| hidden.set(true)>"Ver como cliente"</button>
                <button class="btn btn-sm btn-ghost text-neutral-content hover:bg-white/10" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
            </div>
        </Show>
    })
}
