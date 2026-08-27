async function fetchPortsInfo() {
    try {
        const response = await fetch('/api/ports');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        document.getElementById('ports-loading').style.display = 'none';
        document.getElementById('ports-data').style.display = 'block';

        const tbody = document.getElementById('ports-tbody');
        tbody.innerHTML = ''; // Clear existing rows

        data.forEach(port => {
            const row = document.createElement('tr');
            row.style.borderBottom = "1px solid #eee";
            
            row.innerHTML = `
                <td style="padding: 0.5rem 0;">${port.protocol.toUpperCase()}</td>
                <td style="padding: 0.5rem 0; font-family: monospace;">${port.local_address}</td>
                <td style="padding: 0.5rem 0;">${port.state}</td>
                <td style="padding: 0.5rem 0;">${port.process}</td>
            `;
            tbody.appendChild(row);
        });

    } catch (error) {
        console.error("Could not fetch ports info:", error);
        document.getElementById('ports-loading').textContent = 'Failed to load data.';
    }
}

// Deep Scan Logic
let currentScanJobId = null;
let scanPollingInterval = null;

async function startDeepScan() {
    const target = document.getElementById('scan-target').value;
    const statusDiv = document.getElementById('scan-status');
    const resultPre = document.getElementById('scan-result');
    
    statusDiv.textContent = 'Initiating scan...';
    statusDiv.style.color = '#333';
    resultPre.style.display = 'none';
    
    if (scanPollingInterval) clearInterval(scanPollingInterval);

    try {
        const response = await fetch('/api/ports/scan', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ target: target })
        });
        
        if (!response.ok) {
            const err = await response.text();
            throw new Error(err);
        }
        
        const data = await response.json();
        currentScanJobId = data.job_id;
        statusDiv.textContent = `Job #${currentScanJobId} started. Scanning...`;
        
        // Start polling status
        scanPollingInterval = setInterval(pollScanStatus, 2000);
        
    } catch (error) {
        statusDiv.textContent = `Error: ${error.message}`;
        statusDiv.style.color = 'red';
    }
}

async function pollScanStatus() {
    if (!currentScanJobId) return;
    
    const statusDiv = document.getElementById('scan-status');
    const resultPre = document.getElementById('scan-result');

    try {
        const response = await fetch(`/api/ports/scan/${currentScanJobId}`);
        const job = await response.json();
        
        if (job.status === 'done') {
            clearInterval(scanPollingInterval);
            statusDiv.textContent = `Scan complete (Finished at ${new Date(job.finished_at).toLocaleTimeString()})`;
            statusDiv.style.color = 'green';
            
            resultPre.style.display = 'block';
            try {
                const parsedResult = JSON.parse(job.result_json);
                resultPre.textContent = parsedResult.raw_output || job.result_json;
            } catch (e) {
                resultPre.textContent = job.result_json;
            }
        } else if (job.status === 'failed') {
            clearInterval(scanPollingInterval);
            statusDiv.textContent = 'Scan failed!';
            statusDiv.style.color = 'red';
            resultPre.style.display = 'block';
            resultPre.textContent = job.result_json;
        } else {
            statusDiv.textContent = `Job #${currentScanJobId} status: ${job.status}...`;
        }
    } catch (error) {
        console.error("Polling error:", error);
    }
}

// Initial fetch
fetchPortsInfo();

// Poll every 10 seconds
setInterval(fetchPortsInfo, 10000);
