<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useRouter } from 'vue-router'
import Sortable from 'sortablejs'
import { useServerStore } from '../stores/serverStore'
import { useThemeStore } from '../stores/themeStore'
import { useToastStore } from '../stores/toastStore'
import { useAuthStore } from '../stores/authStore'
import {
  Server, Plus, ShieldCheck, Box, FolderTree, Settings, Cloud, Activity,
  X, User, Lock, Loader2, AlertCircle, LogIn, Pencil, Check, GripVertical, Tag,
  ShieldAlert, Users, ChevronDown, Save, Trash2
} from 'lucide-vue-next'
import { getDistroIcon, getDistroColorClass } from '../utils/distro.js'

const router = useRouter()
const {
  servers, labels,
  addServer, removeServer, setToken, setActiveServer,
  addLabel, removeLabel, renameLabel,
  assignServerToLabel, reorderServersInLabel, reorderLabels,
  ungroupedServers,
} = useServerStore()
const { isDark } = useThemeStore()
const { showToast, showConfirm } = useToastStore()
const { isAdmin, isMaster, isSlave, restrictedLabels, allowedLabels, canWriteConfig, getToken: getGithubToken } = useAuthStore()

// ── Access control helpers ────────────────────────────────────────────────────
// Label restricted = ada di restrictedLabels / tidak ada di allowedLabels
const isLabelAllowed = (labelId) => {
  if (isAdmin.value || isMaster.value) return true
  // Custom role dengan whitelist
  if (allowedLabels.value !== null) return allowedLabels.value.includes(labelId)
  // Slave: cek restrictedLabels
  return !restrictedLabels.value.includes(labelId)
}

const isServerAllowed = (serverId) => {
  if (isAdmin.value || isMaster.value) return true
  const serverLabel = labels.value.find(l => l.serverIds.includes(serverId))
  if (!serverLabel) return true  // ungrouped: bebas
  return isLabelAllowed(serverLabel.id)
}

// ── Ping state ────────────────────────────────────────────────────────────────
const pingState = ref({})
const PING_TIMEOUT_MS = 5000
const PING_INTERVAL_MS = 10000

const checkPing = async (server) => {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), PING_TIMEOUT_MS)
  const start = performance.now()
  try {
    await fetch(`${server.url}/api/ping`, { signal: controller.signal })
    pingState.value[server.id] = { ms: Math.round(performance.now() - start), online: true }
  } catch {
    pingState.value[server.id] = { ms: null, online: false }
  } finally {
    clearTimeout(timer)
  }
}

let intervals = []
onMounted(() => {
  servers.value.forEach(s => {
    pingState.value[s.id] = { ms: null, online: null }
    checkPing(s)
    intervals.push(setInterval(() => checkPing(s), PING_INTERVAL_MS))
  })
})
onUnmounted(() => { intervals.forEach(clearInterval); intervals = [] })

const pingLabel   = (id) => { const p = pingState.value[id]; if (!p || p.online === null) return '···'; if (!p.online) return 'OFFLINE'; return p.ms + 'ms' }
const pingDotClass = (id) => { const p = pingState.value[id]; if (!p || p.online === null) return 'bg-slate-400'; if (!p.online) return 'bg-red-500'; if (p.ms < 100) return 'bg-emerald-500'; if (p.ms < 300) return 'bg-amber-500'; return 'bg-orange-500' }
const pingTextClass = (id) => { const p = pingState.value[id]; if (!p || p.online === null) return 'text-slate-400'; if (!p.online) return 'text-red-400'; if (p.ms < 100) return 'text-emerald-400'; if (p.ms < 300) return 'text-amber-400'; return 'text-orange-400' }
const isOnline  = (id) => pingState.value[id]?.online === true
const isOffline = (id) => pingState.value[id]?.online === false

// ── Card click ────────────────────────────────────────────────────────────────
const handleCardClick = (s) => {
  if (isEditMode.value) return
  if (!isServerAllowed(s.id)) {
    showToast('Akses Terbatas', 'Kamu tidak memiliki izin untuk mengakses server ini.', 'error')
    return
  }
  if (isOffline(s.id)) {
    showToast('Server Offline', `Cannot connect to ${s.url}. Make sure the backend is running.`, 'error')
    return
  }
  router.push(`/server/${s.id}/dashboard`)
}

// ── Card styles ───────────────────────────────────────────────────────────────
const cardStyle = (hover = false) => {
  if (isDark.value) {
    return hover
      ? 'background: rgba(15,23,42,0.30); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 8px 32px rgba(0,0,0,0.32), 0 1px 0 0 rgba(255,255,255,0.08) inset;'
      : 'background: rgba(15,23,42,0.20); backdrop-filter: blur(20px) saturate(160%); -webkit-backdrop-filter: blur(20px) saturate(160%); box-shadow: 0 4px 24px rgba(0,0,0,0.24), 0 1px 0 0 rgba(255,255,255,0.06) inset;'
  }
  return hover
    ? 'background: rgba(255,255,255,0.40); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 8px 32px rgba(0,0,0,0.12), 0 1px 0 0 rgba(255,255,255,0.8) inset;'
    : 'background: rgba(255,255,255,0.25); backdrop-filter: blur(20px) saturate(180%); -webkit-backdrop-filter: blur(20px) saturate(180%); box-shadow: 0 4px 24px rgba(0,0,0,0.08), 0 1px 0 0 rgba(255,255,255,0.6) inset;'
}
const cardStyleOffline = () => isDark.value
  ? 'background: rgba(15,23,42,0.03); backdrop-filter: blur(8px) saturate(60%) grayscale(10%); -webkit-backdrop-filter: blur(8px) saturate(60%) grayscale(10%); box-shadow: none; opacity: 0.35;'
  : 'background: rgba(255,255,255,0.03); backdrop-filter: blur(8px) saturate(60%) grayscale(10%); -webkit-backdrop-filter: blur(8px) saturate(60%) grayscale(10%); box-shadow: none; opacity: 0.35;'

// ── Edit Mode ─────────────────────────────────────────────────────────────────
const isEditMode  = ref(false)
const jiggleClass = ref('')   // '' | 'edit-jiggle-enter' | 'edit-jiggle'

const toggleEditMode = () => { isEditMode.value = !isEditMode.value }

// ── Label rename ──────────────────────────────────────────────────────────────
const editingLabelId = ref(null)
const editingLabelName = ref('')
const labelInputRefs = ref({})

const startRenameLabel = async (label) => {
  editingLabelId.value = label.id
  editingLabelName.value = label.name
  await nextTick()
  labelInputRefs.value[label.id]?.focus()
}

const commitRenameLabel = (id) => {
  if (editingLabelName.value.trim()) renameLabel(id, editingLabelName.value.trim())
  editingLabelId.value = null
}

// ── Delete server ─────────────────────────────────────────────────────────────
const handleDeleteServer = (s) => {
  showConfirm(
    'Remove Server',
    `Remove "${s.name}" from your dashboard? This will not affect the server itself.`,
    () => removeServer(s.id)
  )
}

// ── Delete label ──────────────────────────────────────────────────────────────
const handleDeleteLabel = (label) => {
  showConfirm(
    'Remove Label',
    `Remove label "${label.name}"? Servers in this label will move to Ungrouped.`,
    () => removeLabel(label.id)
  )
}

// ── Sortable instances ────────────────────────────────────────────────────────
const sortableInstances = []

const destroySortables = () => {
  sortableInstances.forEach(s => s.destroy())
  sortableInstances.length = 0
}

// Build ordered server list for a given section
const getServersForLabel = (label) => {
  return label.serverIds
    .map(id => servers.value.find(s => s.id === id))
    .filter(Boolean)
}

const initSortables = async () => {
  await nextTick()
  destroySortables()
  if (!isEditMode.value) return

  // Sortable untuk urutan label (drag antar section header)
  const labelsContainer = document.getElementById('labels-container')
  if (labelsContainer) {
    const inst = Sortable.create(labelsContainer, {
      handle: '.label-drag-handle',
      animation: 150,
      ghostClass: 'sortable-ghost',
      onEnd: (evt) => {
        const newOrder = [...labels.value]
        const [moved] = newOrder.splice(evt.oldIndex, 1)
        newOrder.splice(evt.newIndex, 0, moved)
        reorderLabels(newOrder)
      }
    })
    sortableInstances.push(inst)
  }

  // Sortable untuk card dalam setiap label + ungrouped
  const cardContainers = document.querySelectorAll('.card-sortable-group')
  cardContainers.forEach(container => {
    const labelId = container.dataset.labelId || null  // null = ungrouped

    const inst = Sortable.create(container, {
      group: 'servers',
      animation: 200,
      ghostClass: 'sortable-ghost',
      onMove: (evt) => {
        // Slave tidak boleh drag server ke grup yang restricted
        if (!isSlave.value) return true
        const toLabelId = evt.to.dataset.labelId || null
        if (!toLabelId) return true  // ke ungrouped: selalu boleh
        return isLabelAllowed(toLabelId)
      },
      onEnd: (evt) => {
        const fromLabelId = evt.from.dataset.labelId || null
        const toLabelId   = evt.to.dataset.labelId   || null
        const serverId    = evt.item.dataset.serverId

        if (fromLabelId === toLabelId) {
          // Reorder dalam label yang sama
          if (toLabelId === null) {
            // ungrouped — tidak perlu update (urutan ungrouped mengikuti servers array)
            return
          }
          const label = labels.value.find(l => l.id === toLabelId)
          if (!label) return
          const newIds = [...label.serverIds]
          const [moved] = newIds.splice(evt.oldIndex, 1)
          newIds.splice(evt.newIndex, 0, moved)
          reorderServersInLabel(toLabelId, newIds)
        } else {
          // Lintas label atau masuk/keluar ungrouped
          assignServerToLabel(serverId, toLabelId, evt.newIndex)
        }
      }
    })
    sortableInstances.push(inst)
  })
}

watch(isEditMode, async (val) => {
  if (val) {
    // Enter animation dulu, lalu switch ke loop jiggle setelah selesai
    jiggleClass.value = 'edit-jiggle-enter'
    const enterDuration = 300 + (servers.value.length * 35)
    setTimeout(() => { jiggleClass.value = 'edit-jiggle' }, enterDuration)
    await initSortables()
  } else {
    jiggleClass.value = ''
    destroySortables()
    editingLabelId.value = null
  }
})

onUnmounted(() => destroySortables())

// ── Add Server Modal ──────────────────────────────────────────────────────────
const showAddModal = ref(false)
const newName  = ref('')
const newUrl   = ref('')
const newUser  = ref('')
const newPass  = ref('')
const isAdding = ref(false)
const addError = ref(null)

const openAddModal = () => {
  newName.value = ''; newUrl.value = ''; newUser.value = ''; newPass.value = ''
  addError.value = null
  showAddModal.value = true
}
const closeAddModal = () => { showAddModal.value = false; addError.value = null }

const normalizeUrl = (raw) => {
  let url = raw.trim()
  if (url && !/^https?:\/\//i.test(url)) url = 'http://' + url
  return url.endsWith('/') ? url.slice(0, -1) : url
}

const handleAdd = async () => {
  addError.value = null
  if (!newName.value.trim() || !newUrl.value.trim() || !newUser.value.trim() || !newPass.value) {
    addError.value = 'All fields are required'; return
  }
  const cleanUrl = normalizeUrl(newUrl.value)
  isAdding.value = true
  try {
    const res = await fetch(`${cleanUrl}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: newUser.value.trim(), password: newPass.value })
    })
    const data = await res.json()
    if (!res.ok) { addError.value = data.error || `Authentication failed (${res.status})`; return }

    const id = Date.now().toString()
    addServer(newName.value.trim(), cleanUrl, id)
    setToken(id, data.token, data.username)
    pingState.value[id] = { ms: null, online: null }
    checkPing({ id, url: cleanUrl })
    intervals.push(setInterval(() => checkPing({ id, url: cleanUrl }), PING_INTERVAL_MS))

    closeAddModal()
    setActiveServer(id)
    router.push(`/server/${id}/dashboard`)
  } catch {
    addError.value = 'Cannot connect to server. Make sure the backend is running.'
  } finally {
    isAdding.value = false
  }
}

// ── Computed sections ─────────────────────────────────────────────────────────
const unGrouped = computed(() => ungroupedServers())

// ── Role Management (admin only) ──────────────────────────────────────────────
const showRoleModal    = ref(false)
const roleTab          = ref('users')   // 'users' | 'roles' | 'groups' | 'settings'
const roleUsers        = ref([])
const roleLoading      = ref(false)
const roleSaving       = ref(false)

// ── Custom roles state ────────────────────────────────────────────────────────
const customRoles      = ref([])   // { name, can_write, allowedGroups[], created_at }
const newRoleName      = ref('')
const newRoleCanWrite  = ref(false)
const newRoleGroups    = ref([])
const editingRole      = ref(null) // role yang sedang di-expand untuk edit
const editRoleCanWrite = ref(false)
const editRoleGroups   = ref([])
const rolePanelSaving  = ref(false)

// State untuk tab Groups: { labelId → allowedRoles[] }
// Kosong = open untuk semua (default)
const groupRolesMap    = ref({})
const groupSaving      = ref({})
const newGroupName     = ref('')
const addingGroup      = ref(false)

const addGroupFromModal = () => {
  const name = newGroupName.value.trim()
  if (!name) return
  addingGroup.value = true
  addLabel(name)
  newGroupName.value = ''
  addingGroup.value = false
}

// State untuk tab Settings: default groups untuk user baru
const defaultNewUserGroups = ref([])
const defaultNewUserRole   = ref('slave')
const settingsSaving   = ref(false)

const roleBadgeClass = (role) => {
  if (role === 'admin')  return 'bg-rose-100 text-rose-700 dark:bg-rose-900/40 dark:text-rose-400'
  if (role === 'master') return 'bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-400'
  return 'bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-400'
}

const fetchRoleUsers = async () => {
  roleLoading.value = true
  try {
    const res = await fetch('/api/roles/users', {
      headers: { Authorization: `Bearer ${getGithubToken()}` }
    })
    if (res.ok) roleUsers.value = await res.json()
  } catch (e) { console.error(e) }
  finally { roleLoading.value = false }
}

const fetchGroupRoles = async () => {
  try {
    const res = await fetch('/api/roles/groups', {
      headers: { Authorization: `Bearer ${getGithubToken()}` }
    })
    if (res.ok) groupRolesMap.value = await res.json()
  } catch (e) { console.error(e) }
}

const fetchSettings = async () => {
  try {
    const res = await fetch('/api/roles/settings', {
      headers: { Authorization: `Bearer ${getGithubToken()}` }
    })
    if (res.ok) {
      const data = await res.json()
      defaultNewUserGroups.value = data.defaultGroups ?? []
      defaultNewUserRole.value   = data.defaultRole   ?? 'slave'
    }
  } catch (e) { console.error(e) }
}

const fetchCustomRoles = async () => {
  try {
    const res = await fetch('/api/roles/custom', {
      headers: { Authorization: `Bearer ${getGithubToken()}` }
    })
    if (res.ok) customRoles.value = await res.json()
  } catch (e) { console.error(e) }
}

const openRoleModal = async () => {
  showRoleModal.value = true
  roleTab.value = 'users'
  await Promise.all([fetchRoleUsers(), fetchGroupRoles(), fetchSettings(), fetchCustomRoles()])
}

// Tambah custom role baru
const addCustomRole = async () => {
  const name = newRoleName.value.trim()
  if (!name) return
  rolePanelSaving.value = true
  try {
    const res = await fetch('/api/roles/custom', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getGithubToken()}` },
      body: JSON.stringify({
        name,
        can_write:     newRoleCanWrite.value,
        allowedGroups: newRoleGroups.value,
      })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', `Role '${data.name}' dibuat`, 'success')
      newRoleName.value     = ''
      newRoleCanWrite.value = false
      newRoleGroups.value   = []
      await fetchCustomRoles()
    } else {
      showToast('Error', data.error || 'Gagal buat role', 'error')
    }
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { rolePanelSaving.value = false }
}

// Buka edit panel untuk custom role
const openEditRole = (role) => {
  if (editingRole.value === role.name) {
    editingRole.value = null  // toggle tutup
    return
  }
  editingRole.value      = role.name
  editRoleCanWrite.value = role.can_write
  editRoleGroups.value   = [...(role.allowedGroups ?? [])]
}

const toggleNewRoleGroup = (labelId) => {
  const idx = newRoleGroups.value.indexOf(labelId)
  if (idx === -1) newRoleGroups.value.push(labelId)
  else newRoleGroups.value.splice(idx, 1)
}

const toggleEditRoleGroup = (labelId) => {
  const idx = editRoleGroups.value.indexOf(labelId)
  if (idx === -1) editRoleGroups.value.push(labelId)
  else editRoleGroups.value.splice(idx, 1)
}

const saveEditRole = async () => {
  if (!editingRole.value) return
  rolePanelSaving.value = true
  try {
    const res = await fetch(`/api/roles/custom/${encodeURIComponent(editingRole.value)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getGithubToken()}` },
      body: JSON.stringify({
        can_write:     editRoleCanWrite.value,
        allowedGroups: editRoleGroups.value,
      })
    })
    if (res.ok) {
      showToast('Success', `Role '${editingRole.value}' disimpan`, 'success')
      editingRole.value = null
      await fetchCustomRoles()
    } else {
      showToast('Error', 'Gagal simpan role', 'error')
    }
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { rolePanelSaving.value = false }
}

const deleteCustomRole = (name) => {
  showConfirm('Hapus Role', `Hapus role '${name}'? User dengan role ini akan direset ke slave.`, async () => {
    try {
      const res = await fetch(`/api/roles/custom/${encodeURIComponent(name)}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${getGithubToken()}` }
      })
      if (res.ok) {
        showToast('Success', `Role '${name}' dihapus`, 'success')
        if (editingRole.value === name) editingRole.value = null
        await fetchCustomRoles()
        await fetchRoleUsers()
      } else {
        showToast('Error', 'Gagal hapus role', 'error')
      }
    } catch (e) { showToast('Error', e.message, 'error') }
  })
}

// Semua roles yang tersedia (built-in + custom) untuk dropdown user
const allRoles = computed(() => [
  { name: 'admin',  label: 'admin',  builtin: true },
  { name: 'master', label: 'master', builtin: true },
  { name: 'slave',  label: 'slave',  builtin: true },
  ...customRoles.value.map(r => ({ name: r.name, label: r.name, builtin: false })),
])

const setUserRole = async (username, role) => {
  try {
    const res = await fetch(`/api/roles/${username}/role`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getGithubToken()}` },
      body: JSON.stringify({ role })
    })
    const data = await res.json()
    if (res.ok) {
      showToast('Success', `${username} → ${role}`, 'success')
      await fetchRoleUsers()
    } else {
      showToast('Error', data.error || 'Gagal update role', 'error')
    }
  } catch (e) { showToast('Error', e.message, 'error') }
}

// Toggle role di checklist grup
const toggleGroupRole = (labelId, role) => {
  const current = groupRolesMap.value[labelId] ?? []
  const idx = current.indexOf(role)
  if (idx === -1) {
    groupRolesMap.value = { ...groupRolesMap.value, [labelId]: [...current, role] }
  } else {
    const updated = current.filter(r => r !== role)
    groupRolesMap.value = { ...groupRolesMap.value, [labelId]: updated }
  }
}

// Apakah role diizinkan untuk label ini?
// Kosong di map = open untuk semua = semua role centang
const isRoleAllowedForLabel = (labelId, role) => {
  const current = groupRolesMap.value[labelId]
  if (!current || current.length === 0) return true  // open = semua centang
  return current.includes(role)
}

const saveGroupRoles = async (labelId) => {
  groupSaving.value = { ...groupSaving.value, [labelId]: true }
  try {
    const allowedRoles = groupRolesMap.value[labelId] ?? []
    const res = await fetch(`/api/roles/groups/${encodeURIComponent(labelId)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getGithubToken()}` },
      body: JSON.stringify({ allowedRoles })
    })
    if (res.ok) showToast('Success', 'Akses grup disimpan', 'success')
    else showToast('Error', 'Gagal simpan akses grup', 'error')
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { groupSaving.value = { ...groupSaving.value, [labelId]: false } }
}

// Toggle default group untuk user baru
const toggleDefaultGroup = (labelId) => {
  const idx = defaultNewUserGroups.value.indexOf(labelId)
  if (idx === -1) defaultNewUserGroups.value.push(labelId)
  else defaultNewUserGroups.value.splice(idx, 1)
}

const saveSettings = async () => {
  settingsSaving.value = true
  try {
    const res = await fetch('/api/roles/settings', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getGithubToken()}` },
      body: JSON.stringify({
        defaultGroups: defaultNewUserGroups.value,
        defaultRole:   defaultNewUserRole.value,
      })
    })
    if (res.ok) showToast('Success', 'Pengaturan disimpan', 'success')
    else showToast('Error', 'Gagal simpan pengaturan', 'error')
  } catch (e) { showToast('Error', e.message, 'error') }
  finally { settingsSaving.value = false }
}
</script>

<template>
  <div class="space-y-6">

    <!-- Header -->
    <div class="flex justify-between items-center animate-fade-slide-up">
      <h2 class="text-2xl font-bold text-slate-800 dark:text-slate-100 tracking-tight">Your Servers</h2>
      <div class="flex items-center gap-2">
        <!-- Manage Access (admin only) -->
        <button v-if="isAdmin" @click="openRoleModal"
          class="btn-secondary text-xs"
          title="Manage user roles & access">
          <Users class="w-3.5 h-3.5" /> Manage Access
        </button>

        <!-- Add Label (edit mode only) -->
        <button v-if="isEditMode" @click="addLabel('New Label')"
          class="btn-secondary text-xs"
          title="Add Label">
          <Tag class="w-3.5 h-3.5" /> Add Label
        </button>

        <!-- Add Server -->
        <button @click="openAddModal" class="btn-primary">
          <Plus class="w-4 h-4" /> Add Server
        </button>

        <!-- Edit / Done toggle -->
        <button @click="toggleEditMode"
          class="w-9 h-9 flex items-center justify-center rounded-lg transition-all duration-150 active:scale-90"
          :class="isEditMode
            ? 'bg-brand-500 text-white hover:bg-brand-600'
            : isDark
              ? 'text-slate-400 hover:text-slate-200 hover:bg-white/8'
              : 'text-slate-500 hover:text-slate-700 hover:bg-black/5'"
          :title="isEditMode ? 'Done' : 'Edit'">
          <Check v-if="isEditMode" class="w-4 h-4" />
          <Pencil v-else class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="servers.length === 0"
      class="text-center py-16 bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 border-dashed animate-fade-slide-up">
      <Server class="w-10 h-10 text-slate-300 dark:text-slate-600 mx-auto mb-3" />
      <h3 class="text-base font-semibold text-slate-900 dark:text-slate-100">No servers configured</h3>
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-1 mb-4">Add your first backend server to start monitoring.</p>
      <button @click="openAddModal" class="btn-primary"><Plus class="w-4 h-4" /> Add Server</button>
    </div>

    <!-- Labels + Cards container -->
    <div id="labels-container" class="space-y-8">

      <!-- Named labels -->
      <div v-for="label in labels" :key="label.id" class="space-y-3">
        <!-- Label header -->
        <div class="flex items-center gap-2 py-2">
          <!-- Drag handle for label (edit mode) -->
          <GripVertical v-if="isEditMode"
            class="label-drag-handle w-4 h-4 text-slate-400 cursor-grab active:cursor-grabbing shrink-0" />

          <!-- Rename input (edit mode, clicked) -->
          <input v-if="isEditMode && editingLabelId === label.id"
            :ref="el => { if (el) labelInputRefs[label.id] = el }"
            v-model="editingLabelName"
            class="text-base font-bold tracking-wide bg-transparent border-b outline-none w-40"
            :class="isDark ? 'text-white border-brand-400' : 'text-slate-800 border-brand-500'"
            @blur="commitRenameLabel(label.id)"
            @keydown.enter="commitRenameLabel(label.id)"
            @keydown.escape="editingLabelId = null" />

          <!-- Label name -->
          <span v-else
            class="text-base font-bold tracking-wide whitespace-nowrap"
            :class="[
              isDark ? 'text-white/90' : 'text-slate-700',
              isEditMode ? 'cursor-text hover:text-brand-400 transition-colors' : ''
            ]"
            @click="isEditMode && startRenameLabel(label)">
            {{ label.name }}
          </span>

          <!-- Restricted badge for slave -->
          <span v-if="isSlave && !isLabelAllowed(label.id)"
            class="inline-flex items-center gap-1 text-[10px] px-2 py-0.5 rounded-full font-medium"
            :class="isDark ? 'bg-slate-800 text-slate-500' : 'bg-slate-100 text-slate-400'">
            <ShieldAlert class="w-3 h-3" /> Terbatas
          </span>

          <!-- Delete label -->
          <button v-if="isEditMode" @click="handleDeleteLabel(label)"
            class="w-5 h-5 flex items-center justify-center rounded-full transition-all duration-150 shrink-0 active:scale-90"
            :class="isDark ? 'text-slate-500 hover:text-red-400 hover:bg-red-900/20' : 'text-slate-400 hover:text-red-500 hover:bg-red-50'"
            title="Remove label">
            <X class="w-3 h-3" />
          </button>
        </div>

        <div :data-label-id="label.id"
          class="card-sortable-group grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 min-h-[60px]">
          <div v-for="(s, i) in getServersForLabel(label)" :key="s.id"
            :data-server-id="s.id"
            class="rounded-2xl flex flex-col group hover:-translate-y-0.5"
            :class="[jiggleClass, !isServerAllowed(s.id) ? 'opacity-50 grayscale' : '']"
            :style="`--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle()}`"
            @mouseenter="e => !isEditMode && isServerAllowed(s.id) && e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(true)}`)"
            @mouseleave="e => !isEditMode && isServerAllowed(s.id) && e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(false)}`)"
          >
            <!-- Card Header -->
            <div class="p-5 border-b flex items-start gap-4 flex-1 relative transition-colors border-white/20 dark:border-white/8"
              :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''"
              :class="(!isServerAllowed(s.id) || isOffline(s.id)) ? 'cursor-not-allowed' : isEditMode ? 'cursor-grab active:cursor-grabbing' : 'cursor-pointer hover:bg-white/20 dark:hover:bg-white/5'"
              @click="handleCardClick(s)"
            >
              <!-- Delete button (edit mode) -->
              <button v-if="isEditMode" @click.stop="handleDeleteServer(s)"
                class="absolute top-2 right-2 w-5 h-5 flex items-center justify-center rounded-full z-10 transition-all duration-150 active:scale-90"
                :class="isDark ? 'bg-red-900/60 text-red-400 hover:bg-red-800' : 'bg-red-100 text-red-500 hover:bg-red-200'"
                title="Remove server">
                <X class="w-3 h-3" />
              </button>

              <!-- Distro Icon -->
              <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 shadow-sm transition-opacity"
                :class="[
                  s.os_name ? (getDistroColorClass(s.os_name) || 'bg-slate-100 dark:bg-slate-700') : 'bg-brand-100 dark:bg-brand-900/30',
                  isEditMode ? '' : 'group-hover:opacity-90'
                ]">
                <img v-if="getDistroIcon(s.os_name)" :src="getDistroIcon(s.os_name)" :alt="s.os_name" class="w-7 h-7 object-contain" />
                <Server v-else class="w-6 h-6 text-brand-600 dark:text-brand-400" />
              </div>

              <!-- Server info -->
              <div class="flex-1 min-w-0">
                <h3 class="font-bold text-slate-800 dark:text-slate-100 text-lg leading-tight truncate" :title="s.name">{{ s.name }}</h3>
                <div class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-1 truncate">{{ s.url }}</div>
                <div v-if="s.os_name" class="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5 truncate">{{ s.os_name }}</div>
              </div>

              <!-- Ping badge -->
              <div class="flex items-center gap-1 shrink-0 self-start">
                <div class="relative w-2 h-2 flex items-center justify-center">
                  <span v-if="isOnline(s.id)" class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60" :class="pingDotClass(s.id)"></span>
                  <span class="relative inline-flex w-1.5 h-1.5 rounded-full" :class="pingDotClass(s.id)"></span>
                </div>
                <span class="text-[10px] font-medium tracking-[0.05em] uppercase tabular-nums transition-colors duration-300" :class="pingTextClass(s.id)">
                  {{ pingLabel(s.id) }}
                </span>
              </div>
            </div>

            <!-- Footer Quick Links -->
            <div class="p-2 border-t grid grid-cols-6 divide-x border-white/20 dark:border-white/8 divide-white/20 dark:divide-white/8"
              :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''">
              <RouterLink :to="`/server/${s.id}/speedtest`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Speedtest"><Activity class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/ports`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Ports & Scan"><ShieldCheck class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/containers`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Containers"><Box class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/cloudflare`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Cloudflare"><Cloud class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/files`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="File Explorer"><FolderTree class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/settings`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Settings"><Settings class="w-4 h-4" /></RouterLink>
            </div>
          </div>
        </div>
      </div>

      <!-- Ungrouped section -->
      <div v-if="unGrouped.length > 0" class="space-y-3">
        <!-- Header ungrouped (hanya tampil jika ada label lain) -->
        <div v-if="labels.length > 0" class="flex items-center gap-2 py-2">
          <span class="text-base font-bold tracking-wide"
            :class="isDark ? 'text-white/40' : 'text-slate-400'">Ungrouped</span>
        </div>

        <!-- Ungrouped cards -->
        <div data-label-id=""
          class="card-sortable-group grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 min-h-[60px]">
          <div v-for="(s, i) in unGrouped" :key="s.id"
            :data-server-id="s.id"
            class="rounded-2xl flex flex-col group hover:-translate-y-0.5"
            :class="jiggleClass"
            :style="`--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle()}`"
            @mouseenter="e => !isEditMode && e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(true)}`)"
            @mouseleave="e => !isEditMode && e.currentTarget.setAttribute('style', `--i: ${i}; ${isOffline(s.id) ? cardStyleOffline() : cardStyle(false)}`)"
          >
            <!-- Card Header -->
            <div class="p-5 border-b flex items-start gap-4 flex-1 relative transition-colors border-white/20 dark:border-white/8"
              :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''"
              :class="(!isServerAllowed(s.id) || isOffline(s.id)) ? 'cursor-not-allowed' : isEditMode ? 'cursor-grab active:cursor-grabbing' : 'cursor-pointer hover:bg-white/20 dark:hover:bg-white/5'"
              @click="handleCardClick(s)"
            >
              <!-- Delete button (edit mode) -->
              <button v-if="isEditMode" @click.stop="handleDeleteServer(s)"
                class="absolute top-2 right-2 w-5 h-5 flex items-center justify-center rounded-full z-10 transition-all duration-150 active:scale-90"
                :class="isDark ? 'bg-red-900/60 text-red-400 hover:bg-red-800' : 'bg-red-100 text-red-500 hover:bg-red-200'"
                title="Remove server">
                <X class="w-3 h-3" />
              </button>

              <!-- Distro Icon -->
              <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 shadow-sm transition-opacity"
                :class="[
                  s.os_name ? (getDistroColorClass(s.os_name) || 'bg-slate-100 dark:bg-slate-700') : 'bg-brand-100 dark:bg-brand-900/30',
                  isEditMode ? '' : 'group-hover:opacity-90'
                ]">
                <img v-if="getDistroIcon(s.os_name)" :src="getDistroIcon(s.os_name)" :alt="s.os_name" class="w-7 h-7 object-contain" />
                <Server v-else class="w-6 h-6 text-brand-600 dark:text-brand-400" />
              </div>

              <!-- Server info -->
              <div class="flex-1 min-w-0">
                <h3 class="font-bold text-slate-800 dark:text-slate-100 text-lg leading-tight truncate" :title="s.name">
                  {{ s.name }}
                </h3>
                <div class="text-xs text-slate-500 dark:text-slate-400 font-mono mt-1 truncate">{{ s.url }}</div>
                <div v-if="s.os_name" class="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5 truncate">{{ s.os_name }}</div>
              </div>

              <!-- Ping badge -->
              <div class="flex items-center gap-1 shrink-0 self-start">
                <div class="relative w-2 h-2 flex items-center justify-center">
                  <span v-if="isOnline(s.id)" class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60"
                    :class="pingDotClass(s.id)"></span>
                  <span class="relative inline-flex w-1.5 h-1.5 rounded-full" :class="pingDotClass(s.id)"></span>
                </div>
                <span class="text-[10px] font-medium tracking-[0.05em] uppercase tabular-nums transition-colors duration-300"
                  :class="pingTextClass(s.id)">
                  {{ pingLabel(s.id) }}
                </span>
              </div>
            </div>

            <!-- Footer Quick Links -->
            <div class="p-2 border-t grid grid-cols-6 divide-x border-white/20 dark:border-white/8 divide-white/20 dark:divide-white/8"
              :style="isOffline(s.id) ? 'filter: grayscale(100%)' : ''">
              <RouterLink :to="`/server/${s.id}/speedtest`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Speedtest"><Activity class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/ports`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Ports & Scan"><ShieldCheck class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/containers`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Containers"><Box class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/cloudflare`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Cloudflare"><Cloud class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/files`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="File Explorer"><FolderTree class="w-4 h-4" /></RouterLink>
              <RouterLink :to="`/server/${s.id}/settings`" class="flex items-center justify-center p-2 text-slate-500 dark:text-slate-400 hover:text-brand-600 dark:hover:text-brand-300 hover:bg-white/30 dark:hover:bg-white/8 rounded transition-colors" title="Settings"><Settings class="w-4 h-4" /></RouterLink>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ── Add Server Modal ─────────────────────────────────────────────────── -->
    <Teleport to="body">
      <Transition name="fade">
        <div v-if="showAddModal"
          class="fixed inset-0 z-[100] flex items-center justify-center p-4 backdrop-blur-sm"
          :class="isDark ? 'bg-slate-950/70' : 'bg-slate-900/40'"
          @click.self="closeAddModal">
          <div class="w-full max-w-md rounded-2xl overflow-hidden animate-fade-slide-up"
            :class="isDark ? 'bg-slate-900 border border-slate-800' : 'bg-white border border-slate-200'"
            style="box-shadow: var(--shadow-modal)">

            <!-- Modal Header -->
            <div class="flex items-center justify-between px-6 py-4 border-b"
              :class="isDark ? 'border-slate-800' : 'border-slate-100'">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-brand-500 flex items-center justify-center shrink-0">
                  <Server class="w-4 h-4 text-white" />
                </div>
                <div>
                  <div class="text-sm font-semibold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Add New Server</div>
                  <div class="text-[11px]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Credentials verified via PAM</div>
                </div>
              </div>
              <button @click="closeAddModal"
                class="w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150 active:scale-90"
                :class="isDark ? 'text-slate-500 hover:text-slate-200 hover:bg-slate-800' : 'text-slate-400 hover:text-slate-700 hover:bg-slate-100'"
                title="Close">
                <X class="w-4 h-4" />
              </button>
            </div>

            <!-- Modal Form -->
            <div class="p-6 space-y-4">
              <div v-if="addError"
                class="flex items-start gap-2 rounded-lg px-3 py-2.5 text-xs border"
                :class="isDark ? 'bg-red-900/20 border-red-800/50 text-red-400' : 'bg-red-50 border-red-200 text-red-600'">
                <AlertCircle class="w-3.5 h-3.5 shrink-0 mt-0.5" /><span>{{ addError }}</span>
              </div>

              <div class="space-y-1.5">
                <label class="text-[10px] font-semibold uppercase tracking-[0.08em]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Server Name</label>
                <input v-model="newName" type="text" placeholder="e.g. VPS Singapore" class="input-field" :disabled="isAdding" @keydown.enter="handleAdd" />
              </div>

              <div class="space-y-1.5">
                <label class="text-[10px] font-semibold uppercase tracking-[0.08em]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Backend IP / URL</label>
                <input v-model="newUrl" type="text" placeholder="127.0.0.1:8080" class="input-field" :disabled="isAdding" @keydown.enter="handleAdd" />
                <p class="text-[11px]" :class="isDark ? 'text-slate-600' : 'text-slate-400'">http:// will be added automatically</p>
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div class="space-y-1.5">
                  <label class="text-[10px] font-semibold uppercase tracking-[0.08em]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Username</label>
                  <div class="relative">
                    <User class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 pointer-events-none" :class="isDark ? 'text-slate-600' : 'text-slate-400'" />
                    <input v-model="newUser" type="text" placeholder="ubuntu" class="input-field !pl-9" :disabled="isAdding" autocomplete="username" @keydown.enter="handleAdd" />
                  </div>
                </div>
                <div class="space-y-1.5">
                  <label class="text-[10px] font-semibold uppercase tracking-[0.08em]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Password</label>
                  <div class="relative">
                    <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 pointer-events-none" :class="isDark ? 'text-slate-600' : 'text-slate-400'" />
                    <input v-model="newPass" type="password" placeholder="OS password" class="input-field !pl-9" :disabled="isAdding" autocomplete="current-password" @keydown.enter="handleAdd" />
                  </div>
                </div>
              </div>

              <button @click="handleAdd" :disabled="isAdding" class="btn-primary w-full justify-center mt-2">
                <Loader2 v-if="isAdding" class="w-4 h-4 animate-spin" />
                <LogIn v-else class="w-4 h-4" />
                {{ isAdding ? 'Connecting...' : 'Add & Login' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- ── Role Management Modal (admin only) ──────────────────────── -->
    <Teleport to="body">
      <Transition name="fade">
        <div v-if="showRoleModal && isAdmin"
          class="fixed inset-0 z-[100] flex items-center justify-center p-4 backdrop-blur-sm"
          :class="isDark ? 'bg-slate-950/70' : 'bg-slate-900/40'"
          @click.self="showRoleModal = false">
          <div class="w-full max-w-2xl rounded-2xl overflow-hidden animate-fade-slide-up max-h-[85vh] flex flex-col"
            :class="isDark ? 'bg-slate-900 border border-slate-800' : 'bg-white border border-slate-200'"
            style="box-shadow: var(--shadow-modal)">

            <!-- Modal Header -->
            <div class="flex items-center justify-between px-6 py-4 border-b shrink-0"
              :class="isDark ? 'border-slate-800' : 'border-slate-100'">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-rose-500 flex items-center justify-center shrink-0">
                  <Users class="w-4 h-4 text-white" />
                </div>
                <div>
                  <div class="text-sm font-semibold" :class="isDark ? 'text-slate-100' : 'text-slate-800'">Manage Access</div>
                  <div class="text-[11px]" :class="isDark ? 'text-slate-500' : 'text-slate-400'">Kelola role dan akses grup per user</div>
                </div>
              </div>
              <button @click="showRoleModal = false"
                class="w-8 h-8 flex items-center justify-center rounded-lg transition-all"
                :class="isDark ? 'text-slate-500 hover:text-slate-200 hover:bg-slate-800' : 'text-slate-400 hover:text-slate-700 hover:bg-slate-100'">
                <X class="w-4 h-4" />
              </button>
            </div>

            <!-- Tabs -->
            <div class="flex border-b shrink-0 overflow-x-auto" :class="isDark ? 'border-slate-800' : 'border-slate-100'">
              <button v-for="t in [
                { id: 'users',    label: 'Users',   icon: Users },
                { id: 'roles',    label: 'Roles',   icon: ShieldAlert },
                { id: 'groups',   label: 'Grup',    icon: ShieldCheck },
                { id: 'settings', label: 'Setting', icon: Settings },
              ]" :key="t.id" @click="roleTab = t.id"
                class="px-4 py-3 text-sm font-medium border-b-2 -mb-px transition-colors whitespace-nowrap shrink-0"
                :class="roleTab === t.id
                  ? (isDark ? 'border-brand-400 text-brand-300' : 'border-brand-500 text-brand-700')
                  : (isDark ? 'border-transparent text-slate-400 hover:text-slate-200' : 'border-transparent text-slate-500 hover:text-slate-700')">
                <span class="flex items-center gap-1.5">
                  <component :is="t.icon" class="w-3.5 h-3.5" />{{ t.label }}
                </span>
              </button>
            </div>

            <!-- Tab: Users -->
            <div v-if="roleTab === 'users'" class="flex-1 overflow-y-auto p-4 space-y-2">
              <div v-if="roleLoading" class="flex justify-center py-8">
                <Loader2 class="w-6 h-6 animate-spin text-brand-500" />
              </div>
              <div v-else-if="roleUsers.length === 0" class="text-center py-8 text-sm text-slate-400">
                Belum ada user yang login.
              </div>
              <div v-for="u in roleUsers" :key="u.username"
                class="flex items-center gap-3 p-3 rounded-xl border"
                :class="isDark ? 'border-slate-700 bg-slate-800/40' : 'border-slate-100 bg-slate-50'">
                <img v-if="u.avatar_url" :src="u.avatar_url" :alt="u.username"
                  class="w-9 h-9 rounded-full object-cover shrink-0 ring-2"
                  :class="isDark ? 'ring-slate-700' : 'ring-slate-200'" />
                <div v-else class="w-9 h-9 rounded-full bg-slate-200 dark:bg-slate-700 flex items-center justify-center shrink-0">
                  <User class="w-4 h-4 text-slate-400" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5 flex-wrap">
                    <span class="text-sm font-semibold truncate" :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ u.username }}</span>
                    <span class="text-[10px] px-1.5 py-0.5 rounded-full font-bold capitalize" :class="roleBadgeClass(u.role)">{{ u.role }}</span>
                  </div>
                  <div class="text-xs mt-0.5" :class="isDark ? 'text-slate-500' : 'text-slate-400'">{{ u.name }}</div>
                </div>
                <!-- Dropdown: built-in + custom roles -->
                <select :value="u.role" @change="setUserRole(u.username, $event.target.value)"
                  class="text-xs rounded-lg border px-2 py-1.5 outline-none cursor-pointer shrink-0"
                  :class="isDark ? 'bg-slate-700 border-slate-600 text-slate-200' : 'bg-white border-slate-200 text-slate-700'">
                  <optgroup label="Built-in">
                    <option value="admin">admin</option>
                    <option value="master">master</option>
                    <option value="slave">slave</option>
                  </optgroup>
                  <optgroup v-if="customRoles.length" label="Custom">
                    <option v-for="r in customRoles" :key="r.name" :value="r.name">{{ r.name }}</option>
                  </optgroup>
                </select>
              </div>
            </div>

            <!-- Tab: Roles — kelola custom roles -->
            <div v-else-if="roleTab === 'roles'" class="flex-1 overflow-y-auto p-4 space-y-4">

              <!-- Built-in roles info -->
              <div class="rounded-xl border p-3 space-y-1.5"
                :class="isDark ? 'border-slate-700 bg-slate-800/30' : 'border-slate-200 bg-slate-50'">
                <p class="text-xs font-semibold mb-2" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Built-in Roles (tidak bisa dihapus)</p>
                <div class="flex flex-wrap gap-2">
                  <div v-for="r in [
                    { name: 'admin',  desc: 'Akses penuh + kelola role', write: true },
                    { name: 'master', desc: 'Akses penuh, tidak kelola role', write: true },
                    { name: 'slave',  desc: 'Akses terbatas, read-only', write: false },
                  ]" :key="r.name"
                    class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs border"
                    :class="isDark ? 'border-slate-700 bg-slate-800' : 'border-slate-200 bg-white'">
                    <span class="font-bold px-1.5 py-0.5 rounded-full" :class="roleBadgeClass(r.name)">{{ r.name }}</span>
                    <span :class="isDark ? 'text-slate-400' : 'text-slate-500'">{{ r.desc }}</span>
                    <span :class="r.write ? 'text-green-500' : 'text-slate-400'" class="text-[10px] font-mono">
                      {{ r.write ? 'write' : 'read-only' }}
                    </span>
                  </div>
                </div>
              </div>

              <!-- Custom roles list -->
              <div v-if="customRoles.length === 0" class="text-center py-4 text-sm text-slate-400">
                Belum ada custom role. Buat di bawah.
              </div>
              <div v-for="role in customRoles" :key="role.name" class="rounded-xl border overflow-hidden"
                :class="isDark ? 'border-slate-700' : 'border-slate-200'">
                <!-- Role header -->
                <div class="flex items-center gap-3 px-4 py-3"
                  :class="isDark ? 'bg-slate-800/60' : 'bg-slate-50'">
                  <span class="font-bold text-sm px-2 py-0.5 rounded-full"
                    :class="isDark ? 'bg-cyan-900/40 text-cyan-300' : 'bg-cyan-100 text-cyan-700'">
                    {{ role.name }}
                  </span>
                  <span class="text-xs" :class="role.can_write ? 'text-green-500' : 'text-slate-400'">
                    {{ role.can_write ? '✓ can write' : '✗ read-only' }}
                  </span>
                  <span class="text-xs ml-1" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                    {{ role.allowedGroups?.length ? `${role.allowedGroups.length} grup` : 'semua grup' }}
                  </span>
                  <div class="ml-auto flex gap-1.5">
                    <button @click="openEditRole(role)"
                      class="text-xs px-2.5 py-1 rounded-lg border transition-colors"
                      :class="editingRole === role.name
                        ? (isDark ? 'border-brand-500 bg-brand-900/40 text-brand-300' : 'border-brand-400 bg-brand-50 text-brand-700')
                        : (isDark ? 'border-slate-600 text-slate-400 hover:text-slate-200' : 'border-slate-200 text-slate-500 hover:text-slate-700')">
                      <Pencil class="w-3 h-3 inline -mt-0.5 mr-0.5" />Edit
                    </button>
                    <button @click="deleteCustomRole(role.name)"
                      class="text-xs px-2.5 py-1 rounded-lg border transition-colors"
                      :class="isDark ? 'border-slate-700 text-red-400 hover:border-red-700 hover:bg-red-900/20' : 'border-slate-200 text-red-500 hover:border-red-300 hover:bg-red-50'">
                      <Trash2 class="w-3 h-3 inline -mt-0.5" />
                    </button>
                  </div>
                </div>

                <!-- Edit panel (expandable) -->
                <div v-if="editingRole === role.name" class="p-4 space-y-3 border-t"
                  :class="isDark ? 'border-slate-700' : 'border-slate-100'">
                  <label class="flex items-center gap-2.5 cursor-pointer select-none">
                    <input type="checkbox" v-model="editRoleCanWrite" class="w-4 h-4 rounded accent-brand-500" />
                    <span class="text-sm font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                      Can write config
                    </span>
                    <span class="text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                      (bisa edit server & label global)
                    </span>
                  </label>

                  <div>
                    <p class="text-xs font-semibold mb-2" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                      Akses grup (kosong = semua grup):
                    </p>
                    <div class="grid grid-cols-2 gap-1.5">
                      <label v-for="label in labels" :key="label.id"
                        class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg border cursor-pointer text-xs transition-colors"
                        :class="isDark ? 'border-slate-700 hover:bg-slate-700/40' : 'border-slate-200 hover:bg-slate-50'">
                        <input type="checkbox"
                          :checked="editRoleGroups.includes(label.id)"
                          @change="toggleEditRoleGroup(label.id)"
                          class="w-3.5 h-3.5 rounded accent-brand-500" />
                        <span :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ label.name }}</span>
                      </label>
                    </div>
                    <p v-if="labels.length === 0" class="text-xs text-slate-400 text-center py-2">Belum ada grup.</p>
                  </div>

                  <div class="flex justify-end gap-2">
                    <button @click="editingRole = null" class="btn-secondary text-xs">Batal</button>
                    <button @click="saveEditRole" :disabled="rolePanelSaving" class="btn-primary text-xs">
                      <Loader2 v-if="rolePanelSaving" class="w-3.5 h-3.5 animate-spin" />
                      <Save v-else class="w-3.5 h-3.5" />
                      Simpan
                    </button>
                  </div>
                </div>
              </div>

              <!-- Form buat custom role baru -->
              <div class="rounded-xl border p-4 space-y-3"
                :class="isDark ? 'border-dashed border-slate-600' : 'border-dashed border-slate-300'">
                <p class="text-sm font-semibold" :class="isDark ? 'text-slate-300' : 'text-slate-600'">
                  + Buat Custom Role
                </p>

                <input v-model="newRoleName" type="text" placeholder="Nama role (misal: developer, viewer)"
                  class="input-field text-sm w-full"
                  @keydown.enter="addCustomRole" />

                <label class="flex items-center gap-2.5 cursor-pointer select-none">
                  <input type="checkbox" v-model="newRoleCanWrite" class="w-4 h-4 rounded accent-brand-500" />
                  <span class="text-sm" :class="isDark ? 'text-slate-300' : 'text-slate-700'">Can write config</span>
                </label>

                <div v-if="labels.length">
                  <p class="text-xs font-semibold mb-1.5" :class="isDark ? 'text-slate-400' : 'text-slate-500'">
                    Batasi akses grup (opsional, kosong = semua):
                  </p>
                  <div class="grid grid-cols-2 gap-1.5">
                    <label v-for="label in labels" :key="label.id"
                      class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg border cursor-pointer text-xs transition-colors"
                      :class="isDark ? 'border-slate-700 hover:bg-slate-700/40' : 'border-slate-200 hover:bg-slate-50'">
                      <input type="checkbox"
                        :checked="newRoleGroups.includes(label.id)"
                        @change="toggleNewRoleGroup(label.id)"
                        class="w-3.5 h-3.5 rounded accent-brand-500" />
                      <span :class="isDark ? 'text-slate-200' : 'text-slate-700'">{{ label.name }}</span>
                    </label>
                  </div>
                </div>

                <button @click="addCustomRole" :disabled="!newRoleName.trim() || rolePanelSaving"
                  class="btn-primary text-sm w-full justify-center disabled:opacity-50">
                  <Loader2 v-if="rolePanelSaving" class="w-4 h-4 animate-spin" />
                  <Plus v-else class="w-4 h-4" />
                  Buat Role
                </button>
              </div>
            </div>

            <!-- Tab: Groups — role siapa yang boleh akses tiap grup -->
            <div v-else-if="roleTab === 'groups'" class="flex-1 overflow-y-auto p-4 space-y-3">
              <p class="text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                Centang role yang boleh mengakses tiap grup. Jika semua dicentang = terbuka untuk semua.
              </p>

              <!-- Form tambah grup baru -->
              <div class="flex gap-2">
                <input
                  v-model="newGroupName"
                  type="text"
                  placeholder="Nama grup baru..."
                  class="input-field flex-1 text-sm"
                  @keydown.enter="addGroupFromModal"
                />
                <button @click="addGroupFromModal"
                  :disabled="!newGroupName.trim()"
                  class="btn-primary text-sm shrink-0 disabled:opacity-50 disabled:cursor-not-allowed">
                  <Plus class="w-4 h-4" /> Tambah
                </button>
              </div>

              <div v-if="labels.length === 0" class="text-center py-6 text-sm text-slate-400">
                Belum ada grup. Buat grup pertama di atas.
              </div>

              <div v-for="label in labels" :key="label.id"
                class="rounded-xl border overflow-hidden"
                :class="isDark ? 'border-slate-700' : 'border-slate-200'">

                <!-- Label header -->
                <div class="flex items-center justify-between px-4 py-3"
                  :class="isDark ? 'bg-slate-800/60' : 'bg-slate-50'">
                  <div>
                    <span class="font-semibold text-sm" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                      {{ label.name }}
                    </span>
                    <span class="ml-2 text-xs" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                      {{ label.serverIds.length }} server
                    </span>
                  </div>
                  <!-- Save button -->
                  <button @click="saveGroupRoles(label.id)"
                    :disabled="groupSaving[label.id]"
                    class="text-xs px-3 py-1 rounded-lg font-semibold transition-colors disabled:opacity-50"
                    :class="isDark ? 'bg-brand-800/40 text-brand-300 hover:bg-brand-800/70' : 'bg-brand-100 text-brand-700 hover:bg-brand-200'">
                    <span v-if="groupSaving[label.id]">Saving...</span>
                    <span v-else>Simpan</span>
                  </button>
                </div>

                <!-- Role checkboxes -->
                <div class="flex items-center gap-1 px-4 py-3 divide-x"
                  :class="isDark ? 'divide-slate-700' : 'divide-slate-100'">
                  <label v-for="role in ['admin', 'master', 'slave']" :key="role"
                    class="flex-1 flex items-center justify-center gap-2 py-1.5 cursor-pointer select-none rounded-lg transition-colors"
                    :class="isDark ? 'hover:bg-slate-700/40' : 'hover:bg-slate-50'">
                    <input type="checkbox"
                      :checked="isRoleAllowedForLabel(label.id, role)"
                      @change="toggleGroupRole(label.id, role)"
                      class="w-3.5 h-3.5 rounded accent-brand-500" />
                    <span class="text-xs font-semibold capitalize px-1.5 py-0.5 rounded-full"
                      :class="roleBadgeClass(role)">
                      {{ role }}
                    </span>
                  </label>
                </div>
              </div>
            </div>

            <!-- Tab: Settings -->
            <div v-else-if="roleTab === 'settings'" class="flex-1 overflow-y-auto p-4 space-y-5">

              <!-- Default role untuk user baru -->
              <div>
                <p class="text-sm font-semibold mb-1" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                  Default role untuk user baru
                </p>
                <p class="text-xs mb-3" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                  Setiap user baru yang pertama kali login akan mendapat role ini secara otomatis.
                </p>
                <select v-model="defaultNewUserRole"
                  class="input-field text-sm"
                  :class="isDark ? 'bg-slate-700 border-slate-600 text-slate-200' : 'bg-white border-slate-200 text-slate-700'">
                  <optgroup label="Built-in">
                    <option value="admin">admin</option>
                    <option value="master">master</option>
                    <option value="slave">slave</option>
                  </optgroup>
                  <optgroup v-if="customRoles.length" label="Custom">
                    <option v-for="r in customRoles" :key="r.name" :value="r.name">{{ r.name }}</option>
                  </optgroup>
                </select>
              </div>

              <!-- Default grup untuk user baru (untuk slave/custom read-only) -->
              <div>
                <p class="text-sm font-semibold mb-1" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                  Default grup yang bisa diakses
                </p>
                <p class="text-xs mb-3" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                  Untuk user baru dengan role slave atau custom read-only, grup ini otomatis diizinkan.
                </p>

                <div v-if="labels.length === 0" class="text-center py-4 text-sm text-slate-400">
                  Belum ada grup. Buat dulu di tab Grup.
                </div>

                <div class="space-y-2">
                  <label v-for="label in labels" :key="label.id"
                    class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors"
                    :class="isDark ? 'border-slate-700 hover:bg-slate-800' : 'border-slate-200 hover:bg-slate-50'">
                    <input type="checkbox"
                      :checked="defaultNewUserGroups.includes(label.id)"
                      @change="toggleDefaultGroup(label.id)"
                      class="w-4 h-4 rounded accent-brand-500" />
                    <span class="text-sm font-medium" :class="isDark ? 'text-slate-200' : 'text-slate-700'">
                      {{ label.name }}
                    </span>
                    <span class="text-xs ml-auto" :class="isDark ? 'text-slate-500' : 'text-slate-400'">
                      {{ label.serverIds.length }} server
                    </span>
                  </label>
                </div>
              </div>

              <button @click="saveSettings" :disabled="settingsSaving" class="btn-primary w-full justify-center">
                <Loader2 v-if="settingsSaving" class="w-4 h-4 animate-spin" />
                <Save v-else class="w-4 h-4" />
                {{ settingsSaving ? 'Menyimpan...' : 'Simpan Pengaturan' }}
              </button>
            </div>

          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
/* Jiggle animation saat edit mode */
@keyframes jiggle-enter {
  0%   { transform: rotate(0deg); }
  30%  { transform: rotate(-0.6deg); }
  60%  { transform: rotate(0.5deg); }
  100% { transform: rotate(-0.3deg); }
}

@keyframes jiggle {
  0%, 100% { transform: rotate(-0.3deg); }
  50%       { transform: rotate( 0.3deg); }
}

.edit-jiggle-enter {
  animation: jiggle-enter 0.6s ease-out forwards;
  animation-delay: calc(var(--i, 0) * 100ms);
}

.edit-jiggle {
  animation: jiggle 1.2s ease-in-out infinite alternate;
  animation-delay: calc(var(--i, 0) * 100ms);
}

/* Sortable ghost placeholder */
.sortable-ghost {
  opacity: 0.3;
  border-radius: 1rem;
}

/* Fade modal transition */
.fade-enter-active,
.fade-leave-active { transition: opacity 0.2s ease; }
.fade-enter-from,
.fade-leave-to     { opacity: 0; }
</style>
