import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import ServerLayout from '../views/ServerLayout.vue'
import DashboardView from '../views/DashboardView.vue'
import PortsView from '../views/PortsView.vue'
import PodmanView from '../views/PodmanView.vue'
import CloudflareView from '../views/CloudflareView.vue'
import FilesView from '../views/FilesView.vue'
import SpeedtestView from '../views/SpeedtestView.vue'
import LogsView from '../views/LogsView.vue'
import SettingsView from '../views/SettingsView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
    },
    {
      path: '/settings', // Global settings (adding servers)
      name: 'global-settings',
      component: SettingsView,
    },
    {
      path: '/server/:id',
      component: ServerLayout,
      children: [
        { path: 'dashboard', name: 'dashboard', component: DashboardView },
        { path: 'speedtest', name: 'speedtest', component: SpeedtestView },
        { path: 'ports', name: 'ports', component: PortsView },
        { path: 'cloudflare', name: 'cloudflare', component: CloudflareView },
        { path: 'podman', name: 'podman', component: PodmanView },
        { path: 'files', name: 'files', component: FilesView },
        { path: 'logs', name: 'logs', component: LogsView },
        { path: 'settings', name: 'settings', component: SettingsView },
      ]
    }
  ],
})

export default router
