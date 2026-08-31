import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import ServerLayout from '../views/ServerLayout.vue'
import DashboardView from '../views/DashboardView.vue'
import PortsView from '../views/PortsView.vue'
import ContainerView from '../views/ContainerView.vue'
import CloudflareView from '../views/CloudflareView.vue'
import FilesView from '../views/FilesView.vue'
import LogsView from '../views/LogsView.vue'
import UsersView from '../views/UsersView.vue'
import ServicesView from '../views/ServicesView.vue'
import SyslogsView from '../views/SyslogsView.vue'
import CronView from '../views/CronView.vue'
import SettingsView from '../views/SettingsView.vue'
import UpdatesView from '../views/UpdatesView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
    },
    {
      path: '/settings',
      name: 'global-settings',
      component: SettingsView,
    },
    {
      path: '/server/:id',
      component: ServerLayout,
      children: [
        { path: 'dashboard', name: 'dashboard', component: DashboardView },
        { path: 'updates', name: 'updates', component: UpdatesView },
        { path: 'services', name: 'services', component: ServicesView },
        { path: 'syslogs', name: 'syslogs', component: SyslogsView },
        { path: 'cron', name: 'cron', component: CronView },
        { path: 'ports', name: 'ports', component: PortsView },
        { path: 'cloudflare', name: 'cloudflare', component: CloudflareView },
        { path: 'containers', name: 'containers', component: ContainerView },
        // Legacy redirect
        { path: 'podman', redirect: to => ({ name: 'containers', params: to.params }) },
        { path: 'files', name: 'files', component: FilesView },
        { path: 'users', name: 'users', component: UsersView },
        { path: 'logs', name: 'logs', component: LogsView },
        { path: 'settings', name: 'settings', component: SettingsView },
      ]
    }
  ],
})

export default router
