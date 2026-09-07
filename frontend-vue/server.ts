/**
 * server.ts — Bun + Hono server
 *
 * Tanggung jawab:
 *   1. Handle GitHub OAuth flow (/api/auth/github, /api/auth/github/callback)
 *   2. Handle presence endpoints (/api/auth/github/heartbeat, /api/auth/github/users)
 *   3. Proxy semua /api/* lainnya ke Rust backend (port 8080)
 *   4. Serve Vue static files (production) atau proxy ke Vite dev server (development)
 *
 * JWT menggunakan secret yang sama dengan Rust backend (JWT_SECRET),
 * sehingga token yang di-issue di sini langsung bisa di-verify Rust.
 */

import { Hono } from 'hono'
import { serveStatic } from 'hono/bun'
import { sign, verify } from 'hono/jwt'

// ── Config dari environment ──────────────────────────────────────────────────
const PORT              = parseInt(Bun.env.FRONTEND_PORT ?? '3000')
const RUST_BACKEND_URL  = Bun.env.RUST_BACKEND_URL ?? 'http://localhost:8080'
const FRONTEND_URL      = Bun.env.FRONTEND_URL     ?? `http://localhost:${PORT}`
const JWT_SECRET        = Bun.env.JWT_SECRET        ?? 'changeme-jwt-secret'
const SESSION_SECRET    = Bun.env.GITHUB_SESSION_SECRET ?? 'infoinserver-github-session-secret-2026'
const GITHUB_CLIENT_ID  = Bun.env.GITHUB_CLIENT_ID  ?? ''
const GITHUB_CLIENT_SECRET = Bun.env.GITHUB_CLIENT_SECRET ?? ''
const GITHUB_REDIRECT_URI  = Bun.env.GITHUB_REDIRECT_URI ?? `${FRONTEND_URL}/api/auth/github/callback`
const IS_DEV            = Bun.env.NODE_ENV !== 'production'
const VITE_DEV_URL      = Bun.env.VITE_DEV_URL ?? 'http://localhost:5173'

// ── In-memory presence store ─────────────────────────────────────────────────
// { username → { name, avatar_url, last_seen (unix secs) } }
const presenceStore = new Map<string, { name: string; avatar_url: string; last_seen: number }>()

// ── App ──────────────────────────────────────────────────────────────────────
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

  // Tukar code → access_token
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

  // Ambil profil GitHub
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

  // Simpan ke presence store
  presenceStore.set(ghUser.login, {
    name:       displayName,
    avatar_url: ghUser.avatar_url,
    last_seen:  Math.floor(Date.now() / 1000),
  })

  // Sign JWT dengan GITHUB_SESSION_SECRET (sama dengan Rust GITHUB_SESSION_SECRET)
  // exp: 7 hari
  const exp = Math.floor(Date.now() / 1000) + 86400 * 7
  const sessionToken = await sign(
    { sub: ghUser.login, name: displayName, avatar: ghUser.avatar_url, exp },
    SESSION_SECRET,
  )

  // Redirect ke frontend callback page
  const redirectUrl =
    `${FRONTEND_URL}/auth/callback` +
    `?token=${encodeURIComponent(sessionToken)}` +
    `&user=${encodeURIComponent(ghUser.login)}` +
    `&name=${encodeURIComponent(displayName)}` +
    `&avatar=${encodeURIComponent(ghUser.avatar_url)}`

  return c.redirect(redirectUrl, 302)
})

// ── 3. Heartbeat (nonaktif sementara — digantikan oleh ping di HomeView) ──────
// app.post('/api/auth/github/heartbeat', async (c) => { ... })

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

// ── 5. Proxy semua /api/* lainnya ke Rust backend ────────────────────────────
app.all('/api/*', async (c) => {
  const url = RUST_BACKEND_URL + c.req.path + (c.req.url.includes('?') ? '?' + c.req.url.split('?')[1] : '')

  // Forward request ke Rust — salin method, headers, body
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

// ── 6. Serve static files / proxy ke Vite dev ────────────────────────────────
if (IS_DEV) {
  // Development: proxy semua non-API request ke Vite dev server
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
  // Production: serve file statis dari ../static (hasil build Vite)
  app.use('/*', serveStatic({ root: '../static' }))

  // SPA fallback: semua rute yang tidak dikenal → index.html
  app.get('*', serveStatic({ path: '../static/index.html' }))
}

// ── Start server ──────────────────────────────────────────────────────────────
console.log(`[server] Bun+Hono running on http://0.0.0.0:${PORT}`)
console.log(`[server] Mode: ${IS_DEV ? 'development (proxy → Vite)' : 'production (static)'}`)
console.log(`[server] Rust backend: ${RUST_BACKEND_URL}`)

export default {
  port: PORT,
  fetch: app.fetch,
}
