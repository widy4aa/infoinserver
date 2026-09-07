/**
 * server.ts — Bun + Hono server
 *
 * Tanggung jawab:
 *   1. Handle GitHub OAuth flow (/api/auth/github, /api/auth/github/callback)
 *   2. Handle presence endpoints (/api/auth/github/users)
 *   3. Frontend config sync (/api/frontend/config) — bun:sqlite per GitHub user
 *   4. WebSocket proxy: pipe WS frames bidirectional ke Rust backend
 *   5. Proxy semua /api/* HTTP lainnya ke Rust backend (port 8080)
 *   6. Serve Vue static files (production) atau proxy ke Vite dev server (development)
 */

import { Hono } from 'hono'
import { serveStatic } from 'hono/bun'
import { sign, verify } from 'hono/jwt'
import { Database } from 'bun:sqlite'
import type { ServerWebSocket } from 'bun'

// ── Config dari environment ──────────────────────────────────────────────────
const PORT               = parseInt(Bun.env.FRONTEND_PORT ?? '3000')
const RUST_BACKEND_URL   = Bun.env.RUST_BACKEND_URL ?? 'http://localhost:8080'
const FRONTEND_URL       = Bun.env.FRONTEND_URL     ?? `http://localhost:${PORT}`
const JWT_SECRET         = Bun.env.JWT_SECRET        ?? 'changeme-jwt-secret'
const SESSION_SECRET     = Bun.env.GITHUB_SESSION_SECRET ?? 'infoinserver-github-session-secret-2026'
const GITHUB_CLIENT_ID   = Bun.env.GITHUB_CLIENT_ID  ?? ''
const GITHUB_CLIENT_SECRET = Bun.env.GITHUB_CLIENT_SECRET ?? ''
const GITHUB_REDIRECT_URI  = Bun.env.GITHUB_REDIRECT_URI ?? `${FRONTEND_URL}/api/auth/github/callback`
const IS_DEV             = Bun.env.NODE_ENV !== 'production'
const VITE_DEV_URL       = Bun.env.VITE_DEV_URL ?? 'http://localhost:5173'
const FRONTEND_DB_PATH   = Bun.env.FRONTEND_DB_PATH ?? './frontend.db'

// Rust backend WS URL (http → ws)
const RUST_WS_URL = RUST_BACKEND_URL.replace(/^http/, 'ws')

// ── SQLite: Frontend Config DB ────────────────────────────────────────────────
const db = new Database(FRONTEND_DB_PATH, { create: true })
db.run(`
  CREATE TABLE IF NOT EXISTS frontend_config (
    github_username  TEXT PRIMARY KEY,
    servers          TEXT NOT NULL DEFAULT '[]',
    labels           TEXT NOT NULL DEFAULT '[]',
    active_server_id TEXT NOT NULL DEFAULT '',
    updated_at       INTEGER NOT NULL DEFAULT 0
  )
`)
console.log(`[db] Frontend config DB: ${FRONTEND_DB_PATH}`)

// ── Helper: verify GitHub session token → username ────────────────────────────
const verifySession = async (authHeader: string | undefined): Promise<string | null> => {
  if (!authHeader?.startsWith('Bearer ')) return null
  const token = authHeader.slice(7)
  try {
    const claims = await verify(token, SESSION_SECRET, 'HS256') as { sub?: string }
    return claims.sub ?? null
  } catch {
    return null
  }
}

// ── In-memory presence store ──────────────────────────────────────────────────
const presenceStore = new Map<string, { name: string; avatar_url: string; last_seen: number }>()

// ── Hono App (HTTP routes) ────────────────────────────────────────────────────
const app = new Hono()

// ── 1. GitHub OAuth: initiate ─────────────────────────────────────────────────
app.get('/api/auth/github', (c) => {
  const url =
    `https://github.com/login/oauth/authorize` +
    `?client_id=${encodeURIComponent(GITHUB_CLIENT_ID)}` +
    `&redirect_uri=${encodeURIComponent(GITHUB_REDIRECT_URI)}` +
    `&scope=read%3Auser`
  return c.redirect(url, 302)
})

// ── 2. GitHub OAuth: callback ─────────────────────────────────────────────────
app.get('/api/auth/github/callback', async (c) => {
  const errorParam = c.req.query('error')
  if (errorParam) {
    return c.redirect(`${FRONTEND_URL}/login?error=${encodeURIComponent(errorParam)}`, 302)
  }

  const code = c.req.query('code')
  if (!code) {
    return c.redirect(`${FRONTEND_URL}/login?error=missing_code`, 302)
  }

  const tokenRes = await fetch('https://github.com/login/oauth/access_token', {
    method: 'POST',
    headers: { 'Accept': 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_id:     GITHUB_CLIENT_ID,
      client_secret: GITHUB_CLIENT_SECRET,
      code,
      redirect_uri:  GITHUB_REDIRECT_URI,
    }),
  })

  if (!tokenRes.ok) {
    return c.redirect(`${FRONTEND_URL}/login?error=github_token_failed`, 302)
  }

  const tokenData = await tokenRes.json() as { access_token?: string; error?: string }
  if (!tokenData.access_token) {
    const err = tokenData.error ?? 'no_token'
    return c.redirect(`${FRONTEND_URL}/login?error=${encodeURIComponent(err)}`, 302)
  }

  const userRes = await fetch('https://api.github.com/user', {
    headers: {
      'Authorization': `Bearer ${tokenData.access_token}`,
      'User-Agent': 'infoinserver',
    },
  })

  if (!userRes.ok) {
    return c.redirect(`${FRONTEND_URL}/login?error=github_profile_failed`, 302)
  }

  const ghUser = await userRes.json() as { login: string; name?: string; avatar_url: string }
  const displayName = ghUser.name ?? ghUser.login

  presenceStore.set(ghUser.login, {
    name:       displayName,
    avatar_url: ghUser.avatar_url,
    last_seen:  Math.floor(Date.now() / 1000),
  })

  const exp = Math.floor(Date.now() / 1000) + 86400 * 7
  const sessionToken = await sign(
    { sub: ghUser.login, name: displayName, avatar: ghUser.avatar_url, exp },
    SESSION_SECRET,
  )

  const redirectUrl =
    `${FRONTEND_URL}/auth/callback` +
    `?token=${encodeURIComponent(sessionToken)}` +
    `&user=${encodeURIComponent(ghUser.login)}` +
    `&name=${encodeURIComponent(displayName)}` +
    `&avatar=${encodeURIComponent(ghUser.avatar_url)}`

  return c.redirect(redirectUrl, 302)
})

// ── 3. Heartbeat (nonaktif sementara) ─────────────────────────────────────────
// app.post('/api/auth/github/heartbeat', ...)

// ── 4. GitHub users list (presence) ──────────────────────────────────────────
app.get('/api/auth/github/users', (c) => {
  const now = Math.floor(Date.now() / 1000)
  const users = Array.from(presenceStore.entries()).map(([username, data]) => ({
    username,
    name:       data.name,
    avatar_url: data.avatar_url,
    online:     (now - data.last_seen) < 60,
    last_seen:  data.last_seen,
  }))
  return c.json(users)
})

// ── 5. Frontend Config: GET ───────────────────────────────────────────────────
app.get('/api/frontend/config', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  const row = db.query(
    'SELECT servers, labels, active_server_id FROM frontend_config WHERE github_username = ?'
  ).get(username) as { servers: string; labels: string; active_server_id: string } | null

  if (!row) {
    return c.json({ exists: false })
  }

  return c.json({
    exists: true,
    servers:          JSON.parse(row.servers),
    labels:           JSON.parse(row.labels),
    active_server_id: row.active_server_id,
  })
})

// ── 6. Frontend Config: PUT ───────────────────────────────────────────────────
app.put('/api/frontend/config', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  let body: { servers?: unknown; labels?: unknown; active_server_id?: string }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const servers          = JSON.stringify(body.servers          ?? [])
  const labels           = JSON.stringify(body.labels           ?? [])
  const active_server_id = String(body.active_server_id ?? '')
  const now              = Math.floor(Date.now() / 1000)

  db.run(
    `INSERT INTO frontend_config (github_username, servers, labels, active_server_id, updated_at)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(github_username) DO UPDATE SET
       servers          = excluded.servers,
       labels           = excluded.labels,
       active_server_id = excluded.active_server_id,
       updated_at       = excluded.updated_at`,
    [username, servers, labels, active_server_id, now]
  )

  return c.json({ ok: true })
})

// ── 7. HTTP Proxy: semua /api/* non-WS ke Rust backend ────────────────────────
app.all('/api/*', async (c) => {
  const url = RUST_BACKEND_URL + c.req.path + (c.req.url.includes('?') ? '?' + c.req.url.split('?')[1] : '')

  const proxyReq = new Request(url, {
    method:  c.req.method,
    headers: c.req.raw.headers,
    body:    ['GET', 'HEAD'].includes(c.req.method) ? undefined : c.req.raw.body,
  })

  try {
    const proxyRes = await fetch(proxyReq)
    return new Response(proxyRes.body, {
      status:  proxyRes.status,
      headers: proxyRes.headers,
    })
  } catch (err) {
    console.error('[proxy] Rust backend unreachable:', err)
    return c.json({ error: 'Backend unreachable' }, 502)
  }
})

// ── 8. Serve static files / proxy ke Vite dev ────────────────────────────────
if (IS_DEV) {
  app.get('*', async (c) => {
    const viteUrl = VITE_DEV_URL + c.req.path
    try {
      const res = await fetch(viteUrl, { headers: c.req.raw.headers })
      return new Response(res.body, { status: res.status, headers: res.headers })
    } catch {
      return c.text('Vite dev server not running at ' + VITE_DEV_URL, 502)
    }
  })
} else {
  app.use('/*', serveStatic({ root: '../static' }))
  app.get('*', serveStatic({ path: '../static/index.html' }))
}

// ── WebSocket data type ───────────────────────────────────────────────────────
interface WsData {
  backendUrl: string
  backendWs:  WebSocket | null
}

// ── Bun.serve — unified HTTP + WebSocket server ──────────────────────────────
const server = Bun.serve<WsData>({
  port: PORT,

  fetch(req, server) {
    const url = new URL(req.url)

    // Deteksi WebSocket upgrade: path /api/* dengan header Upgrade: websocket
    if (url.pathname.startsWith('/api/') && req.headers.get('upgrade')?.toLowerCase() === 'websocket') {
      const backendUrl = RUST_WS_URL + url.pathname + url.search
      const success = server.upgrade(req, {
        data: { backendUrl, backendWs: null },
      })
      if (success) return undefined  // upgrade berhasil, Bun akan panggil websocket handlers
      return new Response('WebSocket upgrade failed', { status: 400 })
    }

    // Non-WebSocket request → Hono
    return app.fetch(req, { ip: server.requestIP(req) })
  },

  websocket: {
    // Browser berhasil connect → buka koneksi ke Rust backend
    open(ws: ServerWebSocket<WsData>) {
      const backendUrl = ws.data.backendUrl
      console.log(`[ws] Browser connected, opening backend: ${backendUrl}`)

      const backendWs = new WebSocket(backendUrl)

      // Dari Rust → Browser
      backendWs.onmessage = (event) => {
        try {
          if (ws.readyState === 1) ws.send(event.data)
        } catch { /* browser sudah disconnect */ }
      }

      // Rust tutup koneksi → tutup browser juga
      backendWs.onclose = () => {
        console.log(`[ws] Backend closed: ${backendUrl}`)
        try { ws.close() } catch {}
      }

      backendWs.onerror = (err) => {
        console.error(`[ws] Backend error: ${backendUrl}`, err)
        try { ws.close() } catch {}
      }

      ws.data.backendWs = backendWs
    },

    // Browser kirim message → forward ke Rust
    message(ws: ServerWebSocket<WsData>, msg) {
      const backendWs = ws.data.backendWs
      if (backendWs && backendWs.readyState === WebSocket.OPEN) {
        backendWs.send(msg)
      }
    },

    // Browser disconnect → tutup koneksi ke Rust
    close(ws: ServerWebSocket<WsData>) {
      console.log('[ws] Browser disconnected')
      const backendWs = ws.data.backendWs
      if (backendWs) {
        try { backendWs.close() } catch {}
        ws.data.backendWs = null
      }
    },
  },
})

// ── Start ─────────────────────────────────────────────────────────────────────
console.log(`[server] Bun+Hono running on http://0.0.0.0:${PORT}`)
console.log(`[server] Mode: ${IS_DEV ? 'development (proxy → Vite)' : 'production (static)'}`)
console.log(`[server] Rust backend: ${RUST_BACKEND_URL}`)
console.log(`[server] WebSocket proxy: ${RUST_WS_URL}`)
