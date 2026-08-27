let currentPath = '/';

async function fetchFiles(path) {
    document.getElementById('files-loading').style.display = 'block';
    document.getElementById('files-data').style.display = 'none';
    
    try {
        const response = await fetch(`/api/files/list?path=${encodeURIComponent(path)}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        currentPath = path;
        document.getElementById('files-current-path').textContent = currentPath;
        
        document.getElementById('files-loading').style.display = 'none';
        document.getElementById('files-data').style.display = 'block';

        const tbody = document.getElementById('files-tbody');
        tbody.innerHTML = ''; // Clear existing rows

        if (data.length === 0) {
            tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; padding:1rem;">Empty directory</td></tr>';
            return;
        }

        data.forEach(file => {
            const row = document.createElement('tr');
            row.style.borderBottom = "1px solid #eee";
            
            const icon = file.is_dir ? '📁' : '📄';
            const sizeStr = file.is_dir ? '-' : formatBytes(file.size);
            const dateStr = new Date(file.modified * 1000).toLocaleString();
            
            let nameHtml = file.name;
            let actionHtml = '';

            if (file.is_dir) {
                const nextPath = currentPath === '/' ? `/${file.name}` : `${currentPath}/${file.name}`;
                nameHtml = `<a href="#" onclick="fetchFiles('${nextPath}'); return false;">${file.name}</a>`;
            } else {
                const filePath = currentPath === '/' ? `/${file.name}` : `${currentPath}/${file.name}`;
                actionHtml = `<a href="/api/files/download?path=${encodeURIComponent(filePath)}" target="_blank" download>Download</a>`;
            }

            row.innerHTML = `
                <td style="padding: 0.5rem 0; font-size: 1.2em;">${icon}</td>
                <td style="padding: 0.5rem 0;">${nameHtml}</td>
                <td style="padding: 0.5rem 0;">${sizeStr}</td>
                <td style="padding: 0.5rem 0;">${dateStr}</td>
                <td style="padding: 0.5rem 0;">${actionHtml}</td>
            `;
            tbody.appendChild(row);
        });

    } catch (error) {
        console.error("Could not fetch files:", error);
        document.getElementById('files-loading').textContent = 'Failed to load directory. Access denied or path invalid.';
        
        // Kembalikan ke root jika gagal (misal kena path traversal block)
        if (path !== '/') {
            setTimeout(() => fetchFiles('/'), 2000);
        }
    }
}

function navigateUp() {
    if (currentPath === '/') return;
    
    const parts = currentPath.split('/').filter(p => p !== '');
    parts.pop(); // buang folder terakhir
    
    const newPath = '/' + parts.join('/');
    fetchFiles(newPath);
}

function formatBytes(bytes, decimals = 2) {
    if (!+bytes) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

// Initial fetch on load
fetchFiles('/');
