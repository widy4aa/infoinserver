<script setup>
import { ref, computed, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { Users, UserPlus, Key, Shield, Trash2, Loader2, X, Lock } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

const users = ref([])
const groups = ref([])
const isLoading = ref(true)

const showSystemUsers = ref(false)

const filteredUsers = computed(() => {
  if (showSystemUsers.value) return users.value
  return users.value.filter(u => !u.is_system)
})

// Modals State
const showAddModal = ref(false)
const showPassModal = ref(false)
const showGroupModal = ref(false)

// Forms State
const formUser = ref({ username: '', password: '' })
const formPass = ref({ username: '', password: '' })
const formGroup = ref({ username: '', selected: [] })

const fetchUsersAndGroups = async () => {
  isLoading.value = true
  try {
    const [resU, resG] = await Promise.all([
      apiFetch(`${getActiveServerUrl()}/api/users`),
      apiFetch(`${getActiveServerUrl()}/api/groups`)
    ])
    if (resU.ok) users.value = await resU.json()
    if (resG.ok) groups.value = await resG.json()
  } catch (e) {
    showToast("Error", "Failed to load users & groups", "error")
  } finally {
    isLoading.value = false
  }
}

// ── ADD USER ──
const handleAddUser = async () => {
  if (!formUser.value.username) return showToast("Warning", "Username required", "warning")
  
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formUser.value)
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      showAddModal.value = false
      fetchUsersAndGroups()
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

// ── SSH KEYS ──
const showSshModal = ref(false)
const sshTargetUser = ref('')
const sshKeys = ref([])
const newSshKey = ref('')
const isLoadingSsh = ref(false)

const openSshModal = async (username) => {
  sshTargetUser.value = username
  showSshModal.value = true
  await fetchSshKeys()
}

const fetchSshKeys = async () => {
  isLoadingSsh.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users/${sshTargetUser.value}/ssh`)
    if (res.ok) {
      sshKeys.value = await res.json()
    }
  } catch (e) {
    showToast("Error", "Failed to fetch SSH keys", "error")
  } finally {
    isLoadingSsh.value = false
  }
}

const addSshKey = async () => {
  if (!newSshKey.value.trim()) return
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users/${sshTargetUser.value}/ssh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key: newSshKey.value })
    })
    if (res.ok) {
      showToast("Success", "SSH Key added", "success")
      newSshKey.value = ''
      await fetchSshKeys()
    } else {
      showToast("Error", await res.text(), "error")
    }
  } catch (e) {
    showToast("Error", "Failed to add SSH key", "error")
  }
}

const deleteSshKey = async (key) => {
  showConfirm("Remove Key", "Remove this SSH key from authorized_keys?", async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/users/${sshTargetUser.value}/ssh`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ key })
      })
      if (res.ok) {
        showToast("Success", "SSH Key removed", "success")
        await fetchSshKeys()
      } else {
        showToast("Error", await res.text(), "error")
      }
    } catch (e) {
      showToast("Error", "Failed to remove SSH key", "error")
    }
  })
}

// ── CHANGE PASSWORD ──
const openPassModal = (username) => {
  formPass.value = { username, password: '' }
  showPassModal.value = true
}

const handleChangePassword = async () => {
  if (!formPass.value.password) return showToast("Warning", "Password required", "warning")
  
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users/${formPass.value.username}/password`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: formPass.value.password })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      showPassModal.value = false
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

// ── MANAGE GROUPS ──
const openGroupModal = (user) => {
  formGroup.value = { 
    username: user.username, 
    selected: [...user.groups] 
  }
  showGroupModal.value = true
}

const toggleGroupSelection = (groupName) => {
  const idx = formGroup.value.selected.indexOf(groupName)
  if (idx > -1) formGroup.value.selected.splice(idx, 1)
  else formGroup.value.selected.push(groupName)
}

const handleUpdateGroups = async () => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users/${formGroup.value.username}/groups`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ groups: formGroup.value.selected })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      showGroupModal.value = false
      fetchUsersAndGroups()
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

// ── DELETE USER ──
const handleDeleteUser = (username) => {
  // Pakai default native confirm agar bisa dapat input checkbox untuk delete home dir
  // Karena global Toast confirm kita saat ini tidak support custom checkbox
  const removeHome = window.confirm(`Apakah Anda yakin ingin menghapus user '${username}'?\n\nTekan OK untuk menghapus user.\n(Home directory akan ikut dihapus jika klik OK, Cancel untuk batal)`)
  
  if (!removeHome) return // Batal
  
  // Asumsi jika OK, kita juga hapus home. Jika butuh pisah, user bisa pakai CLI.
  // Untuk simplicity, kita set remove_home = true jika OK ditekan.
  executeDelete(username, true)
}

const executeDelete = async (username, removeHome) => {
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/users/${username}?remove_home=${removeHome}`, {
      method: 'DELETE'
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      fetchUsersAndGroups()
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  }
}

onMounted(fetchUsersAndGroups)
</script>

<template>
  <div class="space-y-6">
    <section class="card">
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
        <div class="flex items-center gap-3">
          <h2 class="card-title mb-0"><Users class="w-5 h-5 text-brand-500" /> Users &amp; Groups</h2>
        </div>
        <div class="flex items-center gap-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" v-model="showSystemUsers" class="rounded text-brand-600 focus:ring-brand-500 w-4 h-4" />
            <span class="text-sm font-medium text-slate-600">Show System Users</span>
          </label>
          <button @click="formUser = {username: '', password: ''}; showAddModal = true" class="btn-primary whitespace-nowrap">
            <UserPlus class="w-4 h-4" /> Add User
          </button>
        </div>
      </div>

      <div v-if="isLoading" class="flex justify-center p-12">
        <Loader2 class="w-8 h-8 animate-spin text-brand-500" />
      </div>

      <div v-else class="overflow-x-auto">
        <table class="w-full relative">
          <thead class="bg-slate-50 border-y border-slate-200">
            <tr>
              <th class="table-th">User</th>
              <th class="table-th">Groups</th>
              <th class="table-th">Home &amp; Shell</th>
              <th class="table-th text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="user in filteredUsers" :key="user.username" class="hover:bg-slate-50">
              <td class="table-td">
                <div class="flex items-center gap-2">
                  <span class="font-bold text-slate-800">{{ user.username }}</span>
                  <span v-if="user.is_system" class="px-1.5 py-0.5 bg-slate-200 text-slate-500 text-[10px] font-bold rounded">SYS</span>
                </div>
                <div class="text-[10px] text-slate-500 font-mono mt-0.5">UID: {{ user.uid }} | GID: {{ user.gid }}</div>
              </td>
              <td class="table-td">
                <div class="flex flex-wrap gap-1">
                  <span v-for="g in user.groups" :key="g" class="px-1.5 py-0.5 bg-blue-50 text-blue-600 border border-blue-100 rounded text-[10px] font-mono">
                    {{ g }}
                  </span>
                </div>
              </td>
              <td class="table-td text-xs text-slate-600 font-mono">
                <div>{{ user.home }}</div>
                <div class="text-[10px] text-slate-400">{{ user.shell }}</div>
              </td>
              <td class="table-td text-right">
                <div class="flex items-center justify-end gap-1.5">
                  <button @click="openSshModal(user.username)" class="p-1.5 rounded bg-emerald-100 text-emerald-600 hover:bg-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-400 dark:hover:bg-emerald-900/50" title="Manage SSH Keys"><Key class="w-3.5 h-3.5" /></button>
                  <button @click="openPassModal(user.username)" class="btn-icon-amber" title="Change Password"><Lock class="w-3.5 h-3.5" /></button>
                  <button @click="openGroupModal(user)" class="btn-icon-blue" title="Manage Groups"><Shield class="w-3.5 h-3.5" /></button>
                  <button @click="handleDeleteUser(user.username)" class="btn-icon-red" title="Delete User"><Trash2 class="w-3.5 h-3.5" /></button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Modal: Add User -->
    <Teleport to="body">
      <div v-if="showAddModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-xl w-full max-w-md overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><UserPlus class="w-4 h-4 text-brand-500"/> Add New User</h3>
            <button @click="showAddModal = false" class="transition-colors" :class="isDark ? 'text-slate-400 hover:text-slate-200' : 'text-slate-400 hover:text-slate-600'"><X class="w-4 h-4"/></button>
          </div>
          <div class="p-5 space-y-4">
            <div>
              <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Username</label>
              <input v-model="formUser.username" type="text" class="input-field" placeholder="e.g. johndoe">
            </div>
            <div>
              <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Password</label>
              <input v-model="formUser.password" type="password" class="input-field" placeholder="Leave empty for no password">
            </div>
            <div class="pt-2 flex justify-end gap-2">
              <button @click="showAddModal = false" class="btn-outline">Cancel</button>
              <button @click="handleAddUser" class="btn-primary">Create User</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Modal: Change Password -->
    <Teleport to="body">
      <div v-if="showPassModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-xl w-full max-w-md overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><Key class="w-4 h-4 text-amber-500"/> Change Password</h3>
            <button @click="showPassModal = false" class="transition-colors" :class="isDark ? 'text-slate-400 hover:text-slate-200' : 'text-slate-400 hover:text-slate-600'"><X class="w-4 h-4"/></button>
          </div>
          <div class="p-5 space-y-4">
            <p class="text-sm" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Set new password for <strong :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ formPass.username }}</strong>.</p>
            <div>
              <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">New Password</label>
              <input v-model="formPass.password" type="password" class="input-field" placeholder="Enter new password" @keydown.enter="handleChangePassword">
            </div>
            <div class="pt-2 flex justify-end gap-2">
              <button @click="showPassModal = false" class="btn-outline">Cancel</button>
              <button @click="handleChangePassword" class="btn-warning !text-amber-900">Update Password</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Modal: Manage Groups -->
    <Teleport to="body">
      <div v-if="showGroupModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-xl w-full max-w-lg overflow-hidden flex flex-col max-h-[80vh]" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center shrink-0" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><Shield class="w-4 h-4 text-blue-500"/> Manage Groups</h3>
            <button @click="showGroupModal = false" class="transition-colors" :class="isDark ? 'text-slate-400 hover:text-slate-200' : 'text-slate-400 hover:text-slate-600'"><X class="w-4 h-4"/></button>
          </div>
          <div class="p-5 overflow-y-auto">
            <p class="text-sm mb-4" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Select secondary groups for <strong :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ formGroup.username }}</strong>.</p>
            
            <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
              <label v-for="g in groups" :key="g.name" class="flex items-center gap-2 p-2 border rounded cursor-pointer transition-colors" :class="[isDark ? 'border-slate-700 hover:bg-slate-700' : 'border-slate-200 hover:bg-slate-50', formGroup.selected.includes(g.name) ? (isDark ? 'bg-blue-900/20 border-blue-800' : 'bg-blue-50 border-blue-200') : '']">
                <input type="checkbox" :checked="formGroup.selected.includes(g.name)" @change="toggleGroupSelection(g.name)" class="rounded focus:ring-brand-500" :class="isDark ? 'bg-slate-800 border-slate-600' : 'text-brand-600'">
                <span class="text-xs font-mono truncate" :class="isDark ? 'text-slate-300' : 'text-slate-700'" :title="g.name">{{ g.name }}</span>
              </label>
            </div>
          </div>
          <div class="p-4 border-t flex justify-end gap-2 shrink-0" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <button @click="showGroupModal = false" class="btn-outline">Cancel</button>
            <button @click="handleUpdateGroups" class="btn-primary">Save Groups</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Modal: SSH Keys -->
    <Teleport to="body">
      <div v-if="showSshModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-xl w-full max-w-2xl overflow-hidden flex flex-col h-[80vh]" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center shrink-0" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'">
              <Key class="w-4 h-4 text-emerald-500"/> SSH Keys for {{ sshTargetUser }}
            </h3>
            <button @click="showSshModal = false" class="transition-colors" :class="isDark ? 'text-slate-400 hover:text-slate-200' : 'text-slate-400 hover:text-slate-600'"><X class="w-4 h-4"/></button>
          </div>
          
          <div class="p-4 flex-1 overflow-y-auto space-y-4">
            <!-- Add new key -->
            <div>
              <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-600'">Add New Public Key</label>
              <textarea v-model="newSshKey" rows="3" class="input-field w-full font-mono text-[10px]" placeholder="ssh-rsa AAAAB3..."></textarea>
              <div class="mt-2 text-right">
                <button @click="addSshKey" class="btn-primary py-1 px-3 text-xs">Add Key</button>
              </div>
            </div>

            <hr :class="isDark ? 'border-slate-700' : 'border-slate-200'" />

            <!-- List keys -->
            <div>
              <label class="block text-xs font-semibold mb-2" :class="isDark ? 'text-slate-400' : 'text-slate-600'">Authorized Keys ({{ sshKeys.length }})</label>
              <div v-if="isLoadingSsh" class="text-center py-4"><Loader2 class="w-5 h-5 animate-spin mx-auto text-brand-500" /></div>
              <div v-else-if="sshKeys.length === 0" class="text-center py-6 italic text-slate-500 text-sm">No SSH keys found for this user.</div>
              <div v-else class="space-y-2">
                <div v-for="(key, i) in sshKeys" :key="i" class="flex gap-2 items-start p-2 rounded border" :class="isDark ? 'bg-slate-900/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
                  <div class="flex-1 overflow-x-auto">
                    <pre class="font-mono text-[10px] whitespace-pre-wrap break-all" :class="isDark ? 'text-slate-300' : 'text-slate-600'">{{ key }}</pre>
                  </div>
                  <button @click="deleteSshKey(key)" class="shrink-0 p-1.5 text-red-500 hover:bg-red-500/20 rounded" title="Remove Key"><Trash2 class="w-3.5 h-3.5" /></button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

  </div>
</template>
