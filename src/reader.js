// ohPDF Reader — vanilla JS, no framework, no bundler-only deps beyond
// @tauri-apps/api and @tauri-apps/plugin-dialog (both tiny, tree-shaken by Vite).
//
// [VERIFY] 本文件在无网络/无 Rust 工具链的沙箱中编写，未实际跑过 `tauri dev` 验证。
// 已在 README.md 的“已知风险”一节标出最可能需要调整的位置。

import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

// ---------------------------------------------------------------
// State
// ---------------------------------------------------------------
let currentPath = null;
let currentPage = 0;
let pageCount = 0;
let dpi = 96;                 // 96dpi = 100% zoom (1 CSS px per PDF pt)
let mode = 'pan';             // pan | highlight | note | ink
let color = '#f2c40c';
let annotationsByPage = {};   // { "0": [ {type,...}, ... ] }
let dragStart = null;         // in-progress highlight rect (PDF pt space)
let inkDrawing = null;        // in-progress ink stroke (array of PDF pt points)
let dirty = false;

// ---------------------------------------------------------------
// DOM refs
// ---------------------------------------------------------------
const $ = (id) => document.getElementById(id);

const stage = document.querySelector('.stage');
const emptyState = $('emptyState');
const pageWrap = $('pageWrap');
const pageCanvas = $('pageCanvas');
const annotCanvas = $('annotCanvas');
const pageCtx = pageCanvas.getContext('2d');
const annotCtx = annotCanvas.getContext('2d');

const pageNumInput = $('pageNum');
const pageTotalEl = $('pageTotal');
const zoomSlider = $('zoom');
const zoomLabel = $('zoomLabel');
const docTitleEl = $('docTitle');
const saveBtn = $('saveBtn');

const notePopup = $('notePopup');
const notePopupInput = $('notePopupInput');

// ---------------------------------------------------------------
// Coordinate helpers (PDF point space <-> canvas pixel space)
// ---------------------------------------------------------------
function scale() { return dpi / 72; }

function ptToPx(x, y) {
  const s = scale();
  return { x: x * s, y: y * s };
}

function pxToPt(x, y) {
  const s = scale();
  return { x: x / s, y: y / s };
}

// Convert a pointer event (screen coords) to canvas-pixel coords,
// accounting for any CSS scaling of the canvas element.
function eventToCanvasPx(evt) {
  const rect = annotCanvas.getBoundingClientRect();
  const scaleX = annotCanvas.width / rect.width;
  const scaleY = annotCanvas.height / rect.height;
  return {
    x: (evt.clientX - rect.left) * scaleX,
    y: (evt.clientY - rect.top) * scaleY,
  };
}

// ---------------------------------------------------------------
// Open / navigate
// ---------------------------------------------------------------
async function pickAndOpenPdf() {
  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (!selected) return;
    const path = Array.isArray(selected) ? selected[0] : selected;
    await openPdfPath(path);
  } catch (err) {
    console.error('打开文件对话框失败：', err);
    alert('无法打开文件选择对话框：' + err);
  }
}

async function openPdfPath(path) {
  try {
    const info = await invoke('ohpdf_open', { path });
    currentPath = path;
    pageCount = info.page_count;
    pageTotalEl.textContent = String(pageCount);
    docTitleEl.textContent = info.title || path.split(/[\\/]/).pop();

    const stored = await invoke('ohpdf_load_annotations', { path });
    annotationsByPage = stored || {};
    dirty = false;
    saveBtn.classList.remove('dirty');

    emptyState.hidden = true;
    pageWrap.hidden = false;

    await goToPage(0);
  } catch (err) {
    console.error('打开 PDF 失败：', err);
    alert('打开 PDF 失败：' + err);
  }
}

async function goToPage(n) {
  if (!currentPath || n < 0 || n >= pageCount) return;
  currentPage = n;
  pageNumInput.value = String(n + 1);
  await renderCurrentPage();
}

async function renderCurrentPage() {
  if (!currentPath) return;
  try {
    const dataUrl = await invoke('ohpdf_render_page', {
      path: currentPath,
      page: currentPage,
      dpi,
    });
    await drawPageImage(dataUrl);
  } catch (err) {
    console.error('渲染页面失败：', err);
  }
}

function drawPageImage(dataUrl) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      pageCanvas.width = img.width;
      pageCanvas.height = img.height;
      annotCanvas.width = img.width;
      annotCanvas.height = img.height;
      pageCanvas.style.width = img.width + 'px';
      pageCanvas.style.height = img.height + 'px';
      annotCanvas.style.width = img.width + 'px';
      annotCanvas.style.height = img.height + 'px';
      pageCtx.drawImage(img, 0, 0);
      redrawAnnotations();
      resolve();
    };
    img.onerror = reject;
    img.src = dataUrl;
  });
}

// ---------------------------------------------------------------
// Annotation rendering
// ---------------------------------------------------------------
function redrawAnnotations() {
  annotCtx.clearRect(0, 0, annotCanvas.width, annotCanvas.height);
  const list = annotationsByPage[String(currentPage)] || [];
  for (const a of list) drawAnnotation(a);
}

function drawAnnotation(a) {
  if (a.type === 'highlight') {
    const p1 = ptToPx(a.rect.x, a.rect.y);
    const p2 = ptToPx(a.rect.x + a.rect.w, a.rect.y + a.rect.h);
    annotCtx.fillStyle = hexToRgba(a.color, 0.35);
    annotCtx.fillRect(p1.x, p1.y, p2.x - p1.x, p2.y - p1.y);
  } else if (a.type === 'note') {
    const p = ptToPx(a.point.x, a.point.y);
    annotCtx.beginPath();
    annotCtx.arc(p.x, p.y, 9, 0, Math.PI * 2);
    annotCtx.fillStyle = a.color;
    annotCtx.fill();
    annotCtx.strokeStyle = 'rgba(0,0,0,0.25)';
    annotCtx.lineWidth = 1;
    annotCtx.stroke();
    annotCtx.fillStyle = '#24221d';
    annotCtx.font = 'bold 11px system-ui';
    annotCtx.textAlign = 'center';
    annotCtx.textBaseline = 'middle';
    annotCtx.fillText('!', p.x, p.y + 0.5);
  } else if (a.type === 'ink') {
    if (!a.points || a.points.length < 2) return;
    annotCtx.strokeStyle = a.color;
    annotCtx.lineWidth = Math.max(1.5, (a.width || 2) * scale() * 0.5);
    annotCtx.lineJoin = 'round';
    annotCtx.lineCap = 'round';
    annotCtx.beginPath();
    a.points.forEach((pt, i) => {
      const p = ptToPx(pt.x, pt.y);
      if (i === 0) annotCtx.moveTo(p.x, p.y);
      else annotCtx.lineTo(p.x, p.y);
    });
    annotCtx.stroke();
  }
}

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

function rectFrom(a, b) {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(b.x - a.x),
    h: Math.abs(b.y - a.y),
  };
}

function pushAnnotation(a) {
  const key = String(currentPage);
  if (!annotationsByPage[key]) annotationsByPage[key] = [];
  annotationsByPage[key].push(a);
  redrawAnnotations();
  markDirty();
}

function markDirty() {
  dirty = true;
  saveBtn.classList.add('dirty');
}

async function saveAnnotations() {
  if (!currentPath) return;
  try {
    await invoke('ohpdf_save_annotations', { path: currentPath, data: annotationsByPage });
    dirty = false;
    saveBtn.classList.remove('dirty');
  } catch (err) {
    console.error('保存标注失败：', err);
    alert('保存标注失败：' + err);
  }
}

function undoLast() {
  const key = String(currentPage);
  const list = annotationsByPage[key];
  if (list && list.length) {
    list.pop();
    redrawAnnotations();
    markDirty();
  }
}

// ---------------------------------------------------------------
// Note popup (custom, avoids relying on window.prompt in a webview)
// ---------------------------------------------------------------
let notePopupResolver = null;

function showNotePopup(screenX, screenY) {
  notePopup.style.left = screenX + 'px';
  notePopup.style.top = screenY + 'px';
  notePopup.style.display = 'flex';
  notePopupInput.value = '';
  notePopupInput.focus();
  return new Promise((resolve) => { notePopupResolver = resolve; });
}

function hideNotePopup() {
  notePopup.style.display = 'none';
  notePopupResolver = null;
}

$('notePopupOk').addEventListener('click', () => {
  const val = notePopupInput.value.trim();
  if (notePopupResolver) notePopupResolver(val || null);
  hideNotePopup();
});
$('notePopupCancel').addEventListener('click', () => {
  if (notePopupResolver) notePopupResolver(null);
  hideNotePopup();
});
notePopupInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') $('notePopupOk').click();
  if (e.key === 'Escape') $('notePopupCancel').click();
});

async function addNoteAt(pt, screenX, screenY) {
  const text = await showNotePopup(screenX, screenY);
  if (text) pushAnnotation({ type: 'note', color, point: pt, text });
}

// ---------------------------------------------------------------
// Pointer interaction on the annotation canvas
// ---------------------------------------------------------------
annotCanvas.addEventListener('pointerdown', (evt) => {
  if (mode === 'pan' || !currentPath) return;
  const { x, y } = eventToCanvasPx(evt);
  const pt = pxToPt(x, y);

  if (mode === 'highlight') {
    dragStart = pt;
  } else if (mode === 'note') {
    addNoteAt(pt, evt.clientX, evt.clientY);
  } else if (mode === 'ink') {
    inkDrawing = [pt];
  }
});

annotCanvas.addEventListener('pointermove', (evt) => {
  if (mode === 'highlight' && dragStart) {
    const { x, y } = eventToCanvasPx(evt);
    const pt = pxToPt(x, y);
    redrawAnnotations();
    drawAnnotation({ type: 'highlight', color, rect: rectFrom(dragStart, pt) });
  } else if (mode === 'ink' && inkDrawing) {
    const { x, y } = eventToCanvasPx(evt);
    const pt = pxToPt(x, y);
    inkDrawing.push(pt);
    redrawAnnotations();
    drawAnnotation({ type: 'ink', color, width: 2, points: inkDrawing });
  }
});

annotCanvas.addEventListener('pointerup', (evt) => {
  if (mode === 'highlight' && dragStart) {
    const { x, y } = eventToCanvasPx(evt);
    const pt = pxToPt(x, y);
    const rect = rectFrom(dragStart, pt);
    if (rect.w > 2 && rect.h > 2) pushAnnotation({ type: 'highlight', color, rect });
    dragStart = null;
    redrawAnnotations();
  } else if (mode === 'ink' && inkDrawing) {
    if (inkDrawing.length > 1) pushAnnotation({ type: 'ink', color, width: 2, points: inkDrawing });
    inkDrawing = null;
    redrawAnnotations();
  }
});

annotCanvas.addEventListener('pointerleave', () => {
  // cancel an in-progress drag if the pointer leaves the canvas
  if (dragStart || inkDrawing) {
    dragStart = null;
    inkDrawing = null;
    redrawAnnotations();
  }
});

// ---------------------------------------------------------------
// Toolbar wiring
// ---------------------------------------------------------------
document.querySelectorAll('[data-mode]').forEach((btn) => {
  btn.addEventListener('click', () => {
    mode = btn.dataset.mode;
    document.querySelectorAll('[data-mode]').forEach((b) => b.classList.remove('active'));
    btn.classList.add('active');
    stage.classList.remove('mode-pan', 'mode-highlight', 'mode-note', 'mode-ink');
    stage.classList.add('mode-' + mode);
  });
});

$('colorPicker').addEventListener('input', (e) => { color = e.target.value; });

$('openBtn').addEventListener('click', pickAndOpenPdf);
$('emptyOpenBtn').addEventListener('click', pickAndOpenPdf);
saveBtn.addEventListener('click', saveAnnotations);
$('undoBtn').addEventListener('click', undoLast);

$('prevBtn').addEventListener('click', () => goToPage(currentPage - 1));
$('nextBtn').addEventListener('click', () => goToPage(currentPage + 1));

pageNumInput.addEventListener('change', () => {
  const n = parseInt(pageNumInput.value, 10) - 1;
  if (!Number.isNaN(n)) goToPage(n);
  else pageNumInput.value = String(currentPage + 1);
});

zoomSlider.addEventListener('input', async (e) => {
  dpi = Math.round(96 * (Number(e.target.value) / 100));
  zoomLabel.textContent = e.target.value + '%';
  await renderCurrentPage();
});

function updateNavButtons() {
  $('prevBtn').disabled = currentPage <= 0;
  $('nextBtn').disabled = currentPage >= pageCount - 1;
}

// keep nav buttons in sync after every page change
const _goToPage = goToPage;
goToPage = async (n) => { await _goToPage(n); updateNavButtons(); };

// ---------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------
window.addEventListener('keydown', (e) => {
  const tag = (e.target && e.target.tagName) || '';
  if (tag === 'INPUT') return; // don't hijack typing in the page/note inputs

  if (e.key === 'ArrowRight') goToPage(currentPage + 1);
  if (e.key === 'ArrowLeft') goToPage(currentPage - 1);
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); saveAnnotations(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') { e.preventDefault(); undoLast(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') { e.preventDefault(); pickAndOpenPdf(); }
});

// warn before closing with unsaved annotations
window.addEventListener('beforeunload', (e) => {
  if (dirty) { e.preventDefault(); e.returnValue = ''; }
});

// initial mode class
stage.classList.add('mode-pan');
