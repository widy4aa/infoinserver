<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import { useServerStore } from '../stores/serverStore'
import { useToastStore } from '../stores/toastStore'
import { Clock, Loader2, Save, Trash2, Edit3, Plus, X } from 'lucide-vue-next'
import { useThemeStore } from '../stores/themeStore'

const { apiFetch } = useApi()
const { getActiveServerUrl } = useServerStore()
const { showToast, showConfirm } = useToastStore()
const { isDark } = useThemeStore()

const crontabRaw = ref('')
const parsedJobs = ref([])
const isLoading = ref(true)

const showModal = ref(false)
const isEditing = ref(false)
const editIndex = ref(-1)

// Form State
const formJob = ref({
  minute: '*',
  hour: '*',
  dayMonth: '*',
  month: '*',
  dayWeek: '*',
  command: ''
})

const fetchCron = async () => {
  isLoading.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cron`)
    if (res.ok) {
      const data = await res.json()
      crontabRaw.value = data.crontab
      parseCron(data.crontab)
    }
  } catch (e) {
    showToast("Error", "Failed to fetch cronjobs", "error")
  } finally {
    isLoading.value = false
  }
}

const parseCron = (rawText) => {
  const lines = rawText.split('\n')
  const jobs = []
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim()
    if (!line || line.startsWith('#') || line.includes('=')) {
      // It's a comment, empty line, or environment variable like PATH=/bin
      jobs.push({ type: 'comment', raw: line })
    } else {
      // Basic split
      const parts = line.split(/\s+/)
      if (parts.length >= 6) {
        jobs.push({
          type: 'job',
          minute: parts[0],
          hour: parts[1],
          dayMonth: parts[2],
          month: parts[3],
          dayWeek: parts[4],
          command: parts.slice(5).join(' '),
          raw: line
        })
      } else {
        jobs.push({ type: 'comment', raw: line })
      }
    }
  }
  parsedJobs.value = jobs
}

const saveCrontab = async (newRawString) => {
  isLoading.value = true
  try {
    const res = await apiFetch(`${getActiveServerUrl()}/api/cron`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ crontab: newRawString })
    })
    const data = await res.json()
    if (res.ok) {
      showToast("Success", "Crontab updated successfully", "success")
      fetchCron()
    } else throw new Error(data.error || "Failed")
  } catch (e) {
    showToast("Error", e.message, "error")
    isLoading.value = false
  }
}

const buildRawFromParsed = (jobsArray) => {
  return jobsArray.map(j => {
    if (j.type === 'comment') return j.raw
    return `${j.minute} ${j.hour} ${j.dayMonth} ${j.month} ${j.dayWeek} ${j.command}`
  }).join('\n')
}

// ── ACTIONS ──

const openAddModal = () => {
  isEditing.value = false
  editIndex.value = -1
  formJob.value = { minute: '*', hour: '*', dayMonth: '*', month: '*', dayWeek: '*', command: '' }
  showModal.value = true
}

const openEditModal = (job, index) => {
  isEditing.value = true
  editIndex.value = index
  formJob.value = { ...job }
  showModal.value = true
}

const submitJob = () => {
  if (!formJob.value.command) return showToast("Warning", "Command is required", "warning")
  
  const newJob = {
    type: 'job',
    ...formJob.value
  }

  const newJobs = [...parsedJobs.value]
  
  if (isEditing.value) {
    newJobs[editIndex.value] = newJob
  } else {
    newJobs.push(newJob)
  }

  showModal.value = false
  saveCrontab(buildRawFromParsed(newJobs))
}

const deleteJob = (index) => {
  showConfirm("Hapus Cronjob", "Apakah Anda yakin ingin menghapus jadwal ini?", () => {
    const newJobs = [...parsedJobs.value]
    newJobs.splice(index, 1)
    saveCrontab(buildRawFromParsed(newJobs))
  })
}

onMounted(fetchCron)
</script>

<template>
  <div class="space-y-6">
    <section class="card">
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
        <div>
          <h2 class="card-title mb-1"><Clock class="w-5 h-5 text-brand-500" /> Root Cronjobs</h2>
          <p class="text-xs text-slate-500">Manage scheduled tasks for the root user. Edits /etc/crontab.</p>
        </div>
        <button @click="openAddModal" class="btn-primary whitespace-nowrap">
          <Plus class="w-4 h-4" /> Add Cronjob
        </button>
      </div>

      <div v-if="isLoading" class="flex justify-center p-12">
        <Loader2 class="w-8 h-8 animate-spin text-brand-500" />
      </div>

      <div v-else class="overflow-x-auto border border-slate-200 rounded-lg">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200">
            <tr>
              <th class="table-th w-32">Schedule</th>
              <th class="table-th">Command</th>
              <th class="table-th text-right w-24">Action</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="(job, index) in parsedJobs" :key="index">
              <!-- Render hanya type 'job' di tabel UI. Comments disimpan di background -->
              <tr v-if="job.type === 'job'" class="hover:bg-slate-50 border-b border-slate-100 last:border-0 group">
                <td class="table-td">
                  <div class="flex gap-1 font-mono text-xs font-bold text-slate-700 bg-slate-100 px-2 py-1 rounded w-max">
                    <span class="w-3 text-center text-blue-600" title="Minute">{{ job.minute }}</span>
                    <span class="w-3 text-center text-green-600" title="Hour">{{ job.hour }}</span>
                    <span class="w-3 text-center text-purple-600" title="Day of Month">{{ job.dayMonth }}</span>
                    <span class="w-3 text-center text-amber-600" title="Month">{{ job.month }}</span>
                    <span class="w-3 text-center text-red-600" title="Day of Week">{{ job.dayWeek }}</span>
                  </div>
                </td>
                <td class="table-td font-mono text-xs text-slate-600 break-all">{{ job.command }}</td>
                <td class="table-td text-right">
                  <div class="flex items-center justify-end gap-1.5 opacity-50 group-hover:opacity-100 transition-opacity">
                    <button @click="openEditModal(job, index)" class="btn-icon-blue" title="Edit"><Edit3 class="w-3 h-3" /></button>
                    <button @click="deleteJob(index)" class="btn-icon-red" title="Delete"><Trash2 class="w-3 h-3" /></button>
                  </div>
                </td>
              </tr>
            </template>
            <tr v-if="!parsedJobs.some(j => j.type === 'job')">
              <td colspan="3" class="text-center p-8 text-slate-500">No scheduled tasks found.</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Modal Form -->
    <Teleport to="body">
      <div v-if="showModal" class="fixed inset-0 z-[100] bg-slate-900/50 backdrop-blur-sm flex items-center justify-center p-4">
        <div class="bg-white rounded-xl shadow-xl w-full max-w-lg overflow-hidden flex flex-col">
          <div class="p-4 border-b border-slate-100 flex justify-between items-center bg-slate-50 shrink-0">
            <h3 class="font-bold flex items-center gap-2"><Clock class="w-4 h-4 text-brand-500"/> {{ isEditing ? 'Edit' : 'Add' }} Cronjob</h3>
            <button @click="showModal = false" class="text-slate-400 hover:text-slate-600"><X class="w-4 h-4"/></button>
          </div>
          
          <div class="p-5 space-y-4">
            <div class="grid grid-cols-5 gap-2">
              <div>
                <label class="block text-[10px] font-bold text-blue-600 uppercase mb-1">Minute</label>
                <input v-model="formJob.minute" type="text" class="input-field text-center font-mono" placeholder="*">
              </div>
              <div>
                <label class="block text-[10px] font-bold text-green-600 uppercase mb-1">Hour</label>
                <input v-model="formJob.hour" type="text" class="input-field text-center font-mono" placeholder="*">
              </div>
              <div>
                <label class="block text-[10px] font-bold text-purple-600 uppercase mb-1">Day</label>
                <input v-model="formJob.dayMonth" type="text" class="input-field text-center font-mono" placeholder="*">
              </div>
              <div>
                <label class="block text-[10px] font-bold text-amber-600 uppercase mb-1">Month</label>
                <input v-model="formJob.month" type="text" class="input-field text-center font-mono" placeholder="*">
              </div>
              <div>
                <label class="block text-[10px] font-bold text-red-600 uppercase mb-1">DOW</label>
                <input v-model="formJob.dayWeek" type="text" class="input-field text-center font-mono" placeholder="*">
              </div>
            </div>

            <div>
              <label class="block text-xs font-semibold text-slate-500 mb-1">Command to Execute</label>
              <textarea v-model="formJob.command" class="input-field font-mono text-sm min-h-[80px]" placeholder="/path/to/script.sh >> /var/log/script.log 2>&1"></textarea>
            </div>
            
          </div>
          <div class="p-4 border-t border-slate-100 bg-slate-50 shrink-0 flex justify-end gap-2">
            <button @click="showModal = false" class="btn-secondary">Cancel</button>
            <button @click="submitJob" class="btn-primary">Save Job</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
