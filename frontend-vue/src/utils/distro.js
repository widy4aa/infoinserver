// frontend-vue/src/utils/distro.js
// Memetakan nama OS distro Linux ke URL ikon dari SimpleIcons CDN

/**
 * Mengembalikan URL gambar ikon distro Linux berdasarkan string os_name.
 * Jika tidak dikenali atau os_name null/kosong, mengembalikan null
 * (agar komponen bisa fallback ke ikon <Server />).
 * @param {string|null|undefined} osName
 * @returns {string|null}
 */
export function getDistroIcon(osName) {
  if (!osName) return null
  const name = osName.toLowerCase()

  if (name.includes('ubuntu')) return 'https://cdn.simpleicons.org/ubuntu/E95420'
  if (name.includes('debian')) return 'https://cdn.simpleicons.org/debian/A81D33'
  if (name.includes('cachyos') || name.includes('cachy')) return 'https://cdn.simpleicons.org/archlinux/1793D1'
  if (name.includes('manjaro')) return 'https://cdn.simpleicons.org/manjaro/35BF5C'
  if (name.includes('arch')) return 'https://cdn.simpleicons.org/archlinux/1793D1'
  if (name.includes('fedora')) return 'https://cdn.simpleicons.org/fedora/51A2DA'
  if (name.includes('centos')) return 'https://cdn.simpleicons.org/centos/262577'
  if (name.includes('red hat') || name.includes('rhel')) return 'https://cdn.simpleicons.org/redhat/EE0000'
  if (name.includes('opensuse') || name.includes('suse')) return 'https://cdn.simpleicons.org/opensuse/73BA25'
  if (name.includes('alpine')) return 'https://cdn.simpleicons.org/alpinelinux/0D597F'
  if (name.includes('mint')) return 'https://cdn.simpleicons.org/linuxmint/87CF3E'
  if (name.includes('kali')) return 'https://cdn.simpleicons.org/kalilinux/557C94'
  if (name.includes('raspberry')) return 'https://cdn.simpleicons.org/raspberrypi/A22846'
  if (name.includes('nixos') || name.includes('nix')) return 'https://cdn.simpleicons.org/nixos/5277C3'
  if (name.includes('rocky')) return 'https://cdn.simpleicons.org/rockylinux/10B981'
  if (name.includes('alma')) return 'https://cdn.simpleicons.org/almalinux/FF4C4C'
  if (name.includes('linux')) return 'https://cdn.simpleicons.org/linux/FCC624' // Generic Linux fallback
  return null // Tidak dikenali sama sekali
}

/**
 * Mengembalikan kelas CSS warna latar belakang (bg-*) yang sesuai distro
 * untuk digunakan sebagai background ikon di HomeView.
 * @param {string|null|undefined} osName
 * @returns {string}
 */
export function getDistroColorClass(osName) {
  if (!osName) return ''
  const name = osName.toLowerCase()

  if (name.includes('ubuntu')) return 'bg-orange-50 dark:bg-orange-900/20'
  if (name.includes('debian')) return 'bg-red-50 dark:bg-red-900/20'
  if (name.includes('arch') || name.includes('cachyos') || name.includes('manjaro')) return 'bg-blue-50 dark:bg-blue-900/20'
  if (name.includes('fedora')) return 'bg-blue-50 dark:bg-blue-900/20'
  if (name.includes('opensuse') || name.includes('suse')) return 'bg-green-50 dark:bg-green-900/20'
  if (name.includes('alpine')) return 'bg-blue-50 dark:bg-blue-900/20'
  if (name.includes('mint')) return 'bg-green-50 dark:bg-green-900/20'
  if (name.includes('kali')) return 'bg-slate-50 dark:bg-slate-700/50'
  return 'bg-slate-50 dark:bg-slate-700/50'
}