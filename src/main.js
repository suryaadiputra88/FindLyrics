// Detect Tauri environment
const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;

let invoke = async (cmd, args) => {
  console.warn(`[Web Mode Mock] invoke("${cmd}")`, args);
  return [];
};

let openDialog = async () => {
  alert("Folder picker requires running inside the Tauri Desktop app (npm run tauri dev).");
  return null;
};

let listenEvent = async () => () => {};

// Initialize Tauri plugins if running inside Tauri
async function initTauri() {
  if (isTauri) {
    try {
      const core = await import('@tauri-apps/api/core');
      invoke = core.invoke;

      const dialog = await import('@tauri-apps/plugin-dialog');
      openDialog = async (options) => {
        return await dialog.open(options);
      };

      const event = await import('@tauri-apps/api/event');
      listenEvent = event.listen;
    } catch (e) {
      console.error("Failed to load Tauri modules:", e);
    }
  }
}

// App State
let currentFolder = null;
let allTracks = [];
let activeFilter = 'all';
let searchQuery = '';
let isProcessing = false;
let isCancelled = false;
let systemConcurrency = 8;
let fetchProgressTimeout = null;
let scanProgressTimeout = null;

// DOM Elements
const btnSelectFolder = document.getElementById('btnSelectFolder');
const folderDisplay = document.getElementById('folderDisplay');
const btnFetchLyrics = document.getElementById('btnFetchLyrics');
const chkRecursive = document.getElementById('chkRecursive');
const chkOverwrite = document.getElementById('chkOverwrite');
const chkUpgradePlain = document.getElementById('chkUpgradePlain');
const chkSyncedOnly = document.getElementById('chkSyncedOnly');
const chkAutoFollow = document.getElementById('chkAutoFollow');
const numTolerance = document.getElementById('numTolerance');

const statBadges = document.getElementById('statBadges');
const badgeTotal = document.getElementById('badgeTotal');
const badgeMissing = document.getElementById('badgeMissing');
const badgeSynced = document.getElementById('badgeSynced');

// Scanning Progress Elements
const scanProgressSection = document.getElementById('scanProgressSection');
const scanCountBadge = document.getElementById('scanCountBadge');
const scanCurrentFile = document.getElementById('scanCurrentFile');

// Fetching Progress Elements
const fetchProgressSection = document.getElementById('fetchProgressSection');
const fetchProgressTitle = document.getElementById('fetchProgressTitle');
const fetchProgressPercent = document.getElementById('fetchProgressPercent');
const fetchWorkerBadge = document.getElementById('fetchWorkerBadge');
const fetchProgressBar = document.getElementById('fetchProgressBar');
const fetchCurrentSong = document.getElementById('fetchCurrentSong');
const fetchCurrentStatusMsg = document.getElementById('fetchCurrentStatusMsg');
const fetchCounterText = document.getElementById('fetchCounterText');

const statFetchSuccess = document.getElementById('statFetchSuccess');
const statFetchSkipped = document.getElementById('statFetchSkipped');
const statFetchNotFound = document.getElementById('statFetchNotFound');
const statFetchError = document.getElementById('statFetchError');

// Table & Content Elements
const emptyState = document.getElementById('emptyState');
const tableContainer = document.getElementById('tableContainer');
const trackTableBody = document.getElementById('trackTableBody');
const searchInput = document.getElementById('searchInput');

const cntAll = document.getElementById('cntAll');
const cntMissing = document.getElementById('cntMissing');
const cntSynced = document.getElementById('cntSynced');
const cntPlain = document.getElementById('cntPlain');

// Modal Elements
const lyricsModal = document.getElementById('lyricsModal');
const modalSongTitle = document.getElementById('modalSongTitle');
const modalSongArtist = document.getElementById('modalSongArtist');
const modalLyricsContent = document.getElementById('modalLyricsContent');
const modalClose = document.getElementById('modalClose');
const modalCloseBtn = document.getElementById('modalCloseBtn');

// Initialize
async function setup() {
  await initTauri();

  // Detect machine hardware concurrency from Rust
  try {
    const detected = await invoke('get_system_concurrency');
    if (detected && detected > 0) {
      systemConcurrency = detected;
    }
  } catch (err) {
    if (navigator.hardwareConcurrency) {
      systemConcurrency = navigator.hardwareConcurrency;
    }
  }

  // Listen to scanning progress events from Rust
  try {
    await listenEvent('scan-progress', (event) => {
      const payload = event.payload;
      if (payload) {
        scanCountBadge.textContent = `${payload.found_count} file ditemukan`;
        scanCurrentFile.textContent = payload.current_file || "Membaca file...";
      }
    });
  } catch (err) {
    console.warn("Could not attach scan-progress listener:", err);
  }

  btnSelectFolder.addEventListener('click', handleSelectFolder);
  btnFetchLyrics.addEventListener('click', () => {
    if (isProcessing) {
      isCancelled = true;
      setFetchButtonState('stopping');
    } else {
      handleFetchAllLyrics();
    }
  });

  searchInput.addEventListener('input', (e) => {
    searchQuery = e.target.value.toLowerCase().trim();
    renderTable();
  });

  // Filter tabs
  document.querySelectorAll('#filterTabs button').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#filterTabs button').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activeFilter = btn.dataset.filter;
      renderTable();
    });
  });

  // Modal close handlers
  modalClose.addEventListener('click', closeModal);
  modalCloseBtn.addEventListener('click', closeModal);
  lyricsModal.addEventListener('click', (e) => {
    if (e.target === lyricsModal) closeModal();
  });
}

// Select folder and scan
async function handleSelectFolder() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Pilih Folder Koleksi Musik",
    });

    if (!selected) return;

    currentFolder = selected;
    folderDisplay.textContent = currentFolder;
    folderDisplay.classList.remove('text-slate-400');
    folderDisplay.classList.add('text-indigo-300');

    await scanFolder();
  } catch (err) {
    console.error("Gagal memilih folder:", err);
    alert("Gagal memilih folder: " + err);
  }
}

// Toggle unified Fetch / Stop button state
function setFetchButtonState(state) {
  if (state === 'stop') {
    btnFetchLyrics.disabled = false;
    btnFetchLyrics.classList.add('btn-stop');
    btnFetchLyrics.innerHTML = `
      <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
        <rect x="6" y="6" width="12" height="12" rx="2" />
      </svg>
      <span>Stop</span>
    `;
  } else if (state === 'stopping') {
    btnFetchLyrics.disabled = true;
    btnFetchLyrics.classList.add('btn-stop');
    btnFetchLyrics.innerHTML = `
      <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      <span>Menghentikan...</span>
    `;
  } else {
    // idle state
    btnFetchLyrics.classList.remove('btn-stop');
    btnFetchLyrics.disabled = allTracks.length === 0;
    btnFetchLyrics.innerHTML = `
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10" />
      </svg>
      <span>Fetch Lyrics</span>
    `;
  }
}

// Scan directory with live scanning progress
async function scanFolder() {
  if (!currentFolder) return;

  if (scanProgressTimeout) {
    clearTimeout(scanProgressTimeout);
    scanProgressTimeout = null;
  }
  if (fetchProgressTimeout) {
    clearTimeout(fetchProgressTimeout);
    fetchProgressTimeout = null;
  }
  fetchProgressSection.classList.add('hidden');
  fetchProgressBar.style.width = '0%';

  btnSelectFolder.disabled = true;
  btnFetchLyrics.disabled = true;
  scanProgressSection.classList.remove('hidden');
  scanCountBadge.textContent = "0 file ditemukan";
  scanCurrentFile.textContent = "Mempersiapkan pemindaian...";

  try {
    const tracks = await invoke('scan_music_directory', {
      path: currentFolder,
      recursive: chkRecursive.checked,
    });

    allTracks = tracks;
    updateCounts();
    renderTable();
  } catch (err) {
    console.error("Scan error:", err);
    alert("Gagal memindai direktori: " + err);
  } finally {
    scanProgressTimeout = setTimeout(() => {
      scanProgressSection.classList.add('hidden');
      scanProgressTimeout = null;
    }, 400);

    btnSelectFolder.disabled = false;
    setFetchButtonState('idle');
  }
}

// Update counts & badges
function updateCounts() {
  const total = allTracks.length;
  const missing = allTracks.filter(t => t.lyric_state === 'none').length;
  const synced = allTracks.filter(t => t.lyric_state === 'synced').length;
  const plain = allTracks.filter(t => t.lyric_state === 'plain').length;

  cntAll.textContent = total;
  cntMissing.textContent = missing;
  cntSynced.textContent = synced;
  cntPlain.textContent = plain;

  if (total > 0) {
    statBadges.classList.remove('hidden');
    badgeTotal.textContent = `${total} Total`;
    badgeMissing.textContent = `${missing} Missing`;
    badgeSynced.textContent = `${synced} Synced`;
  } else {
    statBadges.classList.add('hidden');
  }

  if (!isProcessing) {
    btnFetchLyrics.disabled = total === 0;
  }
}

// Render tracks table
function renderTable() {
  if (allTracks.length === 0) {
    emptyState.classList.remove('hidden');
    tableContainer.classList.add('hidden');
    return;
  }

  emptyState.classList.add('hidden');
  tableContainer.classList.remove('hidden');

  let filtered = allTracks.filter(track => {
    // Filter by tab
    if (activeFilter === 'missing' && track.lyric_state !== 'none') return false;
    if (activeFilter === 'synced' && track.lyric_state !== 'synced') return false;
    if (activeFilter === 'plain' && track.lyric_state !== 'plain') return false;

    // Filter by search query
    if (searchQuery) {
      const matchTitle = track.title.toLowerCase().includes(searchQuery);
      const matchArtist = track.artist.toLowerCase().includes(searchQuery);
      const matchFilename = track.filename.toLowerCase().includes(searchQuery);
      return matchTitle || matchArtist || matchFilename;
    }

    return true;
  });

  trackTableBody.innerHTML = '';

  if (filtered.length === 0) {
    const emptyRow = document.createElement('tr');
    emptyRow.innerHTML = `<td colspan="8" class="text-center py-8 text-slate-500">Tidak ada lagu yang cocok dengan filter saat ini.</td>`;
    trackTableBody.appendChild(emptyRow);
    return;
  }

  filtered.forEach((track, idx) => {
    const tr = document.createElement('tr');
    tr.id = `row-track-${idx}`;
    tr.dataset.path = track.path;

    // Format badge class
    const formatClass = `badge-${track.format.toLowerCase()}`;

    // Lyric badge
    const lyricBadge = renderLyricBadge(track.lyric_state, track.provider);

    tr.innerHTML = `
      <td class="py-3 px-4 text-center text-slate-500 font-mono text-xs">${idx + 1}</td>
      <td class="py-3 px-3 text-center">
        <span class="badge ${formatClass}">${track.format}</span>
      </td>
      <td class="py-3 px-4">
        <div class="font-semibold text-slate-100">${escapeHtml(track.title)}</div>
        <div class="text-[11px] font-mono text-slate-500 truncate max-w-xs">${escapeHtml(track.filename)}</div>
      </td>
      <td class="py-3 px-4 text-slate-300">${escapeHtml(track.artist)}</td>
      <td class="py-3 px-4 text-slate-400 text-xs">${escapeHtml(track.album)}</td>
      <td class="py-3 px-3 text-center font-mono text-xs text-slate-400">${track.duration_formatted}</td>
      <td class="py-3 px-4 text-center" id="status-cell-${idx}">
        ${lyricBadge}
      </td>
      <td class="py-3 px-4 text-center">
        <div class="flex items-center justify-center gap-1.5" id="action-cell-${idx}">
          ${track.lyric_state !== 'none' ? `
            <button class="btn-preview px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white transition-colors" data-path="${escapeHtml(track.path)}" data-title="${escapeHtml(track.title)}" data-artist="${escapeHtml(track.artist)}">
              Lihat
            </button>
          ` : `
            <button class="btn-single-fetch px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white transition-colors" data-idx="${idx}">
              Fetch
            </button>
          `}
        </div>
      </td>
    `;

    trackTableBody.appendChild(tr);
  });

  attachTableListeners(filtered);
}

function renderLyricBadge(state, provider) {
  const provTag = provider ? ` <span class="opacity-70 font-mono text-[9px]">• ${escapeHtml(provider)}</span>` : '';
  if (state === 'downloading') {
    return `<span class="badge badge-downloading">⬇️ Mengunduh...</span>`;
  } else if (state === 'embedding') {
    return `<span class="badge badge-embedding">💾 Menanamkan...</span>`;
  } else if (state === 'synced') {
    return `<span class="badge badge-synced">✓ Synced${provTag}</span>`;
  } else if (state === 'plain') {
    return `<span class="badge badge-plain">Plain Text${provTag}</span>`;
  } else if (state === 'skipped') {
    return `<span class="badge badge-skipped">⏭ Dilewati</span>`;
  } else if (state === 'notfound') {
    return `<span class="badge badge-notfound">❓ Tidak Ditemukan</span>`;
  } else if (state === 'error') {
    return `<span class="badge badge-error">❌ Gagal</span>`;
  } else {
    return `<span class="badge badge-none">Belum Ada</span>`;
  }
}

function attachTableListeners(filteredList) {
  document.querySelectorAll('.btn-preview').forEach(btn => {
    btn.addEventListener('click', () => {
      openPreviewModal(btn.dataset.path, btn.dataset.title, btn.dataset.artist);
    });
  });

  document.querySelectorAll('.btn-single-fetch').forEach(btn => {
    btn.addEventListener('click', () => {
      const idx = parseInt(btn.dataset.idx, 10);
      processSingleTrack(filteredList[idx], idx);
    });
  });
}

let scrollThrottleTimer = null;
let lastScrollTime = 0;

// Auto-follow cursor to keep the currently processed row visible in table viewport
function autoFollowRow(rowEl) {
  if (!rowEl || !chkAutoFollow || !chkAutoFollow.checked) return;

  const scrollContainer = document.getElementById('tableMainContainer');
  if (scrollContainer) {
    const containerRect = scrollContainer.getBoundingClientRect();
    const rowRect = rowEl.getBoundingClientRect();

    // Check if row is already comfortably visible in scrollContainer
    // 45px accounts for the sticky header
    const isVisible = (rowRect.top >= containerRect.top + 45) && (rowRect.bottom <= containerRect.bottom - 10);
    if (isVisible) {
      return;
    }
  }

  const now = Date.now();
  if (now - lastScrollTime >= 250) {
    lastScrollTime = now;
    rowEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  } else {
    clearTimeout(scrollThrottleTimer);
    scrollThrottleTimer = setTimeout(() => {
      if (chkAutoFollow && chkAutoFollow.checked && rowEl && rowEl.isConnected) {
        lastScrollTime = Date.now();
        rowEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }, 150);
  }
}

// Fetch and embed all lyrics concurrently with multi-threading and rich progress
async function handleFetchAllLyrics() {
  if (isProcessing || allTracks.length === 0) return;

  if (fetchProgressTimeout) {
    clearTimeout(fetchProgressTimeout);
    fetchProgressTimeout = null;
  }

  isProcessing = true;
  isCancelled = false;
  setFetchButtonState('stop');
  btnSelectFolder.disabled = true;

  fetchProgressSection.classList.remove('hidden');
  fetchProgressTitle.textContent = "Proses Fetch & Embed Lirik Sedang Berjalan";

  let successCount = 0;
  let skippedCount = 0;
  let notFoundCount = 0;
  let errorCount = 0;
  let completedCount = 0;

  updateFetchStats(successCount, skippedCount, notFoundCount, errorCount);

  const tolerance = parseFloat(numTolerance.value) || 3.0;
  const overwrite = chkOverwrite.checked;
  const upgradePlain = chkUpgradePlain ? chkUpgradePlain.checked : true;
  const syncedOnly = chkSyncedOnly.checked;
  const concurrency = Math.max(1, systemConcurrency);

  if (fetchWorkerBadge) {
    fetchWorkerBadge.textContent = `${concurrency} threads paralel`;
  }

  const total = allTracks.length;
  let nextTrackIdx = 0;
  const activeDownloading = new Set();
  const activeEmbedding = new Set();

  function updateStatusBanner(lastMsg) {
    if (isProcessing) {
      fetchProgressSection.classList.remove('hidden');
    }
    const pct = total > 0 ? Math.round((completedCount / total) * 100) : 0;
    fetchProgressBar.style.width = `${pct}%`;
    fetchProgressPercent.textContent = `${pct}%`;
    fetchCounterText.textContent = `${completedCount} / ${total} file`;

    const dlCount = activeDownloading.size;
    const embCount = activeEmbedding.size;
    const allActive = [...Array.from(activeDownloading), ...Array.from(activeEmbedding)];

    if (allActive.length > 0) {
      const displayNames = allActive.slice(0, 2).join(', ');
      const extra = allActive.length > 2 ? ` (+${allActive.length - 2} lainnya)` : '';
      fetchCurrentSong.textContent = `${displayNames}${extra}`;

      const statusParts = [];
      if (dlCount > 0) statusParts.push(`⬇️ Mengunduh: ${dlCount}`);
      if (embCount > 0) statusParts.push(`💾 Menanamkan: ${embCount}`);
      fetchCurrentStatusMsg.textContent = `${statusParts.join(' | ')} (${concurrency} threads)`;
    } else if (lastMsg) {
      fetchCurrentStatusMsg.textContent = lastMsg;
    } else {
      fetchCurrentSong.textContent = "-";
      fetchCurrentStatusMsg.textContent = "Menyiapkan worker paralel...";
    }
  }

  // Initial banner update
  updateStatusBanner();

  // Worker loop function running concurrently across Tokio runtime
  async function worker() {
    while (!isCancelled) {
      if (nextTrackIdx >= total) {
        break;
      }
      const idx = nextTrackIdx++;
      const track = allTracks[idx];
      const titleDisplay = `${track.title} (${track.artist})`;

      const rowEl = document.querySelector(`tr[data-path="${CSS.escape(track.path)}"]`);
      const statusCell = rowEl ? rowEl.querySelector('td:nth-child(7)') : null;

      // STEP 1: Mengunduh
      activeDownloading.add(titleDisplay);
      if (rowEl) {
        rowEl.classList.add('row-processing');
        autoFollowRow(rowEl);
      }
      if (statusCell) {
        statusCell.innerHTML = `<span class="badge badge-downloading">⬇️ Mengunduh...</span>`;
      }
      updateStatusBanner();

      let lastMsg = "";
      try {
        const fetchRes = await invoke('fetch_track_lyrics', {
          path: track.path,
          toleranceSecs: tolerance,
          syncedOnly: syncedOnly,
          overwrite: overwrite,
          upgradePlain: upgradePlain,
        });

        activeDownloading.delete(titleDisplay);

        if (fetchRes.action === 'embed') {
          // STEP 2: Menanamkan / Menyisipkan
          activeEmbedding.add(titleDisplay);
          if (rowEl) {
            rowEl.classList.add('row-processing');
            autoFollowRow(rowEl);
          }
          if (statusCell) {
            statusCell.innerHTML = `<span class="badge badge-embedding">💾 Menanamkan...</span>`;
          }
          updateStatusBanner();

          // Embed directly into audio tags
          const embedRes = await invoke('embed_track_lyrics', {
            path: track.path,
            lyrics: fetchRes.lyrics,
            provider: fetchRes.provider,
            isSynced: fetchRes.is_synced,
            isUpgrade: fetchRes.is_upgrade,
          });

          // Small visual pause so both steps are clearly distinct to user
          await new Promise(r => setTimeout(r, 220));

          activeEmbedding.delete(titleDisplay);
          successCount++;
          track.lyric_state = embedRes.is_synced ? 'synced' : 'plain';
          track.provider = embedRes.provider;
          lastMsg = `✓ ${embedRes.message}`;
        } else if (fetchRes.action === 'skip') {
          skippedCount++;
          lastMsg = `⏭ ${fetchRes.message}`;
        } else {
          notFoundCount++;
          track.lyric_state = 'notfound';
          lastMsg = `❓ ${fetchRes.message}`;
        }
      } catch (err) {
        activeDownloading.delete(titleDisplay);
        activeEmbedding.delete(titleDisplay);
        errorCount++;
        track.lyric_state = 'error';
        console.error(`Error processing ${track.filename}:`, err);
        lastMsg = `❌ Error: ${err}`;
      } finally {
        activeDownloading.delete(titleDisplay);
        activeEmbedding.delete(titleDisplay);
        completedCount++;

        if (rowEl) {
          rowEl.classList.remove('row-processing');
          if (statusCell) {
            statusCell.innerHTML = renderLyricBadge(track.lyric_state, track.provider);
          }
        }

        updateFetchStats(successCount, skippedCount, notFoundCount, errorCount);
        updateCounts();
        updateStatusBanner(lastMsg);
      }
    }
  }

  // Spawn parallel workers up to concurrency limit
  const workerCount = Math.min(concurrency, total);
  const workers = [];
  for (let w = 0; w < workerCount; w++) {
    workers.push(worker());
  }

  await Promise.all(workers);

  // Done
  if (isCancelled) {
    fetchProgressTitle.textContent = "⚠️ Proses Dihentikan oleh Pengguna";
    fetchCurrentStatusMsg.textContent = `Dihentikan pada ${completedCount} / ${total} file`;
    fetchCurrentSong.textContent = "-";
  } else {
    fetchProgressBar.style.width = '100%';
    fetchProgressPercent.textContent = '100%';
    fetchCounterText.textContent = `${total} / ${total} file`;
    fetchProgressTitle.textContent = `🎉 Selesai! ${successCount} lirik berhasil di-embed (${concurrency} threads paralel).`;
    fetchCurrentStatusMsg.textContent = "Semua file audio siap diputar.";
    fetchCurrentSong.textContent = "-";

    fetchProgressTimeout = setTimeout(() => {
      fetchProgressSection.classList.add('hidden');
      fetchProgressBar.style.width = '0%';
      fetchProgressTimeout = null;
    }, 5000);
  }

  isProcessing = false;
  btnSelectFolder.disabled = false;
  setFetchButtonState('idle');
  renderTable();
}

function updateFetchStats(success, skipped, notFound, error) {
  statFetchSuccess.textContent = `✓ ${success} Berhasil`;
  statFetchSkipped.textContent = `⏭ ${skipped} Dilewati`;
  statFetchNotFound.textContent = `❓ ${notFound} Tidak Ada`;
  statFetchError.textContent = `❌ ${error} Gagal`;
}

// Process single track with granular steps (Mengunduh -> Menanamkan)
async function processSingleTrack(track, idx) {
  if (!track || isProcessing) return;

  const statusCell = document.getElementById(`status-cell-${idx}`);
  const rowEl = document.getElementById(`row-track-${idx}`);
  if (rowEl) {
    rowEl.classList.add('row-processing');
    autoFollowRow(rowEl);
  }

  const tolerance = parseFloat(numTolerance.value) || 3.0;
  const overwrite = chkOverwrite.checked;
  const upgradePlain = chkUpgradePlain ? chkUpgradePlain.checked : true;
  const syncedOnly = chkSyncedOnly.checked;

  try {
    // Step 1: Mengunduh
    if (statusCell) {
      statusCell.innerHTML = `<span class="badge badge-downloading">⬇️ Mengunduh...</span>`;
    }

    const fetchRes = await invoke('fetch_track_lyrics', {
      path: track.path,
      toleranceSecs: tolerance,
      syncedOnly: syncedOnly,
      overwrite: overwrite,
      upgradePlain: upgradePlain,
    });

    if (fetchRes.action === 'embed') {
      // Step 2: Menanamkan
      if (statusCell) {
        statusCell.innerHTML = `<span class="badge badge-embedding">💾 Menanamkan...</span>`;
      }

      const embedRes = await invoke('embed_track_lyrics', {
        path: track.path,
        lyrics: fetchRes.lyrics,
        provider: fetchRes.provider,
        isSynced: fetchRes.is_synced,
        isUpgrade: fetchRes.is_upgrade,
      });

      await new Promise(r => setTimeout(r, 220));

      track.lyric_state = embedRes.is_synced ? 'synced' : 'plain';
      track.provider = embedRes.provider;
      updateCounts();
      renderTable();
    } else {
      alert(`Pemberitahuan: ${fetchRes.message}`);
      if (statusCell) statusCell.innerHTML = renderLyricBadge(track.lyric_state, track.provider);
    }
  } catch (err) {
    alert("Error: " + err);
    if (statusCell) statusCell.innerHTML = renderLyricBadge(track.lyric_state, track.provider);
  } finally {
    if (rowEl) rowEl.classList.remove('row-processing');
  }
}

// Modal lyrics preview
async function openPreviewModal(path, title, artist) {
  modalSongTitle.textContent = title || "Lyrics Preview";
  modalSongArtist.textContent = artist || "";
  modalLyricsContent.textContent = "Membaca metadata lirik...";
  lyricsModal.classList.remove('hidden');

  try {
    const lyrics = await invoke('get_track_lyrics', { path });
    modalLyricsContent.textContent = lyrics || "Tidak ada lirik yang tersimpan dalam file ini.";
  } catch (err) {
    modalLyricsContent.textContent = "Gagal membaca lirik: " + err;
  }
}

function closeModal() {
  lyricsModal.classList.add('hidden');
}

function escapeHtml(text) {
  if (!text) return '';
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

document.addEventListener('DOMContentLoaded', setup);
