async function fetchPodmanInfo() {
    try {
        const response = await fetch('/api/podman/containers');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        document.getElementById('podman-loading').style.display = 'none';
        document.getElementById('podman-data').style.display = 'block';

        const tbody = document.getElementById('podman-tbody');
        tbody.innerHTML = ''; // Clear existing rows

        if (data.length === 0) {
            tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:1rem;">No containers found</td></tr>';
            return;
        }

        data.forEach(container => {
            const row = document.createElement('tr');
            row.style.borderBottom = "1px solid #eee";
            
            const name = container.Names && container.Names.length > 0 ? container.Names[0] : container.Id.substring(0, 12);
            
            row.innerHTML = `
                <td style="padding: 0.5rem 0; font-weight: bold;">${name}</td>
                <td style="padding: 0.5rem 0;">${container.Image}</td>
                <td style="padding: 0.5rem 0;">
                    <span style="padding: 2px 6px; border-radius: 4px; font-size: 0.8em; color: white; background-color: ${container.State === 'running' ? '#27ae60' : '#7f8c8d'}">
                        ${container.State}
                    </span>
                </td>
                <td style="padding: 0.5rem 0; font-size: 0.9em;">${container.Status}</td>
                <td style="padding: 0.5rem 0;">
                    <button onclick="podmanAction('start', '${container.Id}')" ${container.State === 'running' ? 'disabled' : ''}>Start</button>
                    <button onclick="podmanAction('stop', '${container.Id}')" ${container.State !== 'running' ? 'disabled' : ''}>Stop</button>
                    <button onclick="podmanAction('restart', '${container.Id}')">Restart</button>
                </td>
            `;
            tbody.appendChild(row);
        });

    } catch (error) {
        console.error("Could not fetch podman info:", error);
        document.getElementById('podman-loading').textContent = 'Failed to load container data (is podman installed?).';
    }
}

async function podmanAction(action, id) {
    if (!confirm(`Are you sure you want to ${action} container ${id.substring(0, 8)}...?`)) {
        return;
    }
    
    try {
        const response = await fetch(`/api/podman/containers/${action}/${id}`, {
            method: 'POST'
        });
        
        const result = await response.json();
        if (response.ok) {
            alert(result.message);
            fetchPodmanInfo(); // Refresh immediately
        } else {
            alert(`Error: ${result}`);
        }
    } catch (error) {
        alert("Failed to execute action");
        console.error(error);
    }
}

async function createNewContainer() {
    const name = document.getElementById('create-c-name').value.trim();
    const image = document.getElementById('create-c-image').value.trim();
    const portsStr = document.getElementById('create-c-ports').value.trim();
    const statusDiv = document.getElementById('create-c-status');

    if (!name || !image) {
        alert("Name and Image are required!");
        return;
    }

    const ports = portsStr ? portsStr.split(',').map(s => s.trim()) : [];
    
    statusDiv.textContent = 'Creating container...';
    statusDiv.style.color = '#333';

    try {
        const response = await fetch('/api/podman/create', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, image, ports })
        });
        
        if (!response.ok) {
            const err = await response.text();
            throw new Error(err);
        }
        
        const data = await response.json();
        statusDiv.textContent = data.message;
        statusDiv.style.color = 'green';
        
        document.getElementById('create-c-name').value = '';
        document.getElementById('create-c-image').value = '';
        document.getElementById('create-c-ports').value = '';
        
        fetchPodmanInfo();
    } catch (error) {
        statusDiv.textContent = `Error: ${error.message}`;
        statusDiv.style.color = 'red';
    }
}

// Initial fetch
fetchPodmanInfo();

// Poll every 5 seconds
setInterval(fetchPodmanInfo, 5000);
