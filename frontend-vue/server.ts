/**
 * server.ts — Bun + Hono server
 *
 * Tanggung jawab:
 *   1. Handle GitHub OAuth flow (/api/auth/github, /api/auth/github/callback)
 *   2. Handle presence endpoints (/api/auth/github/users)
 *   3. Frontend config sync — global (admin config) (/api/frontend/config)
 *   4. Role-based access control (/api/roles/*)
 *   5. WebSocket proxy: pipe WS frames bidirectional ke Rust backend
 *   6. Proxy semua /api/* HTTP lainnya ke Rust backend (port 8080)
 *   7. Serve Vue static files (production) atau proxy ke Vite dev server (development)
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

// Admin default — pemilik instalasi, selalu role 'admin'
const DEFAULT_ADMIN = Bun.env.DEFAULT_ADMIN ?? 'widy4aa'

// Rust backend WS URL (http → ws)
const RUST_WS_URL = RUST_BACKEND_URL.replace(/^http/, 'ws')

// ── SQLite: Frontend Config DB ────────────────────────────────────────────────
const db = new Database(FRONTEND_DB_PATH, { create: true })

// Config global (disimpan atas nama admin default)
db.run(`
  CREATE TABLE IF NOT EXISTS frontend_config (
    github_username  TEXT PRIMARY KEY,
    servers          TEXT NOT NULL DEFAULT '[]',
    labels           TEXT NOT NULL DEFAULT '[]',
    active_server_id TEXT NOT NULL DEFAULT '',
    updated_at       INTEGER NOT NULL DEFAULT 0
  )
`)

// Role per GitHub user: 'admin' | 'master' | 'slave'
db.run(`
  CREATE TABLE IF NOT EXISTS user_roles (
    github_username TEXT PRIMARY KEY,
    role            TEXT NOT NULL DEFAULT 'slave',
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch())
  )
`)

// Akses grup berdasarkan role: tiap label punya set role yang boleh masuk
// Default: label baru bisa diakses semua role (admin, master, slave)
// Jika slave tidak ada di allowed_roles label → greyed out untuk slave
db.run(`
  CREATE TABLE IF NOT EXISTS group_allowed_roles (
    label_id     TEXT NOT NULL,
    role         TEXT NOT NULL,           -- 'admin' | 'master' | 'slave'
    PRIMARY KEY (label_id, role)
  )
`)

// Pengaturan global: default group untuk user baru yang daftar
db.run(`
  CREATE TABLE IF NOT EXISTS group_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
  )
`)

// Custom roles: role dengan nama bebas yang dibuat admin
db.run(`
  CREATE TABLE IF NOT EXISTS custom_roles (
    name       TEXT PRIMARY KEY,
    can_write  INTEGER NOT NULL DEFAULT 0,   -- 1 = bisa edit config global
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
  )
`)

// Grup yang bisa diakses oleh custom role tertentu
// Kosong = akses semua grup (open)
db.run(`
  CREATE TABLE IF NOT EXISTS custom_role_groups (
    role_name TEXT NOT NULL,
    label_id  TEXT NOT NULL,
    PRIMARY KEY (role_name, label_id)
  )
`)

// Mapping serverId → URL asli Rust backend (tidak pernah dikirim ke browser)
db.run(`
  CREATE TABLE IF NOT EXISTS server_urls (
    server_id  TEXT PRIMARY KEY,
    url        TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
  )
`)

// Error log dari client (browser) — HTTP errors, JSON parse errors, dll
// Berguna untuk debugging tanpa harus buka DevTools
db.run(`
  CREATE TABLE IF NOT EXISTS client_errors (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp  TEXT NOT NULL DEFAULT (datetime('now')),
    server_id  TEXT,           -- serverId aktif saat error terjadi
    server_name TEXT,          -- nama server (untuk display)
    method     TEXT,           -- GET / POST / PUT / DELETE
    path       TEXT NOT NULL,  -- URL path yang gagal (/api/proxy/xxx/api/...)
    status     INTEGER,        -- HTTP status code (500, 503, 404, dll)
    message    TEXT,           -- pesan error dari response body
    level      TEXT NOT NULL DEFAULT 'ERROR'  -- ERROR | WARN
  )
`)

// Catatan per-server — dibaca oleh semua user yang login (shared/public notepad)
// Berguna untuk menginfokan kondisi server, hal yang tidak boleh dilakukan, dll
db.run(`
  CREATE TABLE IF NOT EXISTS server_notes (
    server_id   TEXT PRIMARY KEY,
    content     TEXT NOT NULL DEFAULT '',
    updated_by  TEXT,          -- GitHub username yang terakhir edit
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
  )
`)

// Bootstrap: pastikan admin default selalu ada
db.run(`
  INSERT INTO user_roles (github_username, role)
  VALUES (?, 'admin')
  ON CONFLICT(github_username) DO UPDATE SET
    role = CASE WHEN role != 'admin' THEN 'admin' ELSE role END
`, [DEFAULT_ADMIN])

console.log(`[db] Frontend config DB: ${FRONTEND_DB_PATH}`)
console.log(`[db] Default admin: ${DEFAULT_ADMIN}`)

// ── Konstanta built-in roles ─────────────────────────────────────────────────
const BUILTIN_ROLES = ['admin', 'master', 'slave'] as const

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

// ── Helper: ambil role user (bisa built-in atau custom) ───────────────────────
const getUserRole = (username: string): string => {
  const row = db.query('SELECT role FROM user_roles WHERE github_username = ?').get(username) as { role: string } | null
  return row?.role ?? 'slave'
}

// ── Helper: cek apakah role valid (built-in atau custom yang ada di DB) ───────
const isValidRole = (role: string): boolean => {
  if (BUILTIN_ROLES.includes(role as typeof BUILTIN_ROLES[number])) return true
  const custom = db.query('SELECT name FROM custom_roles WHERE name = ?').get(role)
  return !!custom
}

// ── Helper: apakah role ini bisa write config global ─────────────────────────
const canRoleWrite = (role: string): boolean => {
  if (role === 'admin' || role === 'master') return true
  if (role === 'slave') return false
  // Custom role: cek can_write di DB
  const row = db.query('SELECT can_write FROM custom_roles WHERE name = ?').get(role) as { can_write: number } | null
  return (row?.can_write ?? 0) === 1
}

// ── Helper: ambil label_id yang restricted untuk role ini ────────────────────
const getRoleRestrictedLabels = (role: string): string[] => {
  if (role === 'admin' || role === 'master') return []  // akses semua

  if (role === 'slave') {
    // Built-in slave: restricted berdasarkan group_allowed_roles
    const restrictedRows = db.query(
      'SELECT DISTINCT label_id FROM group_allowed_roles'
    ).all() as { label_id: string }[]
    const restricted: string[] = []
    for (const { label_id } of restrictedRows) {
      const allowed = db.query(
        'SELECT role FROM group_allowed_roles WHERE label_id = ? AND role = ?'
      ).get(label_id, role) as { role: string } | null
      if (!allowed) restricted.push(label_id)
    }
    return restricted
  }

  // Custom role: cek custom_role_groups
  // Jika tidak ada entry → akses semua grup (open)
  const groupRows = db.query(
    'SELECT label_id FROM custom_role_groups WHERE role_name = ?'
  ).all(role) as { label_id: string }[]
  if (groupRows.length === 0) return []  // open = tidak ada restriction

  // Ada entries → hanya label yang listed yang bisa diakses
  // restricted = semua label dari group_allowed_roles yang TIDAK include role ini
  // Untuk custom role kita gunakan pendekatan whitelist:
  // label_id yang ADA di custom_role_groups = allowed
  // Kita return yang restricted berdasarkan context labels (tapi kita tidak tahu semua labels di sini)
  // Jadi kita return allowedLabels dan frontend filter sisanya
  // → kembalikan { restrictedLabels: [], allowedLabels: [...] } lebih clear
  // Untuk sekarang: return allowedLabels sebagai signal khusus dengan prefix
  const allowed = groupRows.map(r => r.label_id)
  return [`__whitelist__:${JSON.stringify(allowed)}`]
}

// ── Helper: daftarkan user baru dengan defaultRole dari settings ──────────────
const ensureUserExists = (username: string) => {
  // Ambil default role dari settings
  const settingRow = db.query("SELECT value FROM group_settings WHERE key = 'default_role'").get() as { value: string } | null
  const defaultRole = settingRow?.value ?? 'slave'

  db.run(`
    INSERT OR IGNORE INTO user_roles (github_username, role)
    VALUES (?, ?)
  `, [username, defaultRole])
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

  // Daftarkan user jika belum ada (default role: slave)
  ensureUserExists(ghUser.login)

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

// ── 3. GitHub users list (presence) ──────────────────────────────────────────
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

// ── 4. Role: GET /api/roles/me ────────────────────────────────────────────────
app.get('/api/roles/me', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  ensureUserExists(username)
  const role = getUserRole(username)
  const canWrite = canRoleWrite(role)
  const rawRestricted = getRoleRestrictedLabels(role)

  // Handle custom role whitelist signal
  let restrictedLabels: string[] = []
  let allowedLabels: string[] | null = null  // null = akses semua

  if (rawRestricted.length === 1 && rawRestricted[0].startsWith('__whitelist__:')) {
    allowedLabels = JSON.parse(rawRestricted[0].replace('__whitelist__:', ''))
  } else {
    restrictedLabels = rawRestricted
  }

  return c.json({ username, role, canWrite, restrictedLabels, allowedLabels })
})

// ── 5. Role: GET /api/roles/users — list semua user (admin only) ──────────────
app.get('/api/roles/users', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const rows = db.query(
    'SELECT github_username, role, updated_at FROM user_roles ORDER BY role, github_username'
  ).all() as { github_username: string; role: string; updated_at: number }[]

  // Enrich dengan presence data (avatar, name)
  const users = rows.map(r => {
    const p = presenceStore.get(r.github_username)
    return {
      username:   r.github_username,
      role:       r.role,
      name:       p?.name       ?? r.github_username,
      avatar_url: p?.avatar_url ?? '',
      updated_at: r.updated_at,
    }
  })

  return c.json(users)
})

// ── 6. Role: PUT /api/roles/:username/role — set role (admin only) ────────────
app.put('/api/roles/:username/role', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const target = c.req.param('username')
  const body = await c.req.json() as { role?: string }
  const newRole = body.role

  if (!newRole || !isValidRole(newRole)) {
    return c.json({ error: `Invalid role: ${newRole}` }, 400)
  }

  // Tidak boleh demote diri sendiri jika satu-satunya admin
  if (target === username && newRole !== 'admin') {
    const adminCount = (db.query("SELECT COUNT(*) as c FROM user_roles WHERE role = 'admin'").get() as { c: number }).c
    if (adminCount <= 1) {
      return c.json({ error: 'Cannot demote the only admin' }, 400)
    }
  }

  db.run(`
    INSERT INTO user_roles (github_username, role, updated_at)
    VALUES (?, ?, unixepoch())
    ON CONFLICT(github_username) DO UPDATE SET role = excluded.role, updated_at = excluded.updated_at
  `, [target, newRole])

  return c.json({ ok: true, username: target, role: newRole })
})

// ── 7. Role: GET /api/roles/groups — semua group + allowed roles mereka ────────
app.get('/api/roles/groups', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const rows = db.query(
    'SELECT label_id, role FROM group_allowed_roles ORDER BY label_id'
  ).all() as { label_id: string; role: string }[]

  const map: Record<string, string[]> = {}
  for (const { label_id, role } of rows) {
    if (!map[label_id]) map[label_id] = []
    map[label_id].push(role)
  }

  return c.json(map)
})

// ── 8. Role: PUT /api/roles/groups/:labelId — set allowed roles untuk satu grup
app.put('/api/roles/groups/:labelId', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const labelId = c.req.param('labelId')
  const body = await c.req.json() as { allowedRoles?: string[] }
  // Filter: hanya built-in roles yang valid (custom roles pakai custom_role_groups)
  const roles = (body.allowedRoles ?? []).filter(r => BUILTIN_ROLES.includes(r as typeof BUILTIN_ROLES[number]))

  db.run('DELETE FROM group_allowed_roles WHERE label_id = ?', [labelId])
  for (const role of roles) {
    db.run(
      'INSERT OR IGNORE INTO group_allowed_roles (label_id, role) VALUES (?, ?)',
      [labelId, role]
    )
  }

  // Semua 3 built-in atau kosong = open
  if (roles.length === 3 || roles.length === 0) {
    db.run('DELETE FROM group_allowed_roles WHERE label_id = ?', [labelId])
  }

  return c.json({ ok: true, labelId, allowedRoles: roles })
})

// ── 9. Role: GET /api/roles/settings — baca pengaturan global ────────────────
app.get('/api/roles/settings', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const groupRow = db.query("SELECT value FROM group_settings WHERE key = 'default_groups'").get() as { value: string } | null
  const roleRow  = db.query("SELECT value FROM group_settings WHERE key = 'default_role'").get()  as { value: string } | null
  const defaultGroups: string[] = groupRow ? JSON.parse(groupRow.value) : []
  const defaultRole: string = roleRow?.value ?? 'slave'
  return c.json({ defaultGroups, defaultRole })
})

// ── 10. Role: PUT /api/roles/settings — simpan pengaturan global ──────────────
app.put('/api/roles/settings', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const body = await c.req.json() as { defaultGroups?: string[]; defaultRole?: string }
  const defaultGroups = body.defaultGroups ?? []
  const defaultRole   = body.defaultRole ?? 'slave'

  if (!isValidRole(defaultRole)) {
    return c.json({ error: `Invalid defaultRole: ${defaultRole}` }, 400)
  }

  db.run(`INSERT INTO group_settings (key, value) VALUES ('default_groups', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value`, [JSON.stringify(defaultGroups)])
  db.run(`INSERT INTO group_settings (key, value) VALUES ('default_role', ?)  ON CONFLICT(key) DO UPDATE SET value = excluded.value`, [defaultRole])

  return c.json({ ok: true, defaultGroups, defaultRole })
})

// ── 11. Custom Roles: GET /api/roles/custom — list semua custom role ──────────
app.get('/api/roles/custom', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const roles = db.query(
    'SELECT name, can_write, created_at FROM custom_roles ORDER BY created_at'
  ).all() as { name: string; can_write: number; created_at: number }[]

  // Enrich dengan allowed groups per custom role
  const result = roles.map(r => {
    const groups = db.query(
      'SELECT label_id FROM custom_role_groups WHERE role_name = ?'
    ).all(r.name) as { label_id: string }[]
    return {
      name:       r.name,
      can_write:  r.can_write === 1,
      allowedGroups: groups.map(g => g.label_id),
      created_at: r.created_at,
    }
  })

  return c.json(result)
})

// ── 12. Custom Roles: POST /api/roles/custom — buat custom role baru ──────────
app.post('/api/roles/custom', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const body = await c.req.json() as { name?: string; can_write?: boolean; allowedGroups?: string[] }
  const name = body.name?.trim().toLowerCase().replace(/\s+/g, '_')

  if (!name || name.length < 2) {
    return c.json({ error: 'Role name must be at least 2 characters' }, 400)
  }
  if (BUILTIN_ROLES.includes(name as typeof BUILTIN_ROLES[number])) {
    return c.json({ error: `'${name}' is a built-in role and cannot be overridden` }, 400)
  }

  const existing = db.query('SELECT name FROM custom_roles WHERE name = ?').get(name)
  if (existing) {
    return c.json({ error: `Role '${name}' already exists` }, 409)
  }

  const canWrite = body.can_write ? 1 : 0
  db.run('INSERT INTO custom_roles (name, can_write) VALUES (?, ?)', [name, canWrite])

  const allowedGroups = body.allowedGroups ?? []
  for (const labelId of allowedGroups) {
    db.run('INSERT OR IGNORE INTO custom_role_groups (role_name, label_id) VALUES (?, ?)', [name, labelId])
  }

  return c.json({ ok: true, name, can_write: canWrite === 1, allowedGroups })
})

// ── 13. Custom Roles: PUT /api/roles/custom/:name — edit custom role ──────────
app.put('/api/roles/custom/:name', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const name = c.req.param('name')
  const body = await c.req.json() as { can_write?: boolean; allowedGroups?: string[] }

  const existing = db.query('SELECT name FROM custom_roles WHERE name = ?').get(name)
  if (!existing) return c.json({ error: `Role '${name}' not found` }, 404)

  const canWrite = body.can_write ? 1 : 0
  db.run('UPDATE custom_roles SET can_write = ? WHERE name = ?', [canWrite, name])

  db.run('DELETE FROM custom_role_groups WHERE role_name = ?', [name])
  const allowedGroups = body.allowedGroups ?? []
  for (const labelId of allowedGroups) {
    db.run('INSERT OR IGNORE INTO custom_role_groups (role_name, label_id) VALUES (?, ?)', [name, labelId])
  }

  return c.json({ ok: true, name, can_write: canWrite === 1, allowedGroups })
})

// ── 14. Custom Roles: DELETE /api/roles/custom/:name — hapus custom role ──────
app.delete('/api/roles/custom/:name', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  const name = c.req.param('name')

  const existing = db.query('SELECT name FROM custom_roles WHERE name = ?').get(name)
  if (!existing) return c.json({ error: `Role '${name}' not found` }, 404)

  // Hapus role dan groups-nya
  db.run('DELETE FROM custom_role_groups WHERE role_name = ?', [name])
  db.run('DELETE FROM custom_roles WHERE name = ?', [name])

  // User yang pakai role ini → reset ke slave
  db.run("UPDATE user_roles SET role = 'slave' WHERE role = ?", [name])

  return c.json({ ok: true, name })
})

// ── 9. Frontend Config: GET ───────────────────────────────────────────────────
// Semua user (admin, master, slave) membaca config yang SAMA — milik admin default.
// Ini berarti satu config global yang di-share.
app.get('/api/frontend/config', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  // Selalu baca config milik admin default
  const row = db.query(
    'SELECT servers, labels, active_server_id FROM frontend_config WHERE github_username = ?'
  ).get(DEFAULT_ADMIN) as { servers: string; labels: string; active_server_id: string } | null

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

// ── 10. Frontend Config: PUT ──────────────────────────────────────────────────
// Hanya admin dan master yang boleh update config global.
// Slave dapat melihat tapi tidak bisa mengubah.
app.put('/api/frontend/config', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  const role = getUserRole(username)
  if (!canRoleWrite(role)) {
    return c.json({ error: 'This role cannot modify the global config' }, 403)
  }

  let body: { servers?: unknown; labels?: unknown; active_server_id?: string }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const servers          = JSON.stringify(body.servers          ?? [])
  const labels           = JSON.stringify(body.labels           ?? [])
  const active_server_id = String(body.active_server_id ?? '')
  const now              = Math.floor(Date.now() / 1000)

  // Simpan selalu ke nama admin default (config global)
  db.run(
    `INSERT INTO frontend_config (github_username, servers, labels, active_server_id, updated_at)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(github_username) DO UPDATE SET
       servers          = excluded.servers,
       labels           = excluded.labels,
       active_server_id = excluded.active_server_id,
       updated_at       = excluded.updated_at`,
    [DEFAULT_ADMIN, servers, labels, active_server_id, now]
  )

  return c.json({ ok: true })
})

// ── 11. Server Notes: GET /api/server-notes/:serverId ────────────────────────
// Ambil catatan untuk server tertentu. Semua user yang login bisa baca.
app.get('/api/server-notes/:serverId', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  const serverId = c.req.param('serverId')
  const row = db.query(
    'SELECT content, updated_by, updated_at FROM server_notes WHERE server_id = ?'
  ).get(serverId) as { content: string; updated_by: string | null; updated_at: number } | null

  return c.json({
    server_id:  serverId,
    content:    row?.content    ?? '',
    updated_by: row?.updated_by ?? null,
    updated_at: row?.updated_at ?? null,
  })
})

// ── 12. Server Notes: PUT /api/server-notes/:serverId ────────────────────────
// Simpan/update catatan. Hanya admin dan master yang boleh edit.
// Slave bisa baca (GET) tapi tidak bisa tulis (PUT).
app.put('/api/server-notes/:serverId', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  const role = getUserRole(username)
  if (!canRoleWrite(role)) {
    return c.json({ error: 'Hanya admin dan master yang bisa mengedit catatan server' }, 403)
  }

  const serverId = c.req.param('serverId')
  let body: { content?: string }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const content = body.content ?? ''
  const now = Math.floor(Date.now() / 1000)

  db.run(`
    INSERT INTO server_notes (server_id, content, updated_by, updated_at)
    VALUES (?, ?, ?, ?)
    ON CONFLICT(server_id) DO UPDATE SET
      content    = excluded.content,
      updated_by = excluded.updated_by,
      updated_at = excluded.updated_at
  `, [serverId, content, username, now])

  return c.json({ ok: true, updated_by: username, updated_at: now })
})

// ── 13. Client Error Log: POST /api/client-errors ────────────────────────────
// Browser kirim error batch setiap kali apiFetch dapat response >= 400.
// Tidak butuh auth Linux, cukup GitHub session (agar tidak bisa diisi sembarang).
app.post('/api/client-errors', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  let body: { errors?: Array<{
    server_id?: string; server_name?: string; method?: string;
    path?: string; status?: number; message?: string; level?: string
  }> }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const errors = body.errors ?? []
  if (!Array.isArray(errors) || errors.length === 0) return c.json({ ok: true, inserted: 0 })

  // Batasi max 50 error per request agar tidak bisa di-abuse
  const batch = errors.slice(0, 50)
  const now = new Date().toISOString()

  const stmt = db.prepare(`
    INSERT INTO client_errors (timestamp, server_id, server_name, method, path, status, message, level)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
  `)

  for (const e of batch) {
    stmt.run(
      now,
      e.server_id   ?? null,
      e.server_name ?? null,
      e.method      ?? null,
      e.path        ?? '(unknown)',
      e.status      ?? null,
      e.message     ?? null,
      e.level       ?? 'ERROR'
    )
  }

  return c.json({ ok: true, inserted: batch.length })
})

// ── 12. Client Error Log: GET /api/client-errors — ambil 300 error terbaru ────
app.get('/api/client-errors', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  const serverId = c.req.query('server_id')  // optional filter by server
  const level    = c.req.query('level')      // optional filter by level

  let query = `SELECT id, timestamp, server_id, server_name, method, path, status, message, level
               FROM client_errors`
  const params: (string | number)[] = []
  const conditions: string[] = []

  if (serverId) { conditions.push('server_id = ?'); params.push(serverId) }
  if (level)    { conditions.push('level = ?');     params.push(level.toUpperCase()) }

  if (conditions.length > 0) query += ' WHERE ' + conditions.join(' AND ')
  query += ' ORDER BY id DESC LIMIT 300'

  const rows = db.query(query).all(...params) as Array<{
    id: number; timestamp: string; server_id: string | null; server_name: string | null;
    method: string | null; path: string; status: number | null; message: string | null; level: string
  }>

  return c.json(rows)
})

// ── 13. Client Error Log: DELETE /api/client-errors — hapus semua (admin only) ─
app.delete('/api/client-errors', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)
  if (getUserRole(username) !== 'admin') return c.json({ error: 'Forbidden' }, 403)

  db.run('DELETE FROM client_errors')
  return c.json({ ok: true })
})

// ── 14. Proxy: POST /api/proxy/connect — login ke server target lewat Bun ─────
// Browser tidak pernah tahu URL asli server lab — Bun yang fetch.
// Body: { serverId, targetUrl, username, password }
// Response: { token, username } — sama seperti Rust /api/auth/login
app.post('/api/proxy/connect', async (c) => {
  const username = await verifySession(c.req.header('Authorization'))
  if (!username) return c.json({ error: 'Unauthorized' }, 401)

  let body: { serverId?: string; targetUrl?: string; username?: string; password?: string }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const { serverId, targetUrl, username: linuxUser, password } = body
  if (!serverId || !targetUrl || !linuxUser || !password) {
    return c.json({ error: 'serverId, targetUrl, username, and password are required' }, 400)
  }

  // Normalisasi URL
  const cleanUrl = targetUrl.endsWith('/') ? targetUrl.slice(0, -1) : targetUrl

  // Bun fetch ke Rust backend target
  let rustRes: Response
  try {
    rustRes = await fetch(`${cleanUrl}/api/auth/login`, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify({ username: linuxUser, password }),
    })
  } catch (err) {
    console.error(`[proxy/connect] Cannot reach ${cleanUrl}:`, err)
    return c.json({ error: `Cannot connect to server at ${cleanUrl}` }, 502)
  }

  const data = await rustRes.json()
  if (!rustRes.ok) {
    return c.json(data, rustRes.status)
  }

  // Simpan URL asli ke SQLite — browser tidak perlu tahu
  db.run(`
    INSERT INTO server_urls (server_id, url, updated_at)
    VALUES (?, ?, unixepoch())
    ON CONFLICT(server_id) DO UPDATE SET url = excluded.url, updated_at = excluded.updated_at
  `, [serverId, cleanUrl])

  console.log(`[proxy/connect] Registered server ${serverId} → ${cleanUrl}`)
  return c.json(data, rustRes.status)
})

// ── 15. Proxy: /api/proxy/:serverId/* — relay HTTP ke server target ───────────
// Semua request dari browser ke /api/proxy/{id}/api/... di-forward ke URL asli.
app.all('/api/proxy/:serverId/*', async (c) => {
  const serverId = c.req.param('serverId')

  const row = db.query('SELECT url FROM server_urls WHERE server_id = ?').get(serverId) as { url: string } | null
  if (!row) return c.json({ error: `Server ${serverId} not registered` }, 404)

  // Strip /api/proxy/:serverId dari path, sisanya forward ke Rust
  const stripped = c.req.path.replace(`/api/proxy/${serverId}`, '')
  const qs = c.req.url.includes('?') ? '?' + c.req.url.split('?')[1] : ''
  const targetUrl = row.url + stripped + qs

  const proxyReq = new Request(targetUrl, {
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
    console.error(`[proxy] Cannot reach server ${serverId} (${row.url}):`, err)
    return c.json({ error: 'Target server unreachable' }, 502)
  }
})

// ── 16. HTTP Proxy: semua /api/* non-WS ke Rust backend ───────────────────────
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

// ── 12. Serve static files / proxy ke Vite dev ────────────────────────────────
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
      let backendUrl: string

      // WebSocket ke server lab via proxy: /api/proxy/:serverId/api/.../ws
      const proxyMatch = url.pathname.match(/^\/api\/proxy\/([^/]+)(\/.*)$/)
      if (proxyMatch) {
        const serverId = proxyMatch[1]
        const restPath = proxyMatch[2]
        const row = db.query('SELECT url FROM server_urls WHERE server_id = ?').get(serverId) as { url: string } | null
        if (!row) return new Response('Server not registered', { status: 404 })
        const wsTargetUrl = row.url.replace(/^http/, 'ws')
        backendUrl = wsTargetUrl + restPath + url.search
      } else {
        // WebSocket ke Rust backend lokal (untuk server gateway itu sendiri)
        backendUrl = RUST_WS_URL + url.pathname + url.search
      }

      const success = server.upgrade(req, {
        data: { backendUrl, backendWs: null },
      })
      if (success) return undefined
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
