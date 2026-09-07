<script setup>
// Static feature showcase — no props needed
</script>

<template>
  <div class="hidden md:flex flex-col justify-between h-full p-10
              border-l border-slate-200 dark:border-slate-700/60"
       style="background: #0d1117">

    <!-- Top: headline -->
    <div class="flex flex-col gap-1">
      <div class="text-[10px] font-semibold uppercase tracking-[0.12em] text-cyan-500/80 mb-2">
        What you get
      </div>
      <h2 class="text-lg font-bold text-white leading-snug tracking-tight">
        Everything your<br/>server runs, exposed.
      </h2>
    </div>

    <!-- Feature list — terminal-style rows -->
    <div class="flex flex-col gap-px mt-6 flex-1">

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">01</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">Live Metrics</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">CPU, RAM, disk — streamed via WebSocket</div>
        </div>
      </div>

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">02</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">systemd Control</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">Start, stop, restart — from your browser</div>
        </div>
      </div>

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">03</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">File Explorer</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">Browse, edit, chmod — three view modes</div>
        </div>
      </div>

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">04</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">Containers</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">Docker & Podman — live status, logs, compose</div>
        </div>
      </div>

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">05</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">Network & Security</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">UFW firewall, Fail2Ban, open ports, Nmap scan</div>
        </div>
      </div>

      <div class="flex items-start gap-3 px-3 py-2.5 rounded-md group hover:bg-white/4 transition-colors">
        <span class="text-cyan-500 font-mono text-[11px] shrink-0 mt-0.5 opacity-60">06</span>
        <div>
          <div class="text-[12px] font-semibold text-slate-200">Terminal</div>
          <div class="text-[11px] text-slate-500 leading-snug mt-0.5">xterm.js tabs — full shell, any time</div>
        </div>
      </div>

    </div>

    <!-- Bottom bar -->
    <div class="mt-6 pt-4 border-t border-white/5 flex items-center justify-end">
      <span class="text-[10px] text-slate-700 font-mono">infoinserver</span>
    </div>

  </div>
</template>
