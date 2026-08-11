const TOOLS = [
  { id: 'merge',         icon: '⊕', name: 'Merge PDFs',      hint: 'Combine multiple files into one',              desc: 'Merge multiple PDF files into one, in the order they were added.',                                                            multi: true },
  { id: 'split',         icon: '⊘', name: 'Split PDF',        hint: 'Split by page ranges',                         desc: 'Split a PDF into multiple files by page range.',                                                                             multi: false },
  { id: 'compress',      icon: '◉', name: 'Compress PDF',     hint: 'Reduce file size',                             desc: 'Optimize PDF size for easier sharing or storage.',                                                                           multi: false },
  { id: 'encrypt',       icon: '⊛', name: 'Encrypt PDF',      hint: 'Set an open password',                         desc: 'Add password protection so only those with the password can open the file.',                                                  multi: false },
  { id: 'decrypt',       icon: '⊖', name: 'Decrypt PDF',      hint: 'Remove existing password',                     desc: 'Remove password protection from a PDF by entering the correct password.',                                                     multi: false },
  { id: 'delete-pages',  icon: '⊟', name: 'Delete Pages',     hint: 'Remove unwanted pages',                        desc: 'Delete specific pages from a PDF.',                                                                                          multi: false, needsPagePick: true },
  { id: 'extract-pages', icon: '⊞', name: 'Extract Pages',    hint: 'Save selected pages as a new file',            desc: 'Extract selected pages into a new PDF file.',                                                                                multi: false, needsPagePick: true },
  { id: 'rotate-pages',  icon: '↻', name: 'Rotate Pages',     hint: 'Rotate specific pages',                        desc: 'Choose pages to rotate and the rotation angle.',                                                                             multi: false, needsPagePick: true, needsRotate: true },
  { id: 'reorder-pages', icon: '⇅', name: 'Reorder Pages',    hint: 'Drag to rearrange page order',                 desc: 'Drag and drop to rearrange the page order of a PDF.',                                                                        multi: false, needsReorder: true },
  { id: 'pdf-to-images', icon: '▣', name: 'PDF to Images',    hint: 'Export each page as a PNG',                    desc: 'Render each PDF page as a PNG image, saved next to the source file.',                                                         multi: false },
  { id: 'images-to-pdf', icon: '▦', name: 'Images to PDF',    hint: 'Combine images into a PDF',                    desc: 'Combine multiple PNG / JPG images into a single PDF, one image per page.',                                                     multi: true,  acceptImages: true },
  { id: 'watermark',     icon: '◈', name: 'Add Watermark',    hint: 'Overlay text watermark on every page',         desc: 'Add a semi-transparent text watermark to every page, e.g. Confidential or Draft.',                                             multi: false },
  { id: 'read',          icon: '▤', name: 'Read & Annotate',  hint: 'View pages, highlight, and mark up',           desc: 'Open a PDF to read, highlight, add sticky notes, and draw on it. Marks are saved next to the file, not into it.',             isReader: true },
];

let currentTool = null;
let files = [];
let pageCount = 0;
let pageThumbnails = new Map();
let selectedPages = new Set();
let reorderOrder = [];
let splitRanges = [];

const mainEl      = document.getElementById('main');
const home        = document.getElementById('home');
const taskView    = document.getElementById('task-view');
const taskTitle   = document.getElementById('task-title');
const taskBody    = document.getElementById('task-body');
const backBtn     = document.getElementById('back-btn');
const toolGrid    = document.getElementById('tool-grid');
const readingSlot = document.getElementById('reading-entry-slot');
const dropOverlay = document.getElementById('drop-overlay');
const readerView  = document.getElementById('reader-view');

// Read & Annotate gets its own wide row above the grid — it's what most
// users opening any PDF want first, so it shouldn't be lost among tool cards.
function renderHome() {
  readingSlot.innerHTML = '';
  const readerTool = TOOLS.find(t => t.isReader);
  if (readerTool) {
    const entry = document.createElement('article');
    entry.className = 'reading-entry';
    entry.innerHTML = `
      <span class="re-box">
        <svg viewBox="0 0 256 256" fill="currentColor"><path d="M224,48H32a8,8,0,0,0-8,8V192a16,16,0,0,0,16,16H216a16,16,0,0,0,16-16V56A8,8,0,0,0,224,48ZM40,56H120v136H40Zm176,136H136V56h80Z"/></svg>
      </span>
      <span class="re-text">
        <span class="name">${readerTool.name}</span>
        <span class="hint">${readerTool.hint}</span>
      </span>
    `;
    entry.addEventListener('click', () => openTool(readerTool.id));
    readingSlot.appendChild(entry);
  }

  toolGrid.innerHTML = '';
  TOOLS.filter(tool => !tool.isReader).forEach(tool => {
    const card = document.createElement('article');
    card.className = 'tool-card';
    card.innerHTML = `
      <span class="icon">${tool.icon}</span>
      <span class="name">${tool.name}</span>
      <span class="hint">${tool.hint}</span>
    `;
    card.addEventListener('click', () => openTool(tool.id));

    card.addEventListener('dragover', e => {
      if (e.dataTransfer.types.includes('Files')) {
        e.preventDefault();
        e.stopPropagation();
        card.classList.add('drag-target');
      }
    });
    card.addEventListener('dragleave', e => {
      if (!card.contains(e.relatedTarget)) card.classList.remove('drag-target');
    });
    card.addEventListener('drop', e => {
      e.preventDefault();
      e.stopPropagation();
      card.classList.remove('drag-target');
      dropOverlay.classList.remove('active');
      // Webview drops never carry real fs paths — open native picker instead
      openTool(tool.id);
    });

    toolGrid.appendChild(card);
  });
}
renderHome();

function openTool(id, droppedFiles) {
  currentTool = TOOLS.find(t => t.id === id);
  if (!currentTool) return;

  if (currentTool.isReader) {
    mainEl.classList.add('hidden');
    readerView.classList.add('active');
    openReader();
    return;
  }

  files = droppedFiles ? Array.from(droppedFiles) : [];
  selectedPages = new Set();
  reorderOrder = [];
  splitRanges = [];
  pageCount = 0;
  pageThumbnails = new Map();

  home.classList.add('hidden');
  taskView.classList.add('active');
  taskTitle.textContent = currentTool.name;
  renderTask();
}

backBtn.addEventListener('click', goHome);

function goHome() {
  currentTool = null;
  files = [];
  mainEl.classList.remove('hidden');
  readerView.classList.remove('active');
  home.classList.remove('hidden');
  taskView.classList.remove('active');
  taskBody.innerHTML = '';
}

function renderTask() {
  taskBody.innerHTML = '';

  if (currentTool.multi || files.length === 0) taskBody.appendChild(makeDropZone());

  const fileListEl = document.createElement('section');
  fileListEl.className = 'file-list';
  fileListEl.id = 'file-list';
  taskBody.appendChild(fileListEl);
  renderFileList();

  if (currentTool.id === 'compress')    taskBody.appendChild(makeCompressControls());
  if (currentTool.id === 'encrypt')     taskBody.appendChild(makePasswordField('encrypt-pwd', 'Set password'));
  if (currentTool.id === 'decrypt')     taskBody.appendChild(makePasswordField('decrypt-pwd', 'Enter existing password'));
  if (currentTool.id === 'split')       taskBody.appendChild(makeSplitControls());
  if (currentTool.needsPagePick)        taskBody.appendChild(makePagePicker());
  if (currentTool.needsRotate)          taskBody.appendChild(makeRotatePicker());
  if (currentTool.needsReorder)         taskBody.appendChild(makeReorderList());
  if (currentTool.id === 'pdf-to-images') taskBody.appendChild(makeDpiControls());
  if (currentTool.id === 'watermark')   taskBody.appendChild(makeWatermarkControls());

  const runBtn = document.createElement('button');
  runBtn.className = 'btn-primary';
  runBtn.id = 'run-btn';
  runBtn.textContent = 'Run';
  runBtn.addEventListener('click', execute);
  taskBody.appendChild(runBtn);
}

function makeDropZone() {
  const zone = document.createElement('section');
  zone.className = 'drop-zone';
  const isImg = !!currentTool.acceptImages;
  const hint    = currentTool.multi ? (isImg ? 'Click to select images' : 'Click to select PDF files') : 'Click to select a PDF file';
  const subhint = isImg ? 'Supports PNG / JPG / WebP — processed locally' : 'Files are processed locally — nothing is uploaded to any server';
  zone.innerHTML = `
    <span class="dz-icon">${isImg ? '▦' : '▭'}</span>
    <span class="dz-text">${hint}</span>
    <span class="dz-hint">${subhint}</span>
  `;

  // Native picker because webview File objects never expose real fs paths
  zone.addEventListener('click', () => pickFilesNative());
  zone.addEventListener('dragover', e => { e.preventDefault(); zone.classList.add('drag-over'); });
  zone.addEventListener('dragleave', () => zone.classList.remove('drag-over'));
  zone.addEventListener('drop', e => { e.preventDefault(); zone.classList.remove('drag-over'); pickFilesNative(); });
  return zone;
}

function pickFilesNative() {
  const isImg = !!currentTool.acceptImages;
  sendIpc({ action: 'pick_files', kind: isImg ? 'image' : 'pdf', multiple: !!currentTool.multi })
    .then(res => {
      if (res.status === 'paths' && res.paths && res.paths.length) {
        addFiles(res.paths.map(p => ({ path: p, name: p.split(/[\\/]/).pop() })));
      }
    })
    .catch(err => toast('error', String(err)));
}

function addFiles(newFiles) {
  files = currentTool.multi ? [...files, ...newFiles] : newFiles.slice(0, 1);
  renderFileList();
  afterFilesChange();
}

function renderFileList() {
  const el = document.getElementById('file-list');
  if (!el) return;
  el.innerHTML = '';
  files.forEach((f, i) => {
    const item = document.createElement('article');
    item.className = 'file-item';
    const isReorder = currentTool.needsReorder;
    item.innerHTML = `
      ${isReorder ? '<span class="fi-drag" title="Drag to reorder">⠿</span>' : ''}
      <span class="fi-icon">▭</span>
      <span class="fi-name" title="${f.path || f.name}">${f.name}</span>
      <button class="fi-remove" data-idx="${i}" title="Remove">✕</button>
    `;
    item.querySelector('.fi-remove').addEventListener('click', e => {
      files.splice(parseInt(e.currentTarget.dataset.idx, 10), 1);
      renderFileList();
      afterFilesChange();
    });
    el.appendChild(item);
  });
}

function afterFilesChange() {
  if (files.length === 0 || !files[0].path) return;
  if (!currentTool.needsPagePick && !currentTool.needsReorder) return;

  const path = files[0].path;
  // Fetch count first so skeleton grid renders immediately, thumbnails fill in after
  sendIpc({ action: 'get_page_count', path })
    .then(res => {
      if (res.status !== 'page_count') return;
      pageCount = res.count;
      if (currentTool.needsPagePick) renderPageChips();
      if (currentTool.needsReorder)  renderReorderItems();
      return sendIpc({ action: 'get_page_thumbnails', path });
    })
    .then(res => {
      if (!res || res.status !== 'thumbnails') return;
      pageThumbnails = new Map(res.pages.map(p => [p.page, p.data_url]));
      if (currentTool.needsPagePick) renderPageChips();
      if (currentTool.needsReorder)  renderReorderItems();
    });
}

function formatSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(0) + ' KB';
  return (bytes / 1048576).toFixed(1) + ' MB';
}

function makeCompressControls() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.innerHTML = `
    <label class="field-label">Image Quality</label>
    <div class="range-row">
      <input class="field-input" type="range" id="quality-slider" min="10" max="100" value="75" step="5" />
      <span class="range-val" id="quality-val">75</span>
    </div>
  `;
  wrap.querySelector('#quality-slider').addEventListener('input', e => {
    wrap.querySelector('#quality-val').textContent = e.target.value;
  });
  return wrap;
}

function makePasswordField(id, label) {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.innerHTML = `
    <label class="field-label" for="${id}">${label}</label>
    <input class="field-input" type="password" id="${id}" placeholder="••••••••" autocomplete="off" />
  `;
  return wrap;
}

let splitRangeCount = 1;

function makeSplitControls() {
  const wrap = document.createElement('section');
  wrap.id = 'split-wrap';
  wrap.className = 'field-group';
  wrap.innerHTML = '<label class="field-label">Split Ranges (page numbers, starting at 1)</label>';
  renderSplitRows(wrap);

  const addBtn = document.createElement('button');
  addBtn.className = 'btn-primary';
  addBtn.style.cssText = 'background:var(--surface);color:var(--text);border:1px solid var(--border);margin-top:6px;';
  addBtn.textContent = '+ Add Range';
  addBtn.addEventListener('click', () => { splitRangeCount++; renderSplitRows(wrap); });
  wrap.appendChild(addBtn);
  return wrap;
}

function renderSplitRows(wrap) {
  wrap.querySelectorAll('.split-row').forEach(el => el.remove());
  for (let i = 0; i < splitRangeCount; i++) {
    const row = document.createElement('span');
    row.className = 'split-row range-row';
    row.style.marginBottom = '6px';
    row.innerHTML = `
      <input class="field-input" type="number" placeholder="Start page" min="1" style="width:50%" id="split-start-${i}" />
      <span style="color:var(--text-dim);padding:0 4px;">—</span>
      <input class="field-input" type="number" placeholder="End page" min="1" style="width:50%" id="split-end-${i}" />
    `;
    wrap.appendChild(row);
  }
}

function makePagePicker() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.id = 'page-picker-wrap';
  wrap.innerHTML = `<label class="field-label">Select Pages (shown after file loads)</label>
    <nav class="page-grid" id="page-grid"></nav>`;
  return wrap;
}

function renderPageChips() {
  const grid = document.getElementById('page-grid');
  if (!grid) return;
  grid.innerHTML = '';
  const keepSelected = new Set(selectedPages);
  selectedPages = new Set();

  for (let i = 1; i <= pageCount; i++) {
    const chip = document.createElement('article');
    chip.className = 'page-chip';
    chip.dataset.page = i;
    const dataUrl = pageThumbnails.get(i);
    chip.innerHTML = `
      <span class="pc-thumb-wrap${dataUrl ? '' : ' pc-loading'}">
        ${dataUrl ? `<img class="pc-thumb" src="${dataUrl}" alt="Page ${i}" />` : ''}
      </span>
      <span class="pc-num">${i}</span>
    `;
    if (keepSelected.has(i)) { selectedPages.add(i); chip.classList.add('selected'); }
    chip.addEventListener('click', () => {
      const p = parseInt(chip.dataset.page, 10);
      if (selectedPages.has(p)) { selectedPages.delete(p); chip.classList.remove('selected'); }
      else { selectedPages.add(p); chip.classList.add('selected'); }
    });
    grid.appendChild(chip);
  }
}

function makeRotatePicker() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.id = 'rotate-picker-wrap';
  wrap.innerHTML = `<label class="field-label">Rotation</label>
    <nav class="rotate-options">
      <span class="rotate-chip selected" data-deg="90">90° Clockwise</span>
      <span class="rotate-chip" data-deg="180">180°</span>
      <span class="rotate-chip" data-deg="270">90° Counter-clockwise</span>
    </nav>`;
  wrap.querySelectorAll('.rotate-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      wrap.querySelectorAll('.rotate-chip').forEach(c => c.classList.remove('selected'));
      chip.classList.add('selected');
    });
  });
  return wrap;
}

function getSelectedDegrees() {
  const chip = taskBody.querySelector('.rotate-chip.selected');
  return chip ? parseInt(chip.dataset.deg, 10) : 90;
}

function makeReorderList() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.id = 'reorder-wrap';
  wrap.innerHTML = `<label class="field-label">Drag to Reorder Pages (shown after file loads)</label>
    <section class="file-list" id="reorder-list"></section>`;
  return wrap;
}

function renderReorderItems() {
  const list = document.getElementById('reorder-list');
  if (!list) return;
  list.innerHTML = '';
  if (reorderOrder.length !== pageCount) reorderOrder = Array.from({ length: pageCount }, (_, i) => i + 1);

  reorderOrder.forEach((pageNum, listIdx) => {
    const item = document.createElement('article');
    item.className = 'file-item';
    item.draggable = true;
    item.dataset.listIdx = listIdx;
    const dataUrl = pageThumbnails.get(pageNum);
    item.innerHTML = `
      <span class="fi-drag">⠿</span>
      <span class="fi-thumb-wrap${dataUrl ? '' : ' pc-loading'}">
        ${dataUrl ? `<img src="${dataUrl}" alt="Page ${pageNum}" />` : ''}
      </span>
      <span class="fi-name">Page ${pageNum}</span>
    `;
    item.addEventListener('dragstart', e => { e.dataTransfer.setData('text/plain', listIdx); item.classList.add('dragging'); });
    item.addEventListener('dragend',   () => item.classList.remove('dragging'));
    item.addEventListener('dragover',  e => e.preventDefault());
    item.addEventListener('drop', e => {
      e.preventDefault();
      const from = parseInt(e.dataTransfer.getData('text/plain'), 10);
      const to   = parseInt(item.dataset.listIdx, 10);
      if (from !== to) { reorderOrder.splice(to, 0, reorderOrder.splice(from, 1)[0]); renderReorderItems(); }
    });
    list.appendChild(item);
  });
}

function makeDpiControls() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.innerHTML = `
    <label class="field-label">Output Resolution (DPI)</label>
    <span class="range-row">
      <input class="field-input" type="range" id="dpi-slider" min="72" max="300" value="150" step="6" />
      <span class="range-val" id="dpi-val">150</span>
    </span>
    <span class="field-label" style="margin-top:4px;font-size:11px;color:#555">
      72 = Screen preview &nbsp;·&nbsp; 150 = Standard print &nbsp;·&nbsp; 300 = High-res print
    </span>
  `;
  wrap.querySelector('#dpi-slider').addEventListener('input', e => {
    wrap.querySelector('#dpi-val').textContent = e.target.value;
  });
  return wrap;
}

function makeWatermarkControls() {
  const wrap = document.createElement('section');
  wrap.className = 'field-group';
  wrap.innerHTML = `
    <label class="field-label" for="wm-text">Watermark Text</label>
    <input class="field-input" type="text" id="wm-text" placeholder="e.g. Confidential / Draft / DO NOT COPY" maxlength="40" />

    <label class="field-label" style="margin-top:12px">Font Size</label>
    <span class="range-row">
      <input class="field-input" type="range" id="wm-size" min="20" max="120" value="60" step="4" />
      <span class="range-val" id="wm-size-val">60</span>
    </span>

    <label class="field-label" style="margin-top:12px">Opacity</label>
    <span class="range-row">
      <input class="field-input" type="range" id="wm-opacity" min="5" max="60" value="15" step="1" />
      <span class="range-val" id="wm-opacity-val">15%</span>
    </span>

    <label class="field-label" style="margin-top:12px">Rotation</label>
    <nav class="rotate-options" id="wm-rotation-opts">
      <span class="rotate-chip" data-deg="0">Horizontal</span>
      <span class="rotate-chip selected" data-deg="45">45°</span>
      <span class="rotate-chip" data-deg="90">Vertical</span>
    </nav>
  `;
  wrap.querySelector('#wm-size').addEventListener('input', e => { wrap.querySelector('#wm-size-val').textContent = e.target.value; });
  wrap.querySelector('#wm-opacity').addEventListener('input', e => { wrap.querySelector('#wm-opacity-val').textContent = e.target.value + '%'; });
  wrap.querySelectorAll('#wm-rotation-opts .rotate-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      wrap.querySelectorAll('#wm-rotation-opts .rotate-chip').forEach(c => c.classList.remove('selected'));
      chip.classList.add('selected');
    });
  });
  return wrap;
}

// IPC responses are delivered globally via window.__ipc_cb and routed back
// to the right Promise by id, so concurrent calls don't interfere.
const pendingIpc = new Map();
window.__ipc_cb = (data) => {
  const { id, ...rest } = data || {};
  const entry = pendingIpc.get(id);
  if (entry) { pendingIpc.delete(id); entry.resolve(rest); }
};

function sendIpc(msg) {
  return new Promise((resolve, reject) => {
    const id = `${Date.now()}_${Math.random().toString(36).slice(2)}`;
    pendingIpc.set(id, { resolve, reject });
    try {
      window.ipc.postMessage(JSON.stringify({ id, ...msg }));
    } catch (e) {
      pendingIpc.delete(id);
      setTimeout(() => resolve({ status: 'ok', output: '/tmp/output.pdf', message: '(Simulated success)' }), 600);
    }
  });
}

async function execute() {
  if (!validateInput()) return;
  const runBtn = document.getElementById('run-btn');
  runBtn.classList.add('running');
  runBtn.innerHTML = '<span class="spinner"></span> Processing…';
  try {
    const result = await buildAndSend();
    runBtn.classList.remove('running');
    if (result.status === 'ok') {
      toast('success', result.message || 'Done!', result.output);
      runBtn.textContent = '✓ Done';
    } else {
      toast('error', result.message || `Operation failed. (raw: ${JSON.stringify(result)})`);
      runBtn.textContent = 'Run';
    }
  } catch (err) {
    runBtn.classList.remove('running');
    runBtn.textContent = 'Run';
    toast('error', String(err));
  }
}

function validateInput() {
  if (files.length === 0) { toast('error', 'Please select a PDF file first.'); return false; }
  if (currentTool.id === 'merge' && files.length < 2) { toast('error', 'Merge requires at least 2 files.'); return false; }
  if (currentTool.needsPagePick && selectedPages.size === 0) { toast('error', 'Please select at least one page.'); return false; }
  return true;
}

async function buildAndSend() {
  const path  = files[0]?.path || files[0]?.name;
  const paths = files.map(f => f.path || f.name);

  switch (currentTool.id) {
    case 'merge':   return sendIpc({ action: 'merge', paths });
    case 'split': {
      const ranges = [];
      for (let i = 0; i < splitRangeCount; i++) {
        const s = parseInt(document.getElementById(`split-start-${i}`)?.value, 10);
        const e = parseInt(document.getElementById(`split-end-${i}`)?.value, 10);
        if (!isNaN(s) && !isNaN(e)) ranges.push({ start: s, end: e });
      }
      if (ranges.length === 0) { toast('error', 'Please add at least one split range.'); return { status: 'error' }; }
      return sendIpc({ action: 'split', path, ranges });
    }
    case 'compress': {
      const q = parseInt(document.getElementById('quality-slider')?.value || '75', 10);
      return sendIpc({ action: 'compress', path, quality: q });
    }
    case 'encrypt': {
      const pwd = document.getElementById('encrypt-pwd')?.value;
      if (!pwd) { toast('error', 'Please enter a password.'); return { status: 'error' }; }
      return sendIpc({ action: 'encrypt', path, password: pwd });
    }
    case 'decrypt': {
      const pwd = document.getElementById('decrypt-pwd')?.value;
      if (!pwd) { toast('error', 'Please enter the existing password.'); return { status: 'error' }; }
      return sendIpc({ action: 'decrypt', path, password: pwd });
    }
    case 'delete-pages':  return sendIpc({ action: 'delete_pages',  path, pages: [...selectedPages] });
    case 'extract-pages': return sendIpc({ action: 'extract_pages', path, pages: [...selectedPages] });
    case 'rotate-pages':  return sendIpc({ action: 'rotate_pages',  path, pages: [...selectedPages], degrees: getSelectedDegrees() });
    case 'reorder-pages': return sendIpc({ action: 'reorder_pages', path, order: reorderOrder });
    case 'pdf-to-images': {
      const dpi = parseInt(document.getElementById('dpi-slider')?.value || '150', 10);
      return sendIpc({ action: 'pdf_to_images', path, dpi });
    }
    case 'images-to-pdf': {
      const stem = (files[0]?.name || 'images').replace(/\.[^.]+$/, '');
      return sendIpc({ action: 'images_to_pdf', paths, output_name: `${stem}_combined.pdf` });
    }
    case 'watermark': {
      const text = document.getElementById('wm-text')?.value?.trim();
      if (!text) { toast('error', 'Please enter watermark text.'); return { status: 'error' }; }
      const fontSize   = parseFloat(document.getElementById('wm-size')?.value || '60');
      const opacityPct = parseFloat(document.getElementById('wm-opacity')?.value || '15');
      const rotChip    = taskBody.querySelector('#wm-rotation-opts .rotate-chip.selected');
      const rotation   = rotChip ? parseFloat(rotChip.dataset.deg) : 45;
      return sendIpc({ action: 'add_watermark', path, text, font_size: fontSize, opacity: opacityPct / 100, rotation });
    }
    default: return { status: 'error', message: 'Unknown tool.' };
  }
}

let toastTimer;
function toast(type, message, outputPath) {
  const el  = document.getElementById('toast');
  const msg = document.getElementById('toast-msg');
  el.className = type;
  msg.innerHTML = outputPath
    ? `${message} <button class="reveal-link" onclick="revealFile('${outputPath}')">Show in Folder ↗</button>`
    : '';
  if (!outputPath) msg.textContent = message;
  el.classList.add('visible');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('visible'), 4000);
}

function revealFile(path) {
  sendIpc({ action: 'reveal_in_finder', path });
}

// ── Reader ──────────────────────────────────────────────────────────────────
// Separate state from generic task flow — reader manages one open doc at a
// time rather than a list of files to batch-process.

let readerPath = null;
let readerPage = 1;
let readerPageCount = 0;
let readerZoomPct = 100;
let readerDpi = 96;       // 96dpi = 100% zoom (1 CSS px per PDF pt at 72dpi base)
let readerMode = 'pan';   // pan | highlight | note | ink
let readerColor = '#f2c40c';
let readerAnnotations = {};
let readerDragStart = null;
let readerInkDrawing = null;
let readerDirty = false;

const readerControls     = document.getElementById('reader-controls');
const readerDocTitleEl   = document.getElementById('reader-doc-title');
const readerStage        = document.getElementById('reader-stage');
const readerEmpty        = document.getElementById('reader-empty');
const readerPageWrap     = document.getElementById('reader-page-wrap');
const readerPageCanvas   = document.getElementById('reader-page-canvas');
const readerAnnotCanvas  = document.getElementById('reader-annot-canvas');
const readerPageCtx      = readerPageCanvas.getContext('2d');
const readerAnnotCtx     = readerAnnotCanvas.getContext('2d');
const readerStatusbar    = document.getElementById('reader-statusbar');
const readerPageNumInput = document.getElementById('reader-page-num');
const readerPageTotalEl  = document.getElementById('reader-page-total');
const readerZoomSlider   = document.getElementById('reader-zoom');
const readerZoomLabel    = document.getElementById('reader-zoom-label');
const readerSaveBtn      = document.getElementById('reader-save-btn');
const readerNotePopup    = document.getElementById('reader-note-popup');
const readerNoteInput    = document.getElementById('reader-note-input');

function readerScale() { return readerDpi / 72; }
function readerPtToPx(x, y) { const s = readerScale(); return { x: x * s, y: y * s }; }
function readerPxToPt(x, y) { const s = readerScale(); return { x: x / s, y: y / s }; }

function readerEventToCanvasPx(evt) {
  const rect  = readerAnnotCanvas.getBoundingClientRect();
  const scaleX = readerAnnotCanvas.width  / rect.width;
  const scaleY = readerAnnotCanvas.height / rect.height;
  return { x: (evt.clientX - rect.left) * scaleX, y: (evt.clientY - rect.top) * scaleY };
}

function openReader() {
  readerPath = null; readerPage = 1; readerPageCount = 0;
  readerAnnotations = {}; readerDirty = false;
  readerMode = 'pan'; readerZoomPct = 100; readerDpi = 96;
  readerZoomSlider.value = 100;
  readerZoomLabel.textContent = '100%';
  updateReaderModeButtons();
  readerControls.classList.remove('visible');
  readerStatusbar.classList.remove('visible');
  readerPageWrap.classList.remove('visible');
  readerEmpty.classList.add('visible');
  readerDocTitleEl.textContent = '';
  readerSaveBtn.classList.remove('dirty');
  hideReaderNotePopup();
}

function pickAndOpenReaderPdf() {
  sendIpc({ action: 'pick_files', kind: 'pdf', multiple: false })
    .then(res => { if (res.status === 'paths' && res.paths?.length) openReaderPath(res.paths[0]); })
    .catch(err => toast('error', String(err)));
}

async function openReaderPath(path) {
  try {
    const countRes = await sendIpc({ action: 'get_page_count', path });
    if (countRes.status !== 'page_count') { toast('error', countRes.message || 'Could not open that file.'); return; }

    readerPath = path;
    readerPageCount = countRes.count;
    readerPage = 1;
    readerDocTitleEl.textContent = path.split(/[\\/]/).pop();
    readerPageTotalEl.textContent = String(readerPageCount);

    const annotRes = await sendIpc({ action: 'load_annotations', path });
    readerAnnotations = annotRes.status === 'annotations' ? (annotRes.pages || {}) : {};
    readerDirty = false;
    readerSaveBtn.classList.remove('dirty');

    readerEmpty.classList.remove('visible');
    readerControls.classList.add('visible');
    readerStatusbar.classList.add('visible');
    readerPageWrap.classList.add('visible');

    await goToReaderPage(1);
  } catch (err) {
    toast('error', String(err));
  }
}

async function goToReaderPage(n) {
  if (!readerPath || n < 1 || n > readerPageCount) return;
  readerPage = n;
  readerPageNumInput.value = String(n);
  updateReaderNavButtons();
  await renderReaderPage();
}

async function renderReaderPage() {
  if (!readerPath) return;
  try {
    const res = await sendIpc({ action: 'render_page', path: readerPath, page: readerPage, dpi: readerDpi });
    if (res.status !== 'page') { toast('error', res.message || 'Failed to render that page.'); return; }
    await drawReaderPageImage(res.data_url);
  } catch (err) {
    toast('error', String(err));
  }
}

function drawReaderPageImage(dataUrl) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      readerPageCanvas.width  = img.width;
      readerPageCanvas.height = img.height;
      readerAnnotCanvas.width  = img.width;
      readerAnnotCanvas.height = img.height;
      readerPageCanvas.style.width  = img.width  + 'px';
      readerPageCanvas.style.height = img.height + 'px';
      readerAnnotCanvas.style.width  = img.width  + 'px';
      readerAnnotCanvas.style.height = img.height + 'px';
      readerPageCtx.drawImage(img, 0, 0);
      redrawReaderAnnotations();
      resolve();
    };
    img.onerror = reject;
    img.src = dataUrl;
  });
}

function updateReaderNavButtons() {
  document.getElementById('reader-prev-btn').disabled = readerPage <= 1;
  document.getElementById('reader-next-btn').disabled = readerPage >= readerPageCount;
}

function redrawReaderAnnotations() {
  readerAnnotCtx.clearRect(0, 0, readerAnnotCanvas.width, readerAnnotCanvas.height);
  for (const a of (readerAnnotations[String(readerPage)] || [])) drawReaderAnnotation(a);
}

function drawReaderAnnotation(a) {
  if (a.type === 'highlight') {
    const p1 = readerPtToPx(a.rect.x, a.rect.y);
    const p2 = readerPtToPx(a.rect.x + a.rect.w, a.rect.y + a.rect.h);
    readerAnnotCtx.fillStyle = hexToRgba(a.color, 0.35);
    readerAnnotCtx.fillRect(p1.x, p1.y, p2.x - p1.x, p2.y - p1.y);
  } else if (a.type === 'note') {
    const p = readerPtToPx(a.point.x, a.point.y);
    readerAnnotCtx.beginPath();
    readerAnnotCtx.arc(p.x, p.y, 9, 0, Math.PI * 2);
    readerAnnotCtx.fillStyle = a.color;
    readerAnnotCtx.fill();
    readerAnnotCtx.strokeStyle = 'rgba(0,0,0,0.25)';
    readerAnnotCtx.lineWidth = 1;
    readerAnnotCtx.stroke();
    readerAnnotCtx.fillStyle = '#24221d';
    readerAnnotCtx.font = 'bold 11px system-ui';
    readerAnnotCtx.textAlign = 'center';
    readerAnnotCtx.textBaseline = 'middle';
    readerAnnotCtx.fillText('!', p.x, p.y + 0.5);
  } else if (a.type === 'ink') {
    if (!a.points || a.points.length < 2) return;
    readerAnnotCtx.strokeStyle = a.color;
    readerAnnotCtx.lineWidth = Math.max(1.5, (a.width || 2) * readerScale() * 0.5);
    readerAnnotCtx.lineJoin = 'round';
    readerAnnotCtx.lineCap = 'round';
    readerAnnotCtx.beginPath();
    a.points.forEach((pt, i) => {
      const p = readerPtToPx(pt.x, pt.y);
      if (i === 0) readerAnnotCtx.moveTo(p.x, p.y); else readerAnnotCtx.lineTo(p.x, p.y);
    });
    readerAnnotCtx.stroke();
  }
}

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

function rectFrom(a, b) {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(b.x - a.x), h: Math.abs(b.y - a.y) };
}

function pushReaderAnnotation(a) {
  const key = String(readerPage);
  if (!readerAnnotations[key]) readerAnnotations[key] = [];
  readerAnnotations[key].push(a);
  redrawReaderAnnotations();
  markReaderDirty();
}

function markReaderDirty() { readerDirty = true; readerSaveBtn.classList.add('dirty'); }

async function saveReaderAnnotations(opts) {
  if (!readerPath) return;
  const silent = !!(opts && opts.silent);
  try {
    const res = await sendIpc({ action: 'save_annotations', path: readerPath, annotations: readerAnnotations });
    if (res.status === 'ok') {
      readerDirty = false;
      readerSaveBtn.classList.remove('dirty');
      if (!silent) toast('success', 'Annotations saved.');
    } else if (!silent) {
      toast('error', res.message || 'Failed to save annotations.');
    }
  } catch (err) {
    if (!silent) toast('error', String(err));
  }
}

function undoReaderLast() {
  const list = readerAnnotations[String(readerPage)];
  if (list && list.length) { list.pop(); redrawReaderAnnotations(); markReaderDirty(); }
}

// Auto-saves silently before navigating away so unsaved marks are never lost
// without requiring a native confirm() dialog
async function leaveReaderSafely(next) {
  if (readerDirty) await saveReaderAnnotations({ silent: true });
  next();
}

let readerNoteEditIndex = -1;
const readerNoteDeleteBtn = document.getElementById('reader-note-delete');
const NOTE_HIT_RADIUS = 12;

function findNoteAt(canvasX, canvasY) {
  const list = readerAnnotations[String(readerPage)] || [];
  for (let i = list.length - 1; i >= 0; i--) {
    const a = list[i];
    if (a.type !== 'note') continue;
    const p = readerPtToPx(a.point.x, a.point.y);
    const dx = p.x - canvasX, dy = p.y - canvasY;
    if (dx * dx + dy * dy <= NOTE_HIT_RADIUS * NOTE_HIT_RADIUS) return i;
  }
  return -1;
}

function showReaderNotePopup(screenX, screenY, existingText) {
  const isEditing = existingText !== undefined;
  readerNotePopup.classList.toggle('editing', isEditing);
  readerNoteInput.value = isEditing ? existingText : '';

  // Measure off-screen first, then clamp to viewport so the popup never
  // overflows at the right or bottom edge
  readerNotePopup.style.left = '-9999px';
  readerNotePopup.style.top  = '-9999px';
  readerNotePopup.classList.add('visible');
  const w = readerNotePopup.offsetWidth;
  const h = readerNotePopup.offsetHeight;
  readerNotePopup.style.left = Math.max(8, Math.min(screenX, window.innerWidth  - w - 12)) + 'px';
  readerNotePopup.style.top  = Math.max(8, Math.min(screenY, window.innerHeight - h - 12)) + 'px';
  readerNoteInput.focus();
  readerNoteInput.setSelectionRange(readerNoteInput.value.length, readerNoteInput.value.length);
}

function hideReaderNotePopup() { readerNotePopup.classList.remove('visible', 'editing'); readerNoteEditIndex = -1; }

let pendingNotePoint = null;

function saveNotePopup() {
  const text = readerNoteInput.value.trim();
  const key  = String(readerPage);
  if (readerNoteEditIndex >= 0) {
    const list = readerAnnotations[key];
    if (!text) list.splice(readerNoteEditIndex, 1);
    else list[readerNoteEditIndex].text = text;
    redrawReaderAnnotations();
    markReaderDirty();
  } else if (text) {
    pushReaderAnnotation({ type: 'note', color: readerColor, point: pendingNotePoint, text });
  }
  hideReaderNotePopup();
}

function deleteNotePopup() {
  if (readerNoteEditIndex >= 0) {
    readerAnnotations[String(readerPage)].splice(readerNoteEditIndex, 1);
    redrawReaderAnnotations();
    markReaderDirty();
  }
  hideReaderNotePopup();
}

function addOrEditReaderNoteAt(pt, canvasX, canvasY, screenX, screenY) {
  const hitIdx = findNoteAt(canvasX, canvasY);
  if (hitIdx >= 0) { readerNoteEditIndex = hitIdx; showReaderNotePopup(screenX, screenY, readerAnnotations[String(readerPage)][hitIdx].text); }
  else { readerNoteEditIndex = -1; pendingNotePoint = pt; showReaderNotePopup(screenX, screenY); }
}

document.getElementById('reader-note-ok').addEventListener('click', saveNotePopup);
document.getElementById('reader-note-cancel').addEventListener('click', hideReaderNotePopup);
readerNoteDeleteBtn.addEventListener('click', deleteNotePopup);
readerNoteInput.addEventListener('keydown', e => {
  if (e.key === 'Escape') { e.preventDefault(); hideReaderNotePopup(); }
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); saveNotePopup(); }
});

readerAnnotCanvas.addEventListener('pointerdown', evt => {
  if (readerMode === 'pan' || !readerPath) return;
  const { x, y } = readerEventToCanvasPx(evt);
  const pt = readerPxToPt(x, y);
  if (readerMode === 'highlight') { readerDragStart = pt; readerAnnotCanvas.setPointerCapture(evt.pointerId); }
  else if (readerMode === 'note') { addOrEditReaderNoteAt(pt, x, y, evt.clientX, evt.clientY); }
  else if (readerMode === 'ink')  { readerInkDrawing = [pt]; readerAnnotCanvas.setPointerCapture(evt.pointerId); }
});

readerAnnotCanvas.addEventListener('pointermove', evt => {
  if (readerMode === 'highlight' && readerDragStart) {
    const { x, y } = readerEventToCanvasPx(evt);
    redrawReaderAnnotations();
    drawReaderAnnotation({ type: 'highlight', color: readerColor, rect: rectFrom(readerDragStart, readerPxToPt(x, y)) });
  } else if (readerMode === 'ink' && readerInkDrawing) {
    const { x, y } = readerEventToCanvasPx(evt);
    readerInkDrawing.push(readerPxToPt(x, y));
    redrawReaderAnnotations();
    drawReaderAnnotation({ type: 'ink', color: readerColor, width: 2, points: readerInkDrawing });
  }
});

readerAnnotCanvas.addEventListener('pointerup', evt => {
  if (readerMode === 'highlight' && readerDragStart) {
    const { x, y } = readerEventToCanvasPx(evt);
    const rect = rectFrom(readerDragStart, readerPxToPt(x, y));
    if (rect.w > 2 && rect.h > 2) pushReaderAnnotation({ type: 'highlight', color: readerColor, rect });
    readerDragStart = null;
    redrawReaderAnnotations();
  } else if (readerMode === 'ink' && readerInkDrawing) {
    if (readerInkDrawing.length > 1) pushReaderAnnotation({ type: 'ink', color: readerColor, width: 2, points: readerInkDrawing });
    readerInkDrawing = null;
    redrawReaderAnnotations();
  }
});

readerAnnotCanvas.addEventListener('pointerleave', () => {
  if (readerDragStart || readerInkDrawing) { readerDragStart = null; readerInkDrawing = null; redrawReaderAnnotations(); }
});

function updateReaderModeButtons() {
  document.querySelectorAll('#reader-mode-group .reader-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.mode === readerMode);
  });
  readerStage.classList.remove('mode-pan', 'mode-highlight', 'mode-note', 'mode-ink');
  readerStage.classList.add('mode-' + readerMode);
}

document.querySelectorAll('#reader-mode-group .reader-btn').forEach(btn => {
  btn.addEventListener('click', () => { readerMode = btn.dataset.mode; updateReaderModeButtons(); });
});

document.getElementById('reader-color').addEventListener('input', e => {
  readerColor = e.target.value;
  document.getElementById('reader-color-swatch').style.background = readerColor;
});

document.getElementById('reader-open-btn').addEventListener('click',       () => leaveReaderSafely(pickAndOpenReaderPdf));
document.getElementById('reader-empty-open-btn').addEventListener('click', pickAndOpenReaderPdf);
document.getElementById('reader-back-btn').addEventListener('click',       () => leaveReaderSafely(goHome));
readerSaveBtn.addEventListener('click', () => saveReaderAnnotations());
document.getElementById('reader-undo-btn').addEventListener('click', undoReaderLast);
document.getElementById('reader-prev-btn').addEventListener('click', () => goToReaderPage(readerPage - 1));
document.getElementById('reader-next-btn').addEventListener('click', () => goToReaderPage(readerPage + 1));

readerPageNumInput.addEventListener('change', () => {
  const n = parseInt(readerPageNumInput.value, 10);
  if (!Number.isNaN(n)) goToReaderPage(n); else readerPageNumInput.value = String(readerPage);
});

readerZoomSlider.addEventListener('input', async e => {
  readerZoomPct = Number(e.target.value);
  readerDpi = Math.round(96 * (readerZoomPct / 100));
  readerZoomLabel.textContent = readerZoomPct + '%';
  await renderReaderPage();
});

window.addEventListener('keydown', e => {
  if (!readerView.classList.contains('active')) return;
  if ((e.target?.tagName || '') === 'INPUT') return;
  if (e.key === 'ArrowRight') goToReaderPage(readerPage + 1);
  if (e.key === 'ArrowLeft')  goToReaderPage(readerPage - 1);
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); saveReaderAnnotations(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') { e.preventDefault(); undoReaderLast(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') { e.preventDefault(); leaveReaderSafely(pickAndOpenReaderPdf); }
});

document.addEventListener('dragenter', e => { if (e.dataTransfer.types.includes('Files')) dropOverlay.classList.add('active'); });
document.addEventListener('dragleave', e => { if (!e.relatedTarget) dropOverlay.classList.remove('active'); });
document.addEventListener('dragover',  e => e.preventDefault());
document.addEventListener('drop', e => {
  e.preventDefault();
  dropOverlay.classList.remove('active');
  if (currentTool?.isReader) { pickAndOpenReaderPdf(); return; }

  const allFiles = Array.from(e.dataTransfer.files);
  const pdfs   = allFiles.filter(f => f.name.endsWith('.pdf'));
  const images = allFiles.filter(f => f.type.startsWith('image/'));

  if (currentTool) { addFiles(currentTool.acceptImages ? images : pdfs); return; }

  if (images.length > 0 && pdfs.length === 0) openTool('images-to-pdf', images);
  else if (pdfs.length > 1) openTool('merge', pdfs);
  else if (pdfs.length === 1) openTool('compress', pdfs);
});
