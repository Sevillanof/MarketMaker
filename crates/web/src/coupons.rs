//! Cupones de una tienda, en el panel (`/tienda/:slug/cupones`): la lista y
//! el formulario para crear o cambiar uno.

use crate::products::{error_text, slug_param, SignInFirst};
use domain::{Coupon, CouponForm, CouponKind, CouponSaved, CouponStatus, CouponsInfo, FieldError};
use leptos::prelude::*;

#[server]
pub async fn store_coupons(slug: String) -> Result<Option<CouponsInfo>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store_min(&slug, "admin").await? else {
        return Ok(None);
    };
    let pool = pool()?;
    Ok(Some(CouponsInfo {
        store_name: tenant.name,
        today: db::coupons::today(&pool).await.map_err(internal)?,
        coupons: db::coupons::list(&pool, tenant.id)
            .await
            .map_err(internal)?,
    }))
}

/// Crea o cambia un cupón. Devuelve los problemas de cada campo o la lista
/// nueva.
#[server]
pub async fn save_coupon(slug: String, form: CouponForm) -> Result<CouponSaved, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store_min(&slug, "admin").await?;
    let pool = pool()?;
    let coupon = match domain::tidy_coupon(&form) {
        Ok(c) => c,
        Err(errors) => {
            return Ok(CouponSaved {
                errors,
                coupons: Vec::new(),
            })
        }
    };
    let problem = |field: &str, message: &str| {
        Ok(CouponSaved {
            errors: vec![FieldError {
                field: field.into(),
                message: message.into(),
            }],
            coupons: Vec::new(),
        })
    };
    // Una tienda no necesita miles de cupones.
    if coupon.id.is_empty()
        && db::coupons::list(&pool, tenant.id)
            .await
            .map_err(internal)?
            .len()
            >= 200
    {
        return problem(
            "code",
            "Llegaste al máximo de 200 cupones. Borrá alguno viejo.",
        );
    }
    match db::coupons::save(&pool, tenant.id, &coupon)
        .await
        .map_err(internal)?
    {
        db::coupons::Saved::Saved => {}
        db::coupons::Saved::CodeTaken => {
            return problem("code", "Ya tenés un cupón con ese código. Elegí otro.")
        }
        db::coupons::Saved::Missing => {
            return Err(ServerFnError::new(
                "Ese cupón ya no existe. Recargá la página.",
            ))
        }
    }
    Ok(CouponSaved {
        errors: Vec::new(),
        coupons: db::coupons::list(&pool, tenant.id)
            .await
            .map_err(internal)?,
    })
}

#[server]
pub async fn set_coupon_active(
    slug: String,
    id: String,
    active: bool,
) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store_min(&slug, "admin").await?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Cupón inválido."))?;
    db::coupons::set_active(&pool()?, tenant.id, id, active)
        .await
        .map_err(internal)?;
    Ok(())
}

#[server]
pub async fn delete_coupon(slug: String, id: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store_min(&slug, "admin").await?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Cupón inválido."))?;
    db::coupons::delete(&pool()?, tenant.id, id)
        .await
        .map_err(internal)?;
    Ok(())
}

/// `/tienda/:slug/cupones`.
#[component]
pub fn CouponsPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), store_coupons);
    view! {
        <leptos_meta::Title text="Cupones · MarketMaker"/>
        <div class="flex w-full max-w-[1440px] flex-col gap-5 px-8 max-sm:px-4 py-8 max-sm:py-5">
            <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Coupons/> }>
                {move || Suspend::new(async move {
                    match data.await {
                        Ok(Some(info)) => view! { <Coupons slug=slug.get_untracked() info/> }.into_any(),
                        Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/cupones", slug.get_untracked())/> }.into_any(),
                        Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
    .into_any()
}

fn error_of(errors: RwSignal<Vec<FieldError>>, field: &str) -> Option<String> {
    errors.with(|e| {
        e.iter()
            .find(|e| e.field == field)
            .map(|e| e.message.clone())
    })
}

/// Cuándo vence y desde cuánto sirve, en una línea.
fn condition_text(c: &Coupon, today: &str) -> String {
    let min = if c.min_cents > 0 {
        format!("Desde {}", domain::peso_text(c.min_cents))
    } else {
        "Sin mínimo".to_string()
    };
    match (c.expires.is_empty(), c.status(today)) {
        (true, _) => format!("{min} · no vence"),
        (false, CouponStatus::Expired) => {
            format!("{min} · venció el {}", domain::date_text(&c.expires))
        }
        (false, _) => format!("{min} · vence el {}", domain::date_text(&c.expires)),
    }
}

fn uses_text(c: &Coupon) -> String {
    if c.max_uses > 0 {
        format!("{} de {}", c.uses, c.max_uses)
    } else {
        c.uses.to_string()
    }
}

#[component]
fn Coupons(slug: String, info: CouponsInfo) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let today = StoredValue::new(info.today);
    let coupons = RwSignal::new(info.coupons);
    let problem = RwSignal::new(None::<String>);
    // Borrar: pide confirmar en la misma fila.
    let confirming = RwSignal::new(None::<String>);

    // El formulario.
    let id = RwSignal::new(String::new());
    let code = RwSignal::new(String::new());
    let kind = RwSignal::new(CouponKind::Percent);
    let value = RwSignal::new(String::new());
    let min = RwSignal::new(String::new());
    let expires = RwSignal::new(String::new());
    let max_uses = RwSignal::new(String::new());
    let errors = RwSignal::new(Vec::<FieldError>::new());
    let saved_note = RwSignal::new(None::<String>);
    let editing = move || !id.with(|i| i.is_empty());
    let clear = move || {
        id.set(String::new());
        code.set(String::new());
        kind.set(CouponKind::Percent);
        value.set(String::new());
        min.set(String::new());
        expires.set(String::new());
        max_uses.set(String::new());
        errors.set(Vec::new());
    };
    let load = move |c: &Coupon| {
        let f = c.to_form();
        id.set(f.id);
        code.set(f.code);
        kind.set(f.kind);
        value.set(f.value);
        min.set(f.min);
        expires.set(f.expires);
        max_uses.set(f.max_uses);
        errors.set(Vec::new());
        saved_note.set(None);
        crate::checkout::focus_id("cp-code".into());
    };

    let save = Action::new(move |f: &CouponForm| save_coupon(slug.get_value(), f.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(r)) if r.errors.is_empty() => {
            // Sin seguir a `id`: `clear()` lo vuelve a escribir y, si este
            // efecto dependiera de él, se dispararía a sí mismo sin parar.
            let was_editing = !id.with_untracked(|i| i.is_empty());
            coupons.set(r.coupons);
            clear();
            saved_note.set(Some(
                if was_editing {
                    "✓ Cambios guardados"
                } else {
                    "✓ Cupón creado"
                }
                .into(),
            ));
        }
        Some(Ok(r)) => {
            if let Some(first) = r.errors.first() {
                let target = match first.field.as_str() {
                    "max_uses" => "cp-max".to_string(),
                    f => format!("cp-{f}"),
                };
                crate::checkout::focus_id(target);
            }
            errors.set(r.errors);
        }
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let submit = move || {
        problem.set(None);
        saved_note.set(None);
        errors.set(Vec::new());
        save.dispatch(CouponForm {
            id: id.get_untracked(),
            code: code.get_untracked(),
            kind: kind.get_untracked(),
            value: value.get_untracked(),
            min: min.get_untracked(),
            expires: expires.get_untracked(),
            max_uses: max_uses.get_untracked(),
        });
    };

    let toggle = Action::new(move |(cid, on): &(String, bool)| {
        set_coupon_active(slug.get_value(), cid.clone(), *on)
    });
    Effect::new(move |_| {
        if let Some(Err(e)) = toggle.value().get() {
            problem.set(Some(error_text(&e)));
        }
    });
    let remove = Action::new(move |cid: &String| delete_coupon(slug.get_value(), cid.clone()));
    Effect::new(move |_| {
        if let Some(Err(e)) = remove.value().get() {
            problem.set(Some(error_text(&e)));
        }
    });

    let field_class = |has_error: bool| {
        if has_error {
            "input w-full input-error"
        } else {
            "input w-full"
        }
    };
    view! {
        <div>
            <h1 class="text-3xl font-bold max-sm:text-2xl">"Cupones"</h1>
            <p class="text-base-content/70">{format!("{} · códigos de descuento para tus compradores", info.store_name)}</p>
        </div>
        {move || problem.get().map(|p| view! { <p role="alert" class="text-sm text-error">{p}</p> })}
        <section aria-labelledby="cp-list" class="card bg-base-100 shadow-sm">
            <div class="card-body gap-3 p-6 max-sm:p-4">
                <div class="flex items-center gap-3">
                    <h2 id="cp-list" class="grow text-lg font-bold">"Tus cupones"</h2>
                    // Con el formulario siempre abajo, el botón solo sirve para
                    // volver a «Nuevo cupón» mientras se edita uno.
                    {move || editing().then(|| view! {
                        <button type="button" class="btn btn-primary btn-sm"
                            on:click=move |_| { clear(); saved_note.set(None); crate::checkout::focus_id("cp-code".into()); }>
                            "+ Nuevo cupón"
                        </button>
                    })}
                </div>
                {move || coupons.with(|c| c.is_empty()).then(|| view! {
                    <p class="py-6 text-center text-base-content/70">
                        "Todavía no creaste cupones. Armá el primero abajo y compartí su código con tus compradores."
                    </p>
                })}
                <div role="region" tabindex="0" aria-labelledby="cp-list" class="overflow-x-auto"
                    hidden=move || coupons.with(|c| c.is_empty())>
                    <table class="table min-w-[44rem]">
                        <thead>
                            <tr>
                                <th scope="col">"Código"</th>
                                <th scope="col">"Descuento"</th>
                                <th scope="col">"Condición"</th>
                                <th scope="col">"Usos"</th>
                                <th scope="col">"Estado"</th>
                                <th scope="col"><span class="sr-only">"Acciones"</span></th>
                            </tr>
                        </thead>
                        <tbody>
                            <For each=move || coupons.get() key=|c| (c.id.clone(), c.active, c.uses, c.value, c.min_cents, c.expires.clone(), c.max_uses, c.code.clone()) let(c)>
                                {
                                    let status = c.status(&today.get_value());
                                    let cid = c.id.clone();
                                    let cid_toggle = c.id.clone();
                                    let cid_del = c.id.clone();
                                    let cid_ask = c.id.clone();
                                    let cid_ask2 = c.id.clone();
                                    let edit_c = c.clone();
                                    let label = format!("Activar el cupón {}", c.code);
                                    view! {
                                        <tr class="align-middle">
                                            <th scope="row" class="font-normal"><span class="rounded-md bg-primary/10 px-2 py-1 font-mono font-bold tracking-wide text-primary">{c.code.clone()}</span></th>
                                            <td class="font-semibold">{c.value_text()}</td>
                                            <td class="text-sm text-base-content/75">{condition_text(&c, &today.get_value())}</td>
                                            <td class="tabular-nums">{uses_text(&c)}</td>
                                            <td>
                                                {match status {
                                                    CouponStatus::Expired => view! { <span class="badge badge-error badge-soft">"Venció"</span> }.into_any(),
                                                    CouponStatus::Exhausted => view! { <span class="badge badge-warning badge-soft">"Sin usos"</span> }.into_any(),
                                                    _ => view! {
                                                        <input type="checkbox" role="switch" class="toggle toggle-primary toggle-sm"
                                                            aria-label=label checked=c.active
                                                            on:change=move |ev| {
                                                                let on = event_target_checked(&ev);
                                                                coupons.update(|l| if let Some(x) = l.iter_mut().find(|x| x.id == cid) { x.active = on; });
                                                                toggle.dispatch((cid_toggle.clone(), on));
                                                            }/>
                                                    }.into_any(),
                                                }}
                                            </td>
                                            <td class="text-right whitespace-nowrap">
                                                {move || if confirming.get().as_deref() == Some(cid_ask.as_str()) {
                                                    let cid_del = cid_del.clone();
                                                    view! {
                                                        <span class="text-sm">"¿Borrar? "</span>
                                                        <button type="button" class="btn btn-error btn-xs"
                                                            on:click=move |_| {
                                                                let d = cid_del.clone();
                                                                coupons.update(|l| l.retain(|x| x.id != d));
                                                                confirming.set(None);
                                                                remove.dispatch(d);
                                                            }>"Sí, borrar"</button>
                                                        <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| confirming.set(None)>"No"</button>
                                                    }.into_any()
                                                } else {
                                                    let edit_c = edit_c.clone();
                                                    let ask = cid_ask2.clone();
                                                    view! {
                                                        <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| load(&edit_c)>"Editar"</button>
                                                        <button type="button" class="btn btn-ghost btn-xs text-error" on:click=move |_| confirming.set(Some(ask.clone()))>"Borrar"</button>
                                                    }.into_any()
                                                }}
                                            </td>
                                        </tr>
                                    }
                                }
                            </For>
                        </tbody>
                    </table>
                </div>
            </div>
        </section>
        <section aria-labelledby="cp-form" class="card bg-base-100 shadow-sm">
            <form class="card-body gap-4 p-6 max-sm:p-4" novalidate
                on:submit=move |ev| { ev.prevent_default(); submit(); }>
                <h2 id="cp-form" class="text-lg font-bold">
                    {move || if editing() { "Editar cupón" } else { "Nuevo cupón" }}
                </h2>
                <div class="grid gap-x-6 gap-y-2 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">
                        <fieldset class="fieldset py-0">
                            <label class="fieldset-legend" for="cp-code">"Código"</label>
                            <input id="cp-code" name="code" class=move || field_class(error_of(errors, "code").is_some())
                                autocomplete="off" autocapitalize="characters" spellcheck="false" maxlength="30"
                                placeholder="VERANO15"
                                aria-invalid=move || error_of(errors, "code").is_some().to_string()
                                aria-describedby="cp-code-help cp-code-error"
                                prop:value=move || code.get()
                                on:input=move |ev| code.set(event_target_value(&ev))/>
                            <p id="cp-code-help" class="label whitespace-normal">"Sin espacios ni tildes. El comprador lo escribe al pagar."</p>
                            {move || error_of(errors, "code").map(|m| view! { <p id="cp-code-error" class="label text-error">{m}</p> })}
                        </fieldset>
                        <fieldset class="fieldset py-0">
                            <label class="fieldset-legend" for="cp-value">"Descuento"</label>
                            <div class="flex gap-2">
                                <input id="cp-value" name="value" class=move || format!("{} min-w-0 flex-1", field_class(error_of(errors, "value").is_some()))
                                    autocomplete="off" inputmode="decimal" maxlength="12"
                                    placeholder=move || if kind.get() == CouponKind::Percent { "15" } else { "5.000" }
                                    aria-invalid=move || error_of(errors, "value").is_some().to_string()
                                    aria-describedby="cp-value-error"
                                    prop:value=move || value.get()
                                    on:input=move |ev| value.set(event_target_value(&ev))/>
                                <select class="select w-28 shrink-0" aria-label="Tipo de descuento"
                                    on:change=move |ev| kind.set(if event_target_value(&ev) == "amount" { CouponKind::Amount } else { CouponKind::Percent })>
                                    <option value="percent" selected=move || kind.get() == CouponKind::Percent>"%"</option>
                                    <option value="amount" selected=move || kind.get() == CouponKind::Amount>"$ fijo"</option>
                                </select>
                            </div>
                            {move || error_of(errors, "value").map(|m| view! { <p id="cp-value-error" class="label text-error">{m}</p> })}
                        </fieldset>
                    </div>
                    <div class="flex flex-col gap-2">
                        <fieldset class="fieldset py-0">
                            <label class="fieldset-legend" for="cp-min">"Compra mínima "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                            <input id="cp-min" name="min" class=move || field_class(error_of(errors, "min").is_some())
                                autocomplete="off" inputmode="decimal" maxlength="12" placeholder="20.000"
                                aria-invalid=move || error_of(errors, "min").is_some().to_string()
                                aria-describedby="cp-min-error"
                                prop:value=move || min.get()
                                on:input=move |ev| min.set(event_target_value(&ev))/>
                            {move || error_of(errors, "min").map(|m| view! { <p id="cp-min-error" class="label text-error">{m}</p> })}
                        </fieldset>
                        <fieldset class="fieldset py-0">
                            <label class="fieldset-legend" for="cp-expires">"Vence "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                            <input id="cp-expires" name="expires" type="date" class=move || field_class(error_of(errors, "expires").is_some())
                                aria-invalid=move || error_of(errors, "expires").is_some().to_string()
                                aria-describedby="cp-expires-error"
                                prop:value=move || expires.get()
                                on:input=move |ev| expires.set(event_target_value(&ev))/>
                            {move || error_of(errors, "expires").map(|m| view! { <p id="cp-expires-error" class="label text-error">{m}</p> })}
                        </fieldset>
                        <fieldset class="fieldset py-0">
                            <label class="fieldset-legend" for="cp-max">"Usos máximos "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                            <input id="cp-max" name="max_uses" class=move || field_class(error_of(errors, "max_uses").is_some())
                                autocomplete="off" inputmode="numeric" maxlength="6" placeholder="100"
                                aria-invalid=move || error_of(errors, "max_uses").is_some().to_string()
                                aria-describedby="cp-max-error"
                                prop:value=move || max_uses.get()
                                on:input=move |ev| max_uses.set(event_target_value(&ev))/>
                            {move || error_of(errors, "max_uses").map(|m| view! { <p id="cp-max-error" class="label text-error">{m}</p> })}
                        </fieldset>
                    </div>
                </div>
                <div class="flex flex-wrap items-center gap-3">
                    <button type="submit" class="btn btn-primary" disabled=move || save.pending().get()>
                        {move || match (save.pending().get(), editing()) {
                            (true, _) => "Guardando…",
                            (false, true) => "Guardar cambios",
                            (false, false) => "Crear cupón",
                        }}
                    </button>
                    {move || editing().then(|| view! {
                        <button type="button" class="btn btn-ghost" on:click=move |_| clear()>"Cancelar"</button>
                    })}
                    <span role="status" class="text-sm font-semibold text-success">{move || saved_note.get()}</span>
                </div>
            </form>
        </section>
    }
}
