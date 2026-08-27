async function fetchSystemInfo() {
    try {
        const response = await fetch('/api/system');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        document.getElementById('system-loading').style.display = 'none';
        document.getElementById('system-data').style.display = 'block';

        document.getElementById('sys-hostname').textContent = data.hostname || 'Unknown';
        document.getElementById('sys-os').textContent = data.os_name || 'Unknown';
        document.getElementById('sys-kernel').textContent = data.kernel_version || 'Unknown';
        
        // Format uptime
        const uptimeSeconds = data.uptime;
        const days = Math.floor(uptimeSeconds / (3600 * 24));
        const hours = Math.floor(uptimeSeconds % (3600 * 24) / 3600);
        const mins = Math.floor(uptimeSeconds % 3600 / 60);
        document.getElementById('sys-uptime').textContent = `${days}d ${hours}h ${mins}m`;

        document.getElementById('sys-cpu-cores').textContent = data.cpu_cores;
        document.getElementById('sys-cpu-usage').textContent = data.global_cpu_usage.toFixed(2);
        
        const ramUsed = (data.used_memory / (1024 * 1024 * 1024)).toFixed(2);
        const ramTotal = (data.total_memory / (1024 * 1024 * 1024)).toFixed(2);
        document.getElementById('sys-ram').textContent = `${ramUsed} GB / ${ramTotal} GB`;

    } catch (error) {
        console.error("Could not fetch system info:", error);
        document.getElementById('system-loading').textContent = 'Failed to load data.';
    }
}

// Initial fetch
fetchSystemInfo();

// Poll every 3 seconds
setInterval(fetchSystemInfo, 3000);
