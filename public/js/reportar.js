/* Reportar un problema: botón en cada página, selección de un área con el
   mouse (o el dedo), formulario y envío a /api/reportes.
   Sin dependencias propias: la captura usa modern-screenshot (MIT), que se
   baja recién cuando alguien toca el botón. */
(function () {
  if (window.top !== window || window.__mkbug) return;
  window.__mkbug = true;
  var LIB = '/js/modern-screenshot-4.7.0.js';
  var MIN = 14; // lado mínimo del área, en px

  var css =
    '#mkbug-fab{position:fixed;left:16px;bottom:max(16px,env(safe-area-inset-bottom));z-index:2147482000;display:flex;align-items:center;' +
    'border:0;border-radius:999px;background:#1f2430;color:#fff;font:500 13px/1 system-ui,sans-serif;padding:11px;cursor:pointer;box-shadow:0 6px 18px rgba(0,0,0,.28);' +
    'transition:padding .2s ease,background .2s ease}' +
    '#mkbug-fab span{max-width:0;opacity:0;overflow:hidden;white-space:nowrap;margin-left:0;transition:max-width .25s ease,opacity .2s ease,margin .25s ease}' +
    '#mkbug-fab:hover,#mkbug-fab:focus-visible{background:#2c3342;padding-right:16px}' +
    '#mkbug-fab:hover span,#mkbug-fab:focus-visible span{max-width:160px;opacity:1;margin-left:8px}' +
    '@media (prefers-reduced-motion:reduce){#mkbug-fab,#mkbug-fab span{transition:none}}' +
    '#mkbug-fab:focus-visible{outline:3px solid #818cf8;outline-offset:2px}#mkbug-fab[disabled]{opacity:.7;cursor:progress}' +
    '#mkbug-fab svg{width:18px;height:18px;flex:none}' +
    '@media (max-width:639px){#mkbug-fab{bottom:max(84px,env(safe-area-inset-bottom))}}' +
    '@media print{#mkbug-fab,#mkbug-root{display:none!important}}' +
    '#mkbug-root{position:fixed;inset:0;z-index:2147483000;font:14px/1.45 system-ui,sans-serif;color:#1f2430;touch-action:none;overscroll-behavior:contain}' +
    '#mkbug-dim{position:absolute;inset:0;background:rgba(15,20,35,.55);cursor:crosshair}' +
    '#mkbug-sel{position:absolute;display:none;border:2px solid #6366f1;border-radius:4px;background:transparent;box-shadow:0 0 0 9999px rgba(15,20,35,.55);pointer-events:none}' +
    '#mkbug-size{position:absolute;display:none;background:#6366f1;color:#fff;font-size:11px;padding:2px 7px;border-radius:4px;pointer-events:none}' +
    '#mkbug-hint{position:absolute;left:50%;top:14px;transform:translateX(-50%);display:flex;flex-wrap:wrap;gap:8px 12px;align-items:center;justify-content:center;max-width:calc(100vw - 24px);' +
    'background:#1f2430;color:#fff;border-radius:14px;padding:10px 14px;box-shadow:0 6px 18px rgba(0,0,0,.35)}' +
    '#mkbug-hint button{border:0;border-radius:8px;background:#ffffff26;color:#fff;font:inherit;font-size:13px;padding:6px 10px;cursor:pointer}' +
    '#mkbug-hint button:hover{background:#ffffff40}#mkbug-hint button:focus-visible{outline:2px solid #818cf8}' +
    '#mkbug-dlg{border:0;border-radius:14px;padding:18px;width:min(340px,calc(100vw - 24px));box-shadow:0 12px 40px rgba(0,0,0,.45);color:#1f2430;background:#fff;overscroll-behavior:contain;font:14px/1.45 system-ui,sans-serif}' +
    '#mkbug-dlg::backdrop{background:transparent;overflow:hidden;overscroll-behavior:contain}' +
    '@media (max-width:639px){#mkbug-dlg{margin:auto 12px 12px;max-height:calc(100dvh - 24px);overflow:auto}}' +
    '#mkbug-dlg h2{font-size:16px;margin:0 0 10px}#mkbug-dlg label{display:block;font-size:12px;font-weight:600;color:#556;margin:10px 0 4px}' +
    '#mkbug-dlg textarea,#mkbug-dlg input{width:100%;box-sizing:border-box;border:1px solid #cfd3dc;border-radius:8px;padding:8px 10px;font:inherit;color:#1f2430;background:#fff}' +
    '#mkbug-dlg textarea{min-height:84px;resize:vertical}#mkbug-dlg textarea:focus,#mkbug-dlg input:focus{outline:2px solid #6366f1;outline-offset:1px;border-color:#6366f1}' +
    '#mkbug-prev{display:block;max-width:100%;max-height:110px;border:1px solid #cfd3dc;border-radius:8px;margin:0 0 2px;object-fit:contain;background:#f1f3f7}' +
    '#mkbug-dlg .mkbug-err{color:#b91c1c;font-size:12px;margin:4px 0 0;min-height:0}#mkbug-dlg .mkbug-note{color:#667;font-size:11.5px;margin:10px 0 0}' +
    '#mkbug-dlg .mkbug-row{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}' +
    '#mkbug-dlg button{border:0;border-radius:8px;padding:8px 14px;font:600 13px system-ui,sans-serif;cursor:pointer;background:#eceef3;color:#334}' +
    '#mkbug-dlg button.mkbug-go{background:#4f46e5;color:#fff}#mkbug-dlg button:disabled{opacity:.6;cursor:progress}#mkbug-dlg button:focus-visible{outline:3px solid #818cf8;outline-offset:2px}';
  var st = document.createElement('style');
  st.textContent = css;
  document.head.appendChild(st);

  var fab = document.createElement('button');
  fab.id = 'mkbug-fab';
  fab.type = 'button';
  fab.setAttribute('aria-label', 'Reportar un problema');
  fab.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg><span>Reportar problema</span>';
  document.body.appendChild(fab);

  var shot = null; // { canvas, w, h } de la pantalla congelada
  var root = null, sel = null, sizeTag = null, dlg = null;
  var rect = null; // área elegida, en px de la pantalla

  function loadLib() {
    if (window.modernScreenshot) return Promise.resolve();
    return new Promise(function (ok, fail) {
      var s = document.createElement('script');
      s.src = LIB;
      s.onload = ok;
      s.onerror = function () { fail(new Error('lib')); };
      document.head.appendChild(s);
    });
  }

  function snapshot() {
    var w = window.innerWidth, h = window.innerHeight;
    return window.modernScreenshot.domToCanvas(document.documentElement, {
      width: w,
      height: h,
      scale: Math.min(window.devicePixelRatio || 1, 2),
      backgroundColor: '#ffffff',
      timeout: 8000,
      font: false,
      style: { transform: 'translate(' + -window.scrollX + 'px,' + -window.scrollY + 'px)', transformOrigin: '0 0' },
      filter: function (n) { return !(n.id === 'mkbug-fab' || n.id === 'mkbug-root'); },
      // Las contraseñas nunca salen en la captura.
      onCloneEachNode: function (c) {
        if (c instanceof HTMLInputElement && c.type === 'password') {
          c.type = 'text';
          c.value = '';
          c.setAttribute('value', '');
          c.style.background = '#e4e4e7'; // queda un campo vacío, sin los puntos
        }
      }
    }).then(function (canvas) { return { canvas: canvas, w: w, h: h }; });
  }

  function start() {
    fab.disabled = true;
    fab.setAttribute('aria-busy', 'true');
    loadLib().then(snapshot).then(function (s) {
      shot = s;
    }, function () {
      shot = null; // sin captura: igual se puede reportar
    }).then(function () {
      fab.disabled = false;
      fab.removeAttribute('aria-busy');
      openOverlay();
    });
  }

  function el(tag, props, kids) {
    var n = document.createElement(tag);
    Object.keys(props || {}).forEach(function (k) { n[k] = props[k]; });
    (kids || []).forEach(function (c) { n.appendChild(typeof c === 'string' ? document.createTextNode(c) : c); });
    return n;
  }

  function openOverlay() {
    closeAll(true);
    root = el('div', { id: 'mkbug-root' });
    var dim = el('div', { id: 'mkbug-dim' });
    sel = el('div', { id: 'mkbug-sel' });
    sizeTag = el('div', { id: 'mkbug-size' });
    var hint = el('div', { id: 'mkbug-hint' });
    hint.setAttribute('role', 'status');
    hint.appendChild(el('span', { textContent: shot ? 'Arrastrá para marcar el área del problema' : 'No pudimos tomar la captura de esta página' }));
    var all = el('button', { type: 'button', textContent: shot ? 'Toda la pantalla' : 'Reportar sin captura' });
    var cancel = el('button', { type: 'button', textContent: 'Cancelar' });
    hint.appendChild(all);
    hint.appendChild(cancel);
    root.appendChild(dim);
    root.appendChild(sel);
    root.appendChild(sizeTag);
    root.appendChild(hint);
    document.body.appendChild(root);
    all.focus();

    cancel.onclick = function () { closeAll(); };
    all.onclick = function () {
      rect = shot ? { x: 0, y: 0, w: window.innerWidth, h: window.innerHeight } : null;
      openForm();
    };
    root.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeAll(); });
    document.addEventListener('keydown', escHandler);
    root.addEventListener('wheel', function (e) { e.preventDefault(); }, { passive: false });
    if (!shot) { dim.style.cursor = 'default'; return; }

    var x0 = 0, y0 = 0, drawing = false;
    function clampX(v) { return Math.max(0, Math.min(window.innerWidth, v)); }
    function clampY(v) { return Math.max(0, Math.min(window.innerHeight, v)); }
    function draw(x1, y1) {
      var x = Math.min(x0, x1), y = Math.min(y0, y1), w = Math.abs(x1 - x0), h = Math.abs(y1 - y0);
      rect = { x: x, y: y, w: w, h: h };
      dim.style.background = 'transparent'; // el recorte oscurece el resto
      sel.style.cssText = 'display:block;left:' + x + 'px;top:' + y + 'px;width:' + w + 'px;height:' + h + 'px';
      sizeTag.style.cssText = 'display:block;left:' + x + 'px;top:' + Math.max(0, y - 22) + 'px';
      sizeTag.textContent = Math.round(w) + ' × ' + Math.round(h);
    }
    dim.addEventListener('pointerdown', function (e) {
      if (e.button !== 0 && e.pointerType === 'mouse') return;
      drawing = true;
      x0 = clampX(e.clientX);
      y0 = clampY(e.clientY);
      dim.setPointerCapture(e.pointerId);
      draw(x0, y0);
      e.preventDefault();
    });
    dim.addEventListener('pointermove', function (e) {
      if (drawing) draw(clampX(e.clientX), clampY(e.clientY));
    });
    dim.addEventListener('pointerup', function (e) {
      if (!drawing) return;
      drawing = false;
      draw(clampX(e.clientX), clampY(e.clientY));
      if (rect.w < MIN || rect.h < MIN) {
        rect = null;
        sel.style.display = 'none';
        sizeTag.style.display = 'none';
        dim.style.background = '';
        hint.firstChild.textContent = 'El área es muy chica: arrastrá para marcar una más grande';
        return;
      }
      openForm();
    });
  }

  function escHandler(e) {
    if (e.key === 'Escape' && root && !(dlg && dlg.open)) closeAll();
  }

  // El recorte elegido como JPEG (lado máx. 1600 px, ≤ 1 MB).
  function cropBlob() {
    if (!shot || !rect) return Promise.resolve(null);
    var k = shot.canvas.width / shot.w;
    var sw = Math.max(1, Math.round(rect.w * k)), sh = Math.max(1, Math.round(rect.h * k));
    var tries = [[1, 0.82], [1, 0.62], [0.7, 0.6], [0.5, 0.55]];
    var scale0 = Math.min(1, 1600 / sw, 1600 / sh);
    function attempt(i) {
      var t = tries[i], c = document.createElement('canvas');
      c.width = Math.max(1, Math.round(sw * scale0 * t[0]));
      c.height = Math.max(1, Math.round(sh * scale0 * t[0]));
      var g = c.getContext('2d');
      g.fillStyle = '#fff';
      g.fillRect(0, 0, c.width, c.height);
      g.drawImage(shot.canvas, Math.round(rect.x * k), Math.round(rect.y * k), sw, sh, 0, 0, c.width, c.height);
      return new Promise(function (ok) { c.toBlob(ok, 'image/jpeg', t[1]); }).then(function (b) {
        if (b && b.size > 1100000 && i < tries.length - 1) return attempt(i + 1);
        return b;
      });
    }
    return attempt(0);
  }

  function openForm() {
    if (dlg) dlg.remove();
    dlg = el('dialog', { id: 'mkbug-dlg' });
    dlg.setAttribute('aria-labelledby', 'mkbug-title');
    dlg.setAttribute('closedby', 'closerequest');
    var title = el('h2', { id: 'mkbug-title', textContent: 'Contanos qué pasó' });
    var prev = null;
    if (shot && rect) {
      prev = el('img', { id: 'mkbug-prev', alt: 'Área marcada en la página' });
      cropBlob().then(function (b) { if (b) prev.src = URL.createObjectURL(b); });
    }
    var l1 = el('label', { htmlFor: 'mkbug-desc', textContent: 'Descripción' });
    var desc = el('textarea', { id: 'mkbug-desc', maxLength: 2000, required: true, autofocus: true, placeholder: 'Por ejemplo: al tocar “Agregar” no pasa nada.' });
    var e1 = el('p', { className: 'mkbug-err' });
    e1.id = 'mkbug-e1';
    desc.setAttribute('aria-describedby', 'mkbug-e1');
    var l2 = el('label', { htmlFor: 'mkbug-mail', textContent: 'Tu email (opcional, por si necesitamos preguntarte algo)' });
    var mail = el('input', { id: 'mkbug-mail', type: 'email', autocomplete: 'email', maxLength: 120, placeholder: 'nombre@correo.com' });
    var e2 = el('p', { className: 'mkbug-err' });
    e2.id = 'mkbug-e2';
    mail.setAttribute('aria-describedby', 'mkbug-e2');
    var note = el('p', { className: 'mkbug-note', textContent: 'Enviamos la captura y la dirección de esta página. Solo las ve el equipo de MarketMaker.' });
    var go = el('button', { type: 'submit', className: 'mkbug-go', textContent: 'Enviar reporte' });
    var again = el('button', { type: 'button', textContent: 'Volver a marcar' });
    var no = el('button', { type: 'button', textContent: 'Cancelar' });
    var row = el('div', { className: 'mkbug-row' }, shot ? [go, again, no] : [go, no]);
    var form = el('form', {}, [title, prev || '', l1, desc, e1, l2, mail, e2, note, row]);
    form.noValidate = true;
    dlg.appendChild(form);
    root.appendChild(dlg);
    placeDialog();
    dlg.showModal();
    desc.focus();

    no.onclick = function () { closeAll(); };
    again.onclick = function () { openOverlay(); };
    dlg.addEventListener('close', function () { if (root && !sending) closeAll(); });
    var sending = false;
    form.onsubmit = function (ev) {
      ev.preventDefault();
      e1.textContent = e2.textContent = '';
      var d = desc.value.trim();
      if (d.length < 5) { e1.textContent = 'Contanos con un poco más de detalle qué pasó.'; desc.focus(); return; }
      var m = mail.value.trim();
      if (m && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(m)) { e2.textContent = 'El email no es válido.'; mail.focus(); return; }
      sending = true;
      go.disabled = again.disabled = no.disabled = true;
      go.textContent = 'Enviando…';
      cropBlob().then(function (blob) {
        var fd = new FormData();
        fd.append('descripcion', d);
        fd.append('pagina', location.href);
        fd.append('contacto', m);
        fd.append('pantalla', window.innerWidth + '×' + window.innerHeight);
        if (blob) fd.append('captura', blob, 'captura.jpg');
        return fetch('/api/reportes', { method: 'POST', body: fd, credentials: 'same-origin' });
      }).then(function (r) {
        if (r.ok) return done();
        return r.text().then(function (t) { throw new Error(t); });
      }).catch(function (err) {
        sending = false;
        go.disabled = again.disabled = no.disabled = false;
        go.textContent = 'Enviar reporte';
        e1.textContent = (err && err.message && err.message.length < 200 && !/^TypeError|Failed to fetch/.test(err.message))
          ? err.message : 'No pudimos enviar el reporte. Revisá tu conexión y probá de nuevo.';
      });
    };

    function done() {
      form.replaceChildren(
        el('h2', { id: 'mkbug-title', textContent: 'Recibimos tu reporte' }),
        el('p', { textContent: 'Gracias por avisarnos: lo vamos a revisar.' }),
        (function () {
          var b = el('button', { type: 'button', className: 'mkbug-go', textContent: 'Cerrar' });
          b.onclick = function () { closeAll(); };
          return el('div', { className: 'mkbug-row' }, [b]);
        })()
      );
      sending = false;
    }
  }

  // En pantallas anchas el formulario va pegado al área marcada.
  function placeDialog() {
    if (window.innerWidth < 640 || !rect) return;
    var W = 340, pad = 16, vw = window.innerWidth, vh = window.innerHeight;
    var left = rect.x + rect.w + pad;
    if (left + W > vw - 12) left = rect.x - pad - W;
    if (left < 12) left = Math.max(12, Math.min(vw - W - 12, rect.x + 12));
    var top = Math.max(12, Math.min(rect.y, vh - 420));
    dlg.style.cssText = 'margin:0;left:' + left + 'px;top:' + top + 'px';
  }

  function closeAll(keepFab) {
    document.removeEventListener('keydown', escHandler);
    if (dlg && dlg.open) dlg.close();
    if (root) root.remove();
    root = sel = sizeTag = dlg = null;
    rect = null;
    if (!keepFab) fab.focus();
  }

  fab.addEventListener('click', start);
})();
