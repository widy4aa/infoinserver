async function fetchNetworkInfo() {
    try {
        const response = await fetch('/api/network');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        document.getElementById('network-loading').style.display = 'none';
        document.getElementById('network-data').style.display = 'block';

        const tbody = document.getElementById('network-tbody');
        tbody.innerHTML = ''; // Clear existing rows

        data.forEach(iface => {
            const row = document.createElement('tr');
            row.style.borderBottom = "1px solid #eee";
            
            row.innerHTML = `
                <td style="padding: 0.5rem 0;">${iface.name}</td>
                <td style="padding: 0.5rem 0; font-family: monospace;">${iface.mac_address}</td>
                <td style="padding: 0.5rem 0;">${iface.ip_networks.join('<br>')}</td>
                <td style="padding: 0.5rem 0;">${(iface.rx_bytes / 1024 / 1024).toFixed(2)} MB</td>
                <td style="padding: 0.5rem 0;">${(iface.tx_bytes / 1024 / 1024).toFixed(2)} MB</td>
            `;
            tbody.appendChild(row);
        });

    } catch (error) {
        console.error("Could not fetch network info:", error);
        document.getElementById('network-loading').textContent = 'Failed to load data.';
    }
}

// Initial fetch
fetchNetworkInfo();

// Poll every 5 seconds
setInterval(fetchNetworkInfo, 5000);
