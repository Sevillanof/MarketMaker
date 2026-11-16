// Un Andreani de mentira para probar sin la cuenta real:
//   node scripts/fake-andreani.js        (escucha en el puerto 4010)
//   ANDREANI_API_URL=http://localhost:4010 cargo leptos watch
//
// Códigos que acepta "Conectar": cualquiera que empiece con "cHltZ" (base64 de
// "pyme…"). "malo" responde 401.
const http = require('http');

const PORT = Number(process.env.PORT || 4010);
const TOKEN = 'token-de-prueba';
const shipments = new Map(); // pedidoId -> { remito, tracking, status, polls }
let next = 1000;

const branches = [
  { Codigo: 'S01', Descripcion: 'Andreani Palermo', Direccion: { Calle: 'Av. Santa Fe', Numero: '3200', Localidad: 'CABA', Provincia: 'Ciudad Autónoma de Buenos Aires', CodigoPostal: '1425' }, horarioDeAtencion: 'Lun a Vie 9 a 18 h' },
  { Codigo: 'S02', Descripcion: 'Andreani Belgrano', Direccion: { Calle: 'Cabildo', Numero: '1800', Localidad: 'CABA', Provincia: 'Ciudad Autónoma de Buenos Aires', CodigoPostal: '1428' }, horarioDeAtencion: 'Lun a Sáb 9 a 13 h' },
  { Codigo: 'S03', Descripcion: 'Andreani Córdoba Centro', Direccion: { Calle: 'Colón', Numero: '450', Localidad: 'Córdoba', Provincia: 'Córdoba', CodigoPostal: '5000' } },
];

function send(res, status, body, type = 'application/json') {
  res.writeHead(status, { 'Content-Type': type });
  res.end(type === 'application/json' ? JSON.stringify(body) : body);
}

function authed(req) {
  return req.headers['x-auth-token'] === TOKEN;
}

http
  .createServer((req, res) => {
    let raw = '';
    req.on('data', (c) => (raw += c));
    req.on('end', () => {
      const url = new URL(req.url, `http://localhost:${PORT}`);
      const body = raw ? JSON.parse(raw) : {};
      console.log(req.method, url.pathname + url.search, raw.slice(0, 300));

      if (url.pathname === '/api/v1/Login' && req.method === 'POST') {
        const cred = req.headers.authorization || '';
        if (!cred.startsWith('cHltZ')) return send(res, 401, { message: 'Credencial inválida' });
        return send(res, 200, {
          message: 'ok',
          response: {
            accessToken: TOKEN,
            clientType: 'pyme',
            razonSocial: 'Tienda de Prueba SRL',
            cuit: '30712345678',
            email: 'prueba@example.com',
            contratos: [
              { id: 11, numeroDeContrato: '400006709', modoDeEntregaNombre: 'A domicilio', tipoDeEnvioNombre: 'paqueteria' },
              { id: 12, numeroDeContrato: '400006710', modoDeEntregaNombre: 'A sucursal', tipoDeEnvioNombre: 'paqueteria' },
              { id: 13, numeroDeContrato: '400006711', modoDeEntregaNombre: 'A domicilio', tipoDeEnvioNombre: 'bigger' },
            ],
          },
        });
      }
      if (!authed(req)) return send(res, 401, { message: 'Token vencido' });

      if (url.pathname === '/api/v1/Settings/origin' && req.method === 'PATCH') {
        return send(res, 200, { message: 'ok', response: body });
      }
      if (url.pathname === '/api/v1/Branch' && req.method === 'GET') {
        const cp = url.searchParams.get('postalCode') || '';
        const near = branches.filter((b) => b.Direccion.CodigoPostal[0] === cp[0]);
        return send(res, 200, { message: 'ok', response: { data: near } });
      }
      if (url.pathname === '/api/v1/Pyme/rates' && req.method === 'POST') {
        const grams = (body.products || []).reduce((s, p) => s + p.quantity * p.dimensions.grams, 0);
        const base = 4500 + Math.round(grams / 1000) * 800;
        const far = body.postal_code_destination?.[0] !== body.postal_code_origin?.[0];
        const home = base + (far ? 2500 : 0);
        return send(res, 200, {
          message: 'ok',
          response: {
            rates: body.force_bigger
              ? [{ code: 'bigger', total: home * 3 }]
              : [
                  { code: 'estándar', total: home },
                  { code: 'sucursal', total: Math.round(home * 0.8) },
                ],
          },
        });
      }
      if (url.pathname === '/api/v1/Pyme/ShippingRegistration' && req.method === 'POST') {
        if (!body.recipient?.phone_number) return send(res, 400, { message: 'Falta el teléfono del destinatario' });
        const pedidoId = String(next++);
        shipments.set(pedidoId, { remito: body.remito, tracking: '', status: 'pendiente de ingreso', polls: 0 });
        return send(res, 200, { message: 'ok', response: { pedidoId, numeroInterno: 'NI' + pedidoId } });
      }
      if (url.pathname === '/api/v1/Pyme/ticket' && req.method === 'POST') {
        return send(res, 200, '%PDF-1.4\n% etiqueta de prueba\n', 'application/pdf');
      }
      if (url.pathname === '/api/v1/Shipments/ByOrderNumbers' && req.method === 'POST') {
        const out = (body.salesOrderNumbers || []).map((id) => {
          const s = shipments.get(String(id));
          if (!s) return { salesOrderNumber: id, trackingNumber: '', trackingStatus: '' };
          // Cada consulta avanza un paso: pagado en Andreani, en camino, entregado.
          s.polls++;
          if (s.polls >= 1 && !s.tracking) s.tracking = '360000' + id;
          if (s.polls >= 2) s.status = 'en camino';
          if (s.polls >= 3) s.status = 'entregado';
          return { salesOrderNumber: id, trackingNumber: s.tracking, trackingStatus: s.status };
        });
        return send(res, 200, { message: 'ok', response: out });
      }
      send(res, 404, { message: 'No existe' });
    });
  })
  .listen(PORT, () => console.log(`Andreani de mentira en http://localhost:${PORT}`));
