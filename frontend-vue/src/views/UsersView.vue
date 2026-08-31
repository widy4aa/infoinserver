<script setup>
import { ref, computed, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { useThemeStore } from '../stores/themeStore'
import { Users, UserPlus, Key, Shield, Trash2, Loader2, X, Lock, UsersRound } from 'lucide-vue-next'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

// ── TABS STATE ──
const activeTab = ref('users') // 'users', 'groups'

// ── DATA STATE ──
const users = ref([])
const groups = ref([])
const isLoading = ref(true)

const fetchUsersAndGroups = async () => {
  try {
    isLoading.value = true
    const [uRes, gRes] = await Promise.all([
      apiFetch(`${getActiveServerUrl()}/api/users`),
      apiFetch(`${getActiveServerUrl()}/api/groups`)
    ])
    
    if (uRes.ok && gRes.ok) {
      users.value = await uRes.json()
      groups.value = await gRes.json()
    } else {
      showToast("Error", "Failed to load users or groups", "error")
    }
  } catch (e) {
    showToast("Error", "API Error", "error")
  } finally {
    isLoading.value = false
  }
}

// ── CREATE NEW GROUP (Tab Groups) ──
const newGroupName = ref('')
const isCreatingGroup = ref(false)

const handleCreateGroup = async () => {
  if (!newGroupName.value.trim()) return
  isCreatingGroup.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/groups`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: newGroupName.value.trim() })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", data.message, "success")
      newGroupName.value = ''
      await fetchUsersAndGroups()
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  } finally {
    isCreatingGroup.value = false
  }
}

// ── DELETE GROUP (Tab Groups) ──
const handleDeleteGroup = (groupname) => {
  showConfirm("Hapus Group", `Yakin ingin menghapus grup Linux: ${groupname}?`, async () => {
    try {
      const res = await apiFetch(`${getActiveServerUrl()}/api/groups/${groupname}`, { method: 'DELETE' })
      const data = await res.json()
      if (res.ok) {
        showToast("Success", data.message, "success")
        fetchUsersAndGroups()
      } else throw new Error(data.error || data)
    } catch(e) {
      showToast("Error", e.message, "error")
    }
  })
}

// ── CREATE USER ──
const showAddModal = ref(false)
const formUser = ref({ username: '', password: '', is_sudo: false })
const isSubmittingUser = ref(false)

const handleCreateUser = async () => {
  if (!formUser.value.username) return showToast("Warning", "Username required", "warning")
  isSubmittingUser.value = true
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
      formUser.value = { username: '', password: '', is_sudo: false }
      fetchUsersAndGroups()
    } else throw new Error(data.error || data)
  } catch (e) {
    showToast("Error", e.message, "error")
  } finally {
    isSubmittingUser.value = false
  }
}

// ── CHANGE PASSWORD ──
const showPassModal = ref(false)
const formPass = ref({ username: '', password: '' })

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

// ── MANAGE SECONDARY GROUPS ──
const showGroupModal = ref(false)
const formGroup = ref({ username: '', selected: [] })

const openGroupModal = (user) => {
  formGroup.value.username = user.username
  formGroup.value.selected = user.groups.filter(g => g !== user.username)
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
  const willDeleteHome = confirm(`Hapus home directory untuk user ${username} juga? (Cancel = Keep home dir, OK = Delete home dir)`)
  const url = `${getActiveServerUrl()}/api/users/${username}?remove_home=${willDeleteHome}`
  
  showConfirm("Hapus User", `Yakin ingin menghapus user Linux: ${username}?`, async () => {
    try {
      const res = await apiFetch(url, { method: 'DELETE' })
      const data = await res.json()
      if (res.ok) {
        showToast("Success", data.message, "success")
        fetchUsersAndGroups()
      } else throw new Error(data.error || data)
    } catch(e) {
      showToast("Error", e.message, "error")
    }
  })
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

// ── UTILS ──
const isSystemUser = (uid) => uid < 1000 && uid !== 0

const displayUsers = computed(() => {
  return users.value.filter(u => !isSystemUser(u.uid))
})

const displayGroups = computed(() => {
  // Hide internal linux groups by default to keep it clean, showing >=1000 + root
  return groups.value.filter(g => g.gid >= 1000 || g.gid === 0 || g.name === 'wheel' || g.name === 'sudo' || g.name === 'docker')
})

onMounted(() => {
  fetchUsersAndGroups()
})
</script>

<template>
  <div class="space-y-4">
    <!-- Tabs Header -->
    <div class="flex items-center gap-2 border-b" :class="isDark ? 'border-slate-800' : 'border-slate-200'">
      <button @click="activeTab = 'users'" class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="activeTab === 'users' ? 'border-brand-500 text-brand-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
        <div class="flex items-center gap-2"><Users class="w-4 h-4"/> Linux Users</div>
      </button>
      <button @click="activeTab = 'groups'" class="px-4 py-2 text-sm font-semibold transition-colors border-b-2"
        :class="activeTab === 'groups' ? 'border-brand-500 text-brand-500' : 'border-transparent text-slate-500 hover:text-slate-700'">
        <div class="flex items-center gap-2"><UsersRound class="w-4 h-4"/> User Groups</div>
      </button>
    </div>

    <!-- ── TAB 1: USERS ── -->
    <section v-if="activeTab === 'users'" class="card">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h2 class="card-title mb-0"><Users class="w-5 h-5 text-brand-500" /> OS Users Management</h2>
          <p class="text-xs text-slate-500 mt-1">Manage system users (UID &ge; 1000 and root). System users are hidden.</p>
        </div>
        <button @click="showAddModal = true" class="btn-primary whitespace-nowrap">
          <UserPlus class="w-4 h-4" /> Add User
        </button>
      </div>

      <div v-if="isLoading" class="p-8 flex justify-center"><Loader2 class="w-6 h-6 animate-spin text-brand-500" /></div>
      
      <div v-else class="overflow-x-auto">
        <table class="w-full">
          <thead class="border-b" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
            <tr>
              <th class="table-th">User / UID</th>
              <th class="table-th">Groups</th>
              <th class="table-th">Home & Shell</th>
              <th class="table-th text-right">Action</th>
            </tr>
          </thead>
          <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
            <tr v-for="user in displayUsers" :key="user.uid" class="hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
              <td class="table-td">
                <div class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-200' : 'text-slate-800'">
                  {{ user.username }}
                  <span v-if="user.uid === 0" class="px-1.5 py-0.5 rounded text-[9px] font-bold bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400">ROOT</span>
                  <span v-else-if="user.groups.includes('wheel') || user.groups.includes('sudo')" class="px-1.5 py-0.5 rounded text-[9px] font-bold bg-amber-100 text-amber-600 dark:bg-amber-900/30 dark:text-amber-400">SUDO</span>
                </div>
                <div class="text-xs text-slate-500 mt-0.5">UID: {{ user.uid }}</div>
              </td>
              <td class="table-td">
                <div class="flex flex-wrap gap-1">
                  <span v-for="g in user.groups" :key="g" class="px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200 text-[10px] dark:bg-slate-800 dark:border-slate-700 dark:text-slate-400">
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
                  <button @click="handleDeleteUser(user.username)" class="btn-icon-red" title="Delete User" :disabled="user.uid === 0"><Trash2 class="w-3.5 h-3.5" /></button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- ── TAB 2: GROUPS ── -->
    <section v-if="activeTab === 'groups'" class="card">
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
        <div>
          <h2 class="card-title mb-0"><UsersRound class="w-5 h-5 text-brand-500" /> OS Groups Management</h2>
          <p class="text-xs text-slate-500 mt-1">Manage user groups (GID &ge; 1000). System groups are hidden by default.</p>
        </div>
        <div class="flex gap-2">
          <input v-model="newGroupName" type="text" placeholder="Create new group..." class="input-field text-sm" :disabled="isCreatingGroup" @keyup.enter="handleCreateGroup">
          <button @click="handleCreateGroup" class="btn-primary whitespace-nowrap" :disabled="isCreatingGroup">
            <Loader2 v-if="isCreatingGroup" class="w-4 h-4 animate-spin" />
            <span v-else>Add Group</span>
          </button>
        </div>
      </div>

      <div v-if="isLoading" class="p-8 flex justify-center"><Loader2 class="w-6 h-6 animate-spin text-brand-500" /></div>

      <div v-else class="overflow-x-auto">
        <table class="w-full">
          <thead class="border-b" :class="isDark ? 'bg-slate-800/50 border-slate-700' : 'bg-slate-50 border-slate-200'">
            <tr>
              <th class="table-th">Group Name</th>
              <th class="table-th w-24 text-center">GID</th>
              <th class="table-th w-1/2">Members</th>
              <th class="table-th text-right">Action</th>
            </tr>
          </thead>
          <tbody class="divide-y" :class="isDark ? 'divide-slate-800' : 'divide-slate-100'">
            <tr v-for="group in displayGroups" :key="group.gid" class="hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors">
              <td class="table-td font-bold" :class="isDark ? 'text-slate-200' : 'text-slate-800'">
                {{ group.name }}
              </td>
              <td class="table-td font-mono text-xs text-center text-slate-500">
                {{ group.gid }}
              </td>
              <td class="table-td">
                <div class="flex flex-wrap gap-1">
                  <span v-if="group.members.length === 0" class="text-xs italic text-slate-400">No members</span>
                  <span v-else v-for="m in group.members" :key="m" class="px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200 text-[10px] dark:bg-slate-800 dark:border-slate-700 dark:text-slate-400">
                    {{ m }}
                  </span>
                </div>
              </td>
              <td class="table-td text-right">
                <button @click="handleDeleteGroup(group.name)" class="btn-icon-red" title="Delete Group" :disabled="group.gid === 0 || group.name === 'wheel' || group.name === 'sudo'">
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
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
              <input v-model="formUser.username" type="text" class="input-field w-full" placeholder="e.g. john" />
            </div>
            <div>
              <label class="block text-xs font-semibold mb-1" :class="isDark ? 'text-slate-400' : 'text-slate-500'">Password</label>
              <input v-model="formUser.password" type="password" class="input-field w-full" placeholder="Leave empty for no password" />
            </div>
            <label class="flex items-center gap-2 cursor-pointer mt-2">
              <input v-model="formUser.is_sudo" type="checkbox" class="rounded text-brand-600 focus:ring-brand-500" :class="isDark ? 'bg-slate-900 border-slate-700' : ''" />
              <span class="text-sm font-medium" :class="isDark ? 'text-slate-300' : 'text-slate-700'">Grant sudo/wheel privileges</span>
            </label>
          </div>
          <div class="p-4 border-t flex justify-end gap-2" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <button @click="showAddModal = false" class="btn-outline">Cancel</button>
            <button @click="handleCreateUser" class="btn-primary" :disabled="isSubmittingUser">
              <Loader2 v-if="isSubmittingUser" class="w-4 h-4 animate-spin" /> Create
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Modal: Change Password -->
    <Teleport to="body">
      <div v-if="showPassModal" class="fixed inset-0 z-[100] backdrop-blur-sm flex items-center justify-center p-4" :class="isDark ? 'bg-slate-950/80' : 'bg-slate-900/50'">
        <div class="rounded-xl shadow-xl w-full max-w-sm overflow-hidden" :class="isDark ? 'bg-slate-800' : 'bg-white'">
          <div class="p-4 border-b flex justify-between items-center" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <h3 class="font-bold flex items-center gap-2" :class="isDark ? 'text-slate-100' : 'text-slate-800'"><Lock class="w-4 h-4 text-amber-500"/> Change Password</h3>
            <button @click="showPassModal = false" class="transition-colors" :class="isDark ? 'text-slate-400 hover:text-slate-200' : 'text-slate-400 hover:text-slate-600'"><X class="w-4 h-4"/></button>
          </div>
          <div class="p-5">
            <p class="text-sm mb-4" :class="isDark ? 'text-slate-300' : 'text-slate-600'">Enter new password for <strong :class="isDark ? 'text-slate-100' : 'text-slate-800'">{{ formPass.username }}</strong>:</p>
            <input v-model="formPass.password" type="password" class="input-field w-full" placeholder="New password" @keyup.enter="handleChangePassword" />
          </div>
          <div class="p-4 border-t flex justify-end gap-2" :class="isDark ? 'bg-slate-900 border-slate-700' : 'bg-slate-50 border-slate-100'">
            <button @click="showPassModal = false" class="btn-outline">Cancel</button>
            <button @click="handleChangePassword" class="btn-primary bg-amber-500 hover:bg-amber-600 border-none text-white">Save</button>
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