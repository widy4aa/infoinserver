async function fetchSpeedtestHistory() {
    try {
        const response = await fetch('/api/speedtest/history');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        document.getElementById('speedtest-loading').style.display = 'none';
        document.getElementById('speedtest-data').style.display = 'block';

        const tbody = document.getElementById('speedtest-tbody');
        tbody.innerHTML = ''; 

        if (data.length === 0) {
            tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:1rem;">No history found</td></tr>';
            return;
        }

        data.forEach(test => {
            const row = document.createElement('tr');
            row.style.borderBottom = "1px solid #eee";
            
            const dateStr = new Date(test.tested_at).toLocaleString();
            
            row.innerHTML = `
                <td style="padding: 0.5rem 0;">${dateStr}</td>
                <td style="padding: 0.5rem 0; font-weight: bold; color: #2980b9;">${test.download_mbps.toFixed(2)}</td>
                <td style="padding: 0.5rem 0; font-weight: bold; color: #27ae60;">${test.upload_mbps.toFixed(2)}</td>
                <td style="padding: 0.5rem 0;">${test.ping_ms.toFixed(1)}</td>
                <td style="padding: 0.5rem 0;">${test.server_name || '-'}</td>
            `;
            tbody.appendChild(row);
        });

    } catch (error) {
        console.error("Could not fetch speedtest history:", error);
        document.getElementById('speedtest-loading').textContent = 'Failed to load history.';
    }
}

async function runSpeedtest() {
    const btn = document.getElementById('btn-run-speedtest');
    const status = document.getElementById('speedtest-status');
    
    btn.disabled = true;
    status.textContent = 'Running speedtest (this may take up to a minute)...';
    status.style.color = '#f39c12';
    
    try {
        const response = await fetch('/api/speedtest/run', {
            method: 'POST'
        });
        
        const result = await response.json();
        if (response.ok) {
            status.textContent = 'Speedtest complete!';
            status.style.color = '#27ae60';
            fetchSpeedtestHistory(); // Refresh table
        } else {
            status.textContent = `Error: ${result}`;
            status.style.color = '#c0392b';
        }
    } catch (error) {
        status.textContent = 'Failed to execute speedtest.';
        status.style.color = '#c0392b';
        console.error(error);
    } finally {
        btn.disabled = false;
        setTimeout(() => {
            if(status.textContent === 'Speedtest complete!') status.textContent = '';
        }, 5000);
    }
}

// Initial fetch on load
fetchSpeedtestHistory();

// Poll every 1 minute just to keep it somewhat updated if scheduler runs
setInterval(fetchSpeedtestHistory, 60000);
