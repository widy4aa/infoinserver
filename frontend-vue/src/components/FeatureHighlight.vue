<script setup>
defineProps({
  title: { type: String, default: 'Your Linux server, fully visible.' },
  description: { type: String, default: 'Monitor CPU, RAM, disk, containers, firewall, and more — all from one elegant dashboard.' },
})
</script>

<template>
  <div class="hidden md:flex flex-col items-center justify-center gap-8 p-10 text-center
              bg-slate-50 dark:bg-slate-900
              border-l border-slate-200 dark:border-slate-700">

    <!-- 2 foto kiri kanan, 1:1, tanpa rounded, mepet -->
    <div class="flex w-full">
      <img
        src="/images.jpeg"
        alt="Dashboard preview"
        class="w-1/2 aspect-square object-cover shadow-lg"
      />
      <img
        src="/icon.jpg"
        alt="Server icon"
        class="w-1/2 aspect-square object-cover shadow-lg"
      />
    </div>

    <!-- Text -->
    <div class="flex flex-col gap-3 max-w-xs">
      <h2 class="text-xl font-bold text-slate-800 dark:text-slate-100 leading-snug">
        {{ title }}
      </h2>
    </div>

  </div>
</template>
