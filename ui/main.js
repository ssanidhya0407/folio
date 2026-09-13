// folio — workspace UI. File dialogs + PDF work happen in Rust.
const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const fileListEl = document.getElementById("file-list");
const viewerEl = document.getElementById("viewer");
const toolsBody = document.getElementById("tools-body");
const toastEl = document.getElementById("toast");
const dragOverlay = document.getElementById("drag-overlay");
const searchEl = document.getElementById("tool-search");

const VIEW_W = 820; // px width pages are rendered at
const VIEW_MAX = 40; // pages rendered for preview

// ---------- icons ----------
const I = {
  merge: '<path d="M12 3l9 5-9 5-9-5 9-5z"/><path d="M3 13l9 5 9-5"/>',
  split: '<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><line x1="20" y1="4" x2="8.12" y2="15.88"/><line x1="14.47" y1="14.48" x2="20" y2="20"/><line x1="8.12" y1="8.12" x2="12" y2="12"/>',
  burst: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
  rotate: '<polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>',
  compress: '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/>',
  image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/>',
  topng: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><circle cx="9.5" cy="13.5" r="1.5"/><path d="M19 19l-4-4-7 6"/>',
  shield: '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/>',
  file: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  alert: '<circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>',
  back: '<path d="M19 12H5"/><path d="M12 19l-7-7 7-7"/>',
  minus: '<line x1="5" y1="12" x2="19" y2="12"/>',
  plus: '<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>',
  prev: '<polyline points="15 18 9 12 15 6"/>',
  next: '<polyline points="9 18 15 12 9 6"/>',
  up: '<polyline points="18 15 12 9 6 15"/>',
  down: '<polyline points="6 9 12 15 18 9"/>',
  x: '<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>',
  lock: '<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  unlock: '<rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>',
  edit: '<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5z"/>',
  organize: '<rect x="3" y="4" width="10" height="13" rx="1.5"/><path d="M16 7h4a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H10a1 1 0 0 1-1-1v-2"/>',
  ocr: '<path d="M4 8V5a1 1 0 0 1 1-1h3"/><path d="M16 4h3a1 1 0 0 1 1 1v3"/><path d="M20 16v3a1 1 0 0 1-1 1h-3"/><path d="M8 20H5a1 1 0 0 1-1-1v-3"/><line x1="8" y1="10" x2="16" y2="10"/><line x1="8" y1="14" x2="13" y2="14"/>',
  redact: '<rect x="3" y="9" width="18" height="6" rx="1" fill="currentColor" stroke="none"/><line x1="3" y1="5" x2="14" y2="5"/><line x1="3" y1="19" x2="11" y2="19"/>',
  batch: '<path d="M12 2 2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>',
  info: '<circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>',
};
function esc(v) { return (v || "").replace(/"/g, "&quot;"); }
const svg = (k, cls = "") =>
  `<svg class="${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${I[k]}</svg>`;

// ---------- tools ----------
const TOOLS = {
  merge: { cat: "Organize", title: "Merge", sub: "Combine & reorder PDFs", icon: "merge", cls: "ic-blue",
    input: "allpdfs", custom: "merge", options: [] },
  split: { cat: "Organize", title: "Split", sub: "Extract, burst, or chunk", icon: "split", cls: "ic-blue",
    input: "active", custom: "split", options: [] },
  organize: { cat: "Organize", title: "Organize pages", sub: "Reorder & delete pages", icon: "organize", cls: "ic-blue",
    input: "active", custom: "organize", options: [] },
  rotate: { cat: "Organize", title: "Rotate", sub: "Turn pages", icon: "rotate", cls: "ic-blue",
    input: "active", output: "savePdf", name: "rotated.pdf",
    options: [
      { type: "select", id: "degrees", label: "Rotation", choices: [["90", "90° clockwise"], ["180", "180°"], ["270", "90° counter-clockwise"]] },
      { type: "text", id: "range", label: "Pages (leave blank for all)", placeholder: "e.g. 1-3, 5" },
    ],
    cmd: "run_rotate", args: (f, o, opt) => ({ input: f[0], output: o, degrees: parseInt(opt.degrees, 10), range: (opt.range || "").trim() || null }) },
  compress: { cat: "Optimize", title: "Compress", sub: "Shrink the file size", icon: "compress", cls: "ic-green",
    input: "active", output: "savePdf", name: "compressed.pdf",
    options: [{ type: "select", id: "level", label: "Strength", choices: [["balanced", "Balanced"], ["light", "Light (best quality)"], ["strong", "Strong (smallest)"]] }],
    cmd: "run_compress", args: (f, o, opt) => ({ input: f[0], output: o, level: opt.level || "balanced" }) },
  img2pdf: { cat: "Convert", title: "Images → PDF", sub: "Reorder & set page size", icon: "image", cls: "ic-orange",
    input: "images", custom: "img2pdf", options: [] },
  topng: { cat: "Convert", title: "PDF → images", sub: "Export pages as images", icon: "topng", cls: "ic-orange",
    input: "active", output: "folder",
    options: [
      { type: "select", id: "dpi", label: "Resolution", choices: [["150", "150 DPI — good"], ["72", "72 DPI — small"], ["300", "300 DPI — high quality"]] },
      { type: "select", id: "format", label: "Format", choices: [["png", "PNG (lossless)"], ["jpeg", "JPEG (smaller)"]] },
      { type: "text", id: "pages", label: "Pages (leave blank for all)", placeholder: "e.g. 1-3, 5" },
    ],
    cmd: "run_topng", args: (f, o, opt) => ({ input: f[0], outdir: o, dpi: parseFloat(opt.dpi || "150"), format: opt.format || "png", range: (opt.pages || "").trim() || null }) },
  watermark: { cat: "Edit", title: "Watermark", sub: "Stamp text on pages", icon: "edit", cls: "ic-orange",
    input: "active", custom: "watermark", options: [] },
  encrypt: { cat: "Security", title: "Protect", sub: "Password & permissions", icon: "lock", cls: "ic-blue",
    input: "active", custom: "encrypt", options: [] },
  decrypt: { cat: "Security", title: "Remove password", sub: "Unlock a protected PDF", icon: "unlock", cls: "ic-blue",
    input: "active", custom: "decrypt", options: [] },
  meta: { cat: "Privacy", title: "Metadata", sub: "View, edit, or remove", icon: "shield", cls: "ic-green",
    input: "active", custom: "meta", options: [] },
  redact: { cat: "Privacy", title: "Redact", sub: "Delete content for good", icon: "redact", cls: "ic-green",
    input: "active", custom: "redact", options: [] },
  ocr: { cat: "Convert", title: "Make searchable", sub: "Read text in a scan (OCR)", icon: "ocr", cls: "ic-orange",
    input: "active", custom: "ocr", options: [] },
  batch: { cat: "Batch", title: "Batch process", sub: "One job, every open file", icon: "batch", cls: "ic-blue",
    input: "allpdfs", custom: "batch", options: [] },
};
const CATS = ["Organize", "Edit", "Optimize", "Convert", "Security", "Privacy", "Batch"];

// ---------- state ----------
const S = {
  files: [], active: -1, pages: null, loading: false, zoom: 600,
  tool: null, opt: {}, images: [], busy: false, result: null, search: "",
  orgFor: null, orgThumbs: null, order: null,
  ocrLangs: null, ocrReady: false, ocrHasText: null, ocrFor: null,
  redFor: null, redPages: null, redPage: 0, regions: [],
  batchReport: null, progress: null,
};
const base = (p) => p.split(/[\\/]/).pop();
const isPdf = (p) => /\.pdf$/i.test(p);
const isImg = (p) => /\.(png|jpe?g|bmp|gif|tiff?|webp)$/i.test(p);

function toast(msg, kind) {
  toastEl.textContent = msg;
  toastEl.className = "toast show" + (kind ? " " + kind : "");
  setTimeout(() => (toastEl.className = "toast"), 3800);
}

// ============ files (left) ============
function renderFiles() {
  if (!S.files.length) {
    fileListEl.innerHTML = `<div class="file-empty">No files yet.<br/>Open or drop a PDF to begin.</div>`;
    return;
  }
  fileListEl.innerHTML = S.files
    .map((f, i) => `<div class="file-item ${i === S.active ? "active" : ""}" data-file="${i}">
      <span class="fic">${svg("file")}</span>
      <span class="finfo"><span class="fn">${f.name}</span><span class="fm">${f.pages != null ? f.pages + " pages" : "PDF"}</span></span>
      <button class="fx" data-remove="${i}" title="Remove">✕</button>
    </div>`)
    .join("");
  fileListEl.querySelectorAll("[data-file]").forEach((el) =>
    el.addEventListener("click", (e) => { if (!e.target.closest("[data-remove]")) setActive(+el.dataset.file); })
  );
  fileListEl.querySelectorAll("[data-remove]").forEach((b) =>
    b.addEventListener("click", () => removeFile(+b.dataset.remove))
  );
}

async function addPdfs(paths) {
  const fresh = paths.filter(isPdf).filter((p) => !S.files.some((f) => f.path === p));
  if (!fresh.length) return;
  const start = S.files.length;
  fresh.forEach((p) => S.files.push({ path: p, name: base(p), pages: null }));
  renderFiles();
  setActive(start);
  if (S.tool) renderToolPanel(); // refresh "not ready" → ready prompts
  for (let i = start; i < S.files.length; i++) {
    invoke("page_count", { path: S.files[i].path }).then((n) => { S.files[i].pages = n; renderFiles(); }).catch(() => {});
  }
}

function removeFile(i) {
  S.files.splice(i, 1);
  if (S.active >= S.files.length) S.active = S.files.length - 1;
  renderFiles();
  if (S.active >= 0) setActive(S.active);
  else { S.pages = null; renderViewer(); }
  if (S.tool) renderTools();
}

// ============ viewer (center) ============
async function setActive(i) {
  S.active = i;
  S.pages = null;
  S.loading = true;
  renderFiles();
  renderViewer();
  const path = S.files[i].path;
  try {
    S.pages = await invoke("render_view", { path, maxPages: VIEW_MAX, width: VIEW_W });
  } catch (e) {
    S.pages = [];
    toast("Could not render preview.", "err");
  }
  S.loading = false;
  if (S.active === i) renderViewer();
}

function renderViewer() {
  if (S.active < 0 || !S.files[S.active]) {
    viewerEl.innerHTML = `<div class="v-empty">
      <svg class="art" viewBox="0 0 120 120" fill="none">
        <rect x="30" y="20" width="60" height="80" rx="5" fill="#fff" stroke="#e6e7eb" stroke-width="2"/>
        <path d="M74 20 L90 36 H74 Z" fill="#fff3e2" stroke="#f7941d" stroke-width="2" stroke-linejoin="round"/>
        <line x1="40" y1="52" x2="80" y2="52" stroke="#e2e5ea" stroke-width="3" stroke-linecap="round"/>
        <line x1="40" y1="62" x2="80" y2="62" stroke="#e2e5ea" stroke-width="3" stroke-linecap="round"/>
        <line x1="40" y1="72" x2="66" y2="72" stroke="#e2e5ea" stroke-width="3" stroke-linecap="round"/>
        <text x="60" y="93" text-anchor="middle" font-size="10" font-weight="800" fill="#f7941d" font-family="Arial">.pdf</text>
      </svg>
      <h2>Drop a PDF to get started</h2>
      <p>Everything happens on this computer — nothing is uploaded.</p>
      <button class="open-btn" id="v-open">${svg("file")} Open from computer</button>
    </div>`;
    document.getElementById("v-open")?.addEventListener("click", openPdfs);
    return;
  }

  const f = S.files[S.active];
  const bar = `<div class="v-bar">
    <span class="v-name">${f.name}</span>
    <div class="v-controls">
      <button class="iconbtn" id="pg-prev" title="Previous page">${svg("prev")}</button>
      <span class="pagi" id="pagi">— / —</span>
      <button class="iconbtn" id="pg-next" title="Next page">${svg("next")}</button>
      <span class="v-div"></span>
      <button class="iconbtn" id="z-out" title="Zoom out">${svg("minus")}</button>
      <span class="zoom-val" id="z-val">${Math.round((S.zoom / VIEW_W) * 100)}%</span>
      <button class="iconbtn" id="z-in" title="Zoom in">${svg("plus")}</button>
    </div>
  </div>`;

  let body;
  if (S.loading) {
    body = `<div class="v-loading"><div class="spinner"></div><div>Rendering preview…</div></div>`;
  } else if (!S.pages || !S.pages.length) {
    body = `<div class="v-loading"><div>Preview unavailable.</div></div>`;
  } else {
    const pages = S.pages
      .map((src, i) => `<div class="page-wrap" data-page="${i}" style="width:${S.zoom}px">
        <img src="${src}" alt="page ${i + 1}" loading="lazy"/><span class="page-num">${i + 1}</span></div>`)
      .join("");
    const more = (f.pages || 0) > S.pages.length ? `<div class="file-empty">Preview limited to first ${S.pages.length} of ${f.pages} pages — tools still act on the whole file.</div>` : "";
    body = `<div class="pages" id="pages">${pages}${more}</div>`;
  }
  viewerEl.innerHTML = bar + body;
  wireViewer();
}

function wireViewer() {
  const total = S.pages ? S.pages.length : 0;
  const pagesEl = document.getElementById("pages");
  const pagi = document.getElementById("pagi");
  const setZoom = (z) => { S.zoom = Math.max(360, Math.min(VIEW_W, z)); renderViewer(); };
  document.getElementById("z-in")?.addEventListener("click", () => setZoom(S.zoom + 80));
  document.getElementById("z-out")?.addEventListener("click", () => setZoom(S.zoom - 80));

  const scrollToPage = (idx) => {
    const el = pagesEl?.querySelector(`[data-page="${idx}"]`);
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
  };
  let current = 0;
  const updatePagi = () => { if (pagi) pagi.textContent = `${current + 1} / ${total}`; };
  document.getElementById("pg-prev")?.addEventListener("click", () => scrollToPage(Math.max(0, current - 1)));
  document.getElementById("pg-next")?.addEventListener("click", () => scrollToPage(Math.min(total - 1, current + 1)));
  if (pagesEl && total) {
    updatePagi();
    pagesEl.addEventListener("scroll", () => {
      const wraps = pagesEl.querySelectorAll(".page-wrap");
      const top = pagesEl.scrollTop;
      let nearest = 0, best = Infinity;
      wraps.forEach((w, i) => { const d = Math.abs(w.offsetTop - top - 22); if (d < best) { best = d; nearest = i; } });
      if (nearest !== current) { current = nearest; updatePagi(); }
    });
  }
}

// ============ tools (right) ============
function renderTools() {
  if (S.tool) { renderToolPanel(); return; }
  const q = S.search.trim().toLowerCase();
  let html = "";
  for (const cat of CATS) {
    const ids = Object.keys(TOOLS).filter(
      (id) => TOOLS[id].cat === cat && (!q || (TOOLS[id].title + " " + TOOLS[id].sub).toLowerCase().includes(q))
    );
    if (!ids.length) continue;
    html += `<div class="tool-cat">${cat}</div>`;
    html += ids.map((id) => {
      const t = TOOLS[id];
      return `<button class="tool-row" data-tool="${id}">
        <span class="ic ${t.cls}">${svg(t.icon)}</span>
        <span class="tx"><span class="tt">${t.title}</span><span class="ts">${t.sub}</span></span>
        <span class="chev">${svg("next")}</span>
      </button>`;
    }).join("");
  }
  if (!html) html = `<div class="file-empty">No tools match “${S.search}”.</div>`;
  toolsBody.innerHTML = html;
  toolsBody.querySelectorAll("[data-tool]").forEach((b) => b.addEventListener("click", () => openTool(b.dataset.tool)));
}

function openTool(id) {
  S.tool = id;
  S.result = null;
  S.busy = false;
  S.images = [];
  S.opt = {};
  S.metaFor = null;
  S.orgFor = null;
  S.redFor = null;
  S.regions = [];
  S.batchReport = null;
  S.progress = null;
  S.ocrHasText = null;
  S.ocrFor = null;
  for (const o of (TOOLS[id].options || [])) S.opt[o.id] = o.type === "select" ? o.choices[0][0] : "";
  renderToolPanel();
}

function renderToolPanel() {
  const t = TOOLS[S.tool];
  const head = `<button class="tp-back" id="tp-back">${svg("back")} Back to tools</button>
    <div class="tp-title">
      <span class="ic ${t.cls}">${svg(t.icon)}</span>
      <div class="tp-titletext"><h3>${t.title}</h3><p>${t.sub}</p></div>
    </div>`;

  if (S.busy) { toolsBody.innerHTML = head + busyHTML(); wireBack(); return; }
  if (S.result) { toolsBody.innerHTML = head + resultHTML(); wireBack(); wireResult(); return; }
  if (t.custom === "merge") return renderMerge(head);
  if (t.custom === "split") return renderSplit(head);
  if (t.custom === "organize") return renderOrganize(head);
  if (t.custom === "img2pdf") return renderImg2pdf(head);
  if (t.custom === "watermark") return renderWatermark(head);
  if (t.custom === "encrypt") return renderEncrypt(head);
  if (t.custom === "decrypt") return renderDecrypt(head);
  if (t.custom === "meta") return renderMeta(head);
  if (t.custom === "ocr") return renderOcr(head);
  if (t.custom === "redact") return renderRedact(head);
  if (t.custom === "batch") return renderBatch(head);

  // input readiness + area
  let ready = false;
  let inputArea = "";
  if (t.input === "active") {
    const f = S.files[S.active];
    ready = !!f;
    inputArea = ready
      ? targetCard(t.icon, `Acts on <b>${f.name}</b>`)
      : promptCard("upload", "Open a PDF to use this tool.", "Open PDF", "cta-open");
  } else if (t.input === "allpdfs") {
    ready = S.files.length >= 2;
    inputArea = ready
      ? targetCard("merge", `Merges <b>${S.files.length} PDFs</b> in the order shown`)
      : promptCard("merge", `Merge needs at least 2 PDFs${S.files.length ? ` — you have ${S.files.length}` : ""}.`, "Add PDFs", "cta-open");
  } else { // images
    ready = S.images.length > 0;
    inputArea = ready
      ? targetCard("image", `<b>${S.images.length}</b> image(s) selected`) +
        `<button class="btn-soft" id="cta-images" style="width:100%;margin:-4px 0 14px">Change images</button>`
      : promptCard("image", "Choose images to turn into a PDF.", "Choose images", "cta-images");
  }

  const opts = ready
    ? t.options.map((o) => {
        if (o.type === "text")
          return `<div class="field"><label>${o.label}</label><input type="text" data-opt="${o.id}" value="${S.opt[o.id] || ""}" placeholder="${o.placeholder || ""}"/></div>`;
        const ch = o.choices.map(([v, l]) => `<option value="${v}" ${S.opt[o.id] === v ? "selected" : ""}>${l}</option>`).join("");
        return `<div class="field"><label>${o.label}</label><select data-opt="${o.id}">${ch}</select></div>`;
      }).join("")
    : "";

  const runBtn = ready ? `<button class="run-btn" id="run">Run</button>` : "";

  toolsBody.innerHTML = head + inputArea + opts + runBtn;
  wireBack();
  document.getElementById("cta-open")?.addEventListener("click", openPdfs);
  document.getElementById("cta-images")?.addEventListener("click", pickImages);
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) => inp.addEventListener("input", () => (S.opt[inp.dataset.opt] = inp.value)));
  document.getElementById("run")?.addEventListener("click", runTool);
}

function wireBack() { document.getElementById("tp-back")?.addEventListener("click", () => { S.tool = null; renderTools(); }); }

function targetCard(icon, html) {
  return `<div class="tp-target">${svg(icon)}<span>${html}</span></div>`;
}

// ----- Merge: reorderable list -----
function renderMerge(head) {
  if (S.files.length < 2) {
    toolsBody.innerHTML = head + promptCard("merge", `Merge needs at least 2 PDFs${S.files.length ? ` — you have ${S.files.length}` : ""}.`, "Add PDFs", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  const rows = S.files.map((f, i) => `<div class="ord-row">
      <span class="ord-n">${i + 1}</span>
      <span class="ord-name" title="${f.name}">${f.name}</span>
      <span class="ord-act">
        <button class="iconbtn sm" data-up="${i}" ${i === 0 ? "disabled" : ""} title="Move up">${svg("up")}</button>
        <button class="iconbtn sm" data-down="${i}" ${i === S.files.length - 1 ? "disabled" : ""} title="Move down">${svg("down")}</button>
        <button class="iconbtn sm" data-del="${i}" title="Remove">${svg("x")}</button>
      </span>
    </div>`).join("");
  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("merge")}<span>Merges top → bottom. Reorder below.</span></div>
     <div class="ord-list">${rows}</div>
     <button class="btn-soft" id="cta-open" style="width:100%;margin:8px 0 14px">+ Add PDFs</button>
     <button class="run-btn" id="run">Merge ${S.files.length} PDFs</button>`;
  wireBack();
  document.getElementById("cta-open")?.addEventListener("click", openPdfs);
  toolsBody.querySelectorAll("[data-up]").forEach((b) => b.addEventListener("click", () => moveFile(+b.dataset.up, -1)));
  toolsBody.querySelectorAll("[data-down]").forEach((b) => b.addEventListener("click", () => moveFile(+b.dataset.down, 1)));
  toolsBody.querySelectorAll("[data-del]").forEach((b) => b.addEventListener("click", () => removeFile(+b.dataset.del)));
  document.getElementById("run")?.addEventListener("click", runMerge);
}
function moveFile(i, dir) {
  const j = i + dir;
  if (j < 0 || j >= S.files.length) return;
  [S.files[i], S.files[j]] = [S.files[j], S.files[i]];
  if (S.active === i) S.active = j; else if (S.active === j) S.active = i;
  renderFiles();
  renderToolPanel();
}
async function runMerge() {
  if (S.files.length < 2) return;
  const output = await invoke("save_pdf", { defaultName: "merged.pdf" });
  if (!output) return;
  await doRun("run_merge", { inputs: S.files.map((f) => f.path), output }, output, "file");
}

// ----- Split: modes -----
function renderSplit(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("upload", "Open a PDF to split.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  const mode = S.opt.mode || "extract";
  const modeSel = `<div class="field"><label>How to split</label>
    <select data-mode>
      <option value="extract" ${mode === "extract" ? "selected" : ""}>Extract a page range → one PDF</option>
      <option value="each" ${mode === "each" ? "selected" : ""}>Each page → separate PDFs</option>
      <option value="every" ${mode === "every" ? "selected" : ""}>Every N pages → multiple PDFs</option>
      <option value="at" ${mode === "at" ? "selected" : ""}>Split at pages → multiple PDFs</option>
    </select></div>`;
  let extra = "";
  if (mode === "extract") extra = field("range", "Pages to extract", S.opt.range, "e.g. 1-3, 5, 8-10");
  else if (mode === "every") extra = field("n", "Pages per file", S.opt.n, "e.g. 2");
  else if (mode === "at") extra = field("at", "Split before pages", S.opt.at, "e.g. 4, 8");
  else extra = `<div class="tp-note">Creates one PDF for every page (you’ll choose a folder).</div>`;

  toolsBody.innerHTML = head +
    targetCard("split", `Acts on <b>${f.name}</b>${f.pages != null ? ` · ${f.pages} pages` : ""}`) +
    modeSel + extra + `<button class="run-btn" id="run">Run</button>`;
  wireBack();
  toolsBody.querySelector("[data-mode]")?.addEventListener("change", (e) => { S.opt.mode = e.target.value; renderToolPanel(); });
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) => inp.addEventListener("input", () => (S.opt[inp.dataset.opt] = inp.value)));
  document.getElementById("run")?.addEventListener("click", runSplit);
}
function field(id, label, val, ph) {
  return `<div class="field"><label>${label}</label><input type="text" data-opt="${id}" value="${val || ""}" placeholder="${ph}"/></div>`;
}
async function runSplit() {
  const f = S.files[S.active];
  if (!f) return;
  const mode = S.opt.mode || "extract";
  if (mode === "extract") {
    const range = (S.opt.range || "").trim();
    if (!range) return toast("Enter a page range.", "err");
    const out = await invoke("save_pdf", { defaultName: "pages.pdf" });
    if (!out) return;
    return doRun("run_split", { input: f.path, output: out, range }, out, "file");
  }
  const dir = await invoke("choose_folder");
  if (!dir) return;
  if (mode === "each") return doRun("run_burst", { input: f.path, outdir: dir }, dir, "folder");
  if (mode === "every") {
    const n = parseInt(S.opt.n, 10);
    if (!n || n < 1) return toast("Enter how many pages per file.", "err");
    return doRun("run_split_every", { input: f.path, outdir: dir, n }, dir, "folder");
  }
  if (mode === "at") {
    const points = (S.opt.at || "").split(/[\s,]+/).map((x) => parseInt(x, 10)).filter((x) => x > 0);
    if (!points.length) return toast("Enter the page numbers to split before.", "err");
    return doRun("run_split_at", { input: f.path, outdir: dir, points }, dir, "folder");
  }
}

// ----- Organize pages: thumbnail grid, drag to reorder, ✕ to delete -----
function renderOrganize(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("upload", "Open a PDF to reorganize its pages.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }

  // (Re)load page thumbnails whenever the active file changes.
  if (S.orgFor !== f.path) {
    S.orgFor = f.path;
    S.orgThumbs = null;
    S.order = null;
    toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Loading pages…</div>`;
    wireBack();
    const cap = Math.max(1, f.pages || 300);
    invoke("render_view", { path: f.path, maxPages: cap, width: 240 })
      .then((thumbs) => {
        if (S.tool !== "organize" || S.orgFor !== f.path) return;
        S.orgThumbs = thumbs;
        S.order = thumbs.map((_, i) => i + 1); // 1-based original page numbers
        renderToolPanel();
      })
      .catch(() => { if (S.tool === "organize" && S.orgFor === f.path) { S.orgThumbs = []; renderToolPanel(); } });
    return;
  }

  if (!S.orgThumbs) { toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Loading pages…</div>`; wireBack(); return; }
  if (!S.orgThumbs.length) {
    toolsBody.innerHTML = head + promptCard("alert", "Couldn’t render this PDF’s pages.", "Back to tools", "cta-back2");
    wireBack();
    document.getElementById("cta-back2")?.addEventListener("click", () => { S.tool = null; renderTools(); });
    return;
  }

  const total = S.orgThumbs.length;
  const changed = S.order.length !== total || S.order.some((p, i) => p !== i + 1);
  const cards = S.order.map((p, i) => `
    <div class="pg-card" data-pos="${i}">
      <button class="pg-del" data-del="${i}" title="Delete page ${p}">${svg("x")}</button>
      <div class="pg-thumb"><img src="${S.orgThumbs[p - 1]}" alt="page ${p}" draggable="false"/></div>
      <div class="pg-foot">page ${p}</div>
    </div>`).join("");

  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("organize")}<span>Drag to reorder · ✕ to delete · <b>${S.order.length}</b> of ${total} kept</span></div>
     <div class="pg-grid">${cards}</div>
     ${changed ? `<button class="btn-soft" id="org-reset" style="width:100%;margin:10px 0 0">Reset</button>` : ""}
     <button class="run-btn" id="run" style="margin-top:12px" ${S.order.length ? "" : "disabled"}>Save new PDF</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-del]").forEach((b) =>
    b.addEventListener("click", () => { S.order.splice(+b.dataset.del, 1); renderToolPanel(); }));
  document.getElementById("org-reset")?.addEventListener("click", () => { S.order = S.orgThumbs.map((_, i) => i + 1); renderToolPanel(); });
  document.getElementById("run")?.addEventListener("click", runOrganize);
  wireOrgDnd();
}
// Pointer-based drag reorder. We can't use HTML5 drag-and-drop here: the Tauri
// webview's OS file-drop handler (needed for "drop a PDF to load") swallows
// in-page drag events, so element dragging never fires. Pointer events aren't
// intercepted, so we implement the drag ourselves.
function wireOrgDnd() {
  const grid = toolsBody.querySelector(".pg-grid");
  if (!grid) return;
  const cards = [...grid.querySelectorAll(".pg-card")];
  let from = -1, startX = 0, startY = 0, active = false, ghost = null;

  const nearestInsert = (x, y) => {
    // Insertion index in [0, cards.length], by nearest card centre.
    let best = -1, bestD = Infinity, after = false;
    cards.forEach((c, i) => {
      const r = c.getBoundingClientRect();
      const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
      const d = (x - cx) ** 2 + (y - cy) ** 2;
      if (d < bestD) { bestD = d; best = i; after = (y > cy + 4) || (Math.abs(y - cy) <= r.height / 2 && x > cx); }
    });
    if (best < 0) return cards.length;
    return after ? best + 1 : best;
  };

  const highlight = (ins) => {
    cards.forEach((c, i) => c.classList.toggle("drop-into", i === ins || (ins === cards.length && i === cards.length - 1)));
  };

  const cleanup = () => {
    if (ghost) { ghost.remove(); ghost = null; }
    cards.forEach((c) => c.classList.remove("drop-into", "drag-src"));
    document.body.classList.remove("pg-dragging");
    document.removeEventListener("pointermove", onMove);
    document.removeEventListener("pointerup", onUp);
    document.removeEventListener("pointercancel", onUp);
    active = false; from = -1;
  };

  const onMove = (e) => {
    if (from < 0) return;
    if (!active) {
      if (Math.abs(e.clientX - startX) < 5 && Math.abs(e.clientY - startY) < 5) return;
      active = true;
      const src = cards[from];
      src.classList.add("drag-src");
      document.body.classList.add("pg-dragging");
      ghost = src.cloneNode(true);
      ghost.classList.add("pg-ghost");
      ghost.style.width = src.offsetWidth + "px";
      document.body.appendChild(ghost);
    }
    ghost.style.left = (e.clientX - ghost.offsetWidth / 2) + "px";
    ghost.style.top = (e.clientY - ghost.offsetHeight / 2) + "px";
    highlight(nearestInsert(e.clientX, e.clientY));
  };

  const onUp = (e) => {
    if (active && from >= 0) {
      let to = nearestInsert(e.clientX, e.clientY);
      const [moved] = S.order.splice(from, 1);
      if (to > from) to--;
      to = Math.max(0, Math.min(S.order.length, to));
      S.order.splice(to, 0, moved);
      cleanup();
      renderToolPanel();
      return;
    }
    cleanup();
  };

  cards.forEach((card, i) => {
    card.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 || e.target.closest(".pg-del")) return;
      e.preventDefault();
      from = i; startX = e.clientX; startY = e.clientY;
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
      document.addEventListener("pointercancel", onUp);
    });
  });
}
async function runOrganize() {
  const f = S.files[S.active];
  if (!f || !S.order || !S.order.length) return;
  const out = await invoke("save_pdf", { defaultName: "organized.pdf" });
  if (!out) return;
  S.orgFor = null; // force a fresh reload if the tool is reopened
  await doRun("run_reorder", { input: f.path, output: out, order: S.order }, out, "file");
}

// ----- Images → PDF: reorder + page size -----
function renderImg2pdf(head) {
  if (!S.images.length) {
    toolsBody.innerHTML = head + promptCard("image", "Choose images to turn into a PDF.", "Choose images", "cta-images");
    wireBack();
    document.getElementById("cta-images")?.addEventListener("click", pickImages);
    return;
  }
  S.opt.size = S.opt.size || "fit";
  const rows = S.images.map((p, i) => `<div class="ord-row">
      <span class="ord-n">${i + 1}</span>
      <span class="ord-name" title="${p}">${base(p)}</span>
      <span class="ord-act">
        <button class="iconbtn sm" data-up="${i}" ${i === 0 ? "disabled" : ""}>${svg("up")}</button>
        <button class="iconbtn sm" data-down="${i}" ${i === S.images.length - 1 ? "disabled" : ""}>${svg("down")}</button>
        <button class="iconbtn sm" data-del="${i}">${svg("x")}</button>
      </span></div>`).join("");
  const sizeSel = `<div class="field"><label>Page size</label>
    <select data-opt="size">
      <option value="fit" ${S.opt.size === "fit" ? "selected" : ""}>Fit to image</option>
      <option value="a4" ${S.opt.size === "a4" ? "selected" : ""}>A4</option>
      <option value="letter" ${S.opt.size === "letter" ? "selected" : ""}>Letter</option>
    </select></div>`;
  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("image")}<span>${S.images.length} image(s) · pages in this order</span></div>
     <div class="ord-list">${rows}</div>
     <button class="btn-soft" id="cta-add" style="width:100%;margin:8px 0 14px">+ Add more images</button>
     ${sizeSel}
     <button class="run-btn" id="run">Create PDF</button>`;
  wireBack();
  document.getElementById("cta-add")?.addEventListener("click", pickImagesAdd);
  toolsBody.querySelectorAll("[data-up]").forEach((b) => b.addEventListener("click", () => moveImg(+b.dataset.up, -1)));
  toolsBody.querySelectorAll("[data-down]").forEach((b) => b.addEventListener("click", () => moveImg(+b.dataset.down, 1)));
  toolsBody.querySelectorAll("[data-del]").forEach((b) => b.addEventListener("click", () => { S.images.splice(+b.dataset.del, 1); renderToolPanel(); }));
  toolsBody.querySelector('[data-opt="size"]')?.addEventListener("change", (e) => (S.opt.size = e.target.value));
  document.getElementById("run")?.addEventListener("click", runImg2pdf);
}
function moveImg(i, dir) {
  const j = i + dir;
  if (j < 0 || j >= S.images.length) return;
  [S.images[i], S.images[j]] = [S.images[j], S.images[i]];
  renderToolPanel();
}
async function pickImagesAdd() {
  const imgs = await invoke("browse_images");
  if (imgs && imgs.length) { S.images = [...S.images, ...imgs]; renderToolPanel(); }
}
async function runImg2pdf() {
  if (!S.images.length) return;
  const out = await invoke("save_pdf", { defaultName: "images.pdf" });
  if (!out) return;
  await doRun("run_img2pdf", { inputs: S.images, output: out, pageSize: S.opt.size || "fit" }, out, "file");
}

// ----- Metadata: view / edit / remove -----
function renderMeta(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("upload", "Open a PDF to view its metadata.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  if (S.metaFor !== f.path) {
    toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Reading metadata…</div>`;
    wireBack();
    invoke("get_metadata", { path: f.path })
      .then((m) => { S.meta = m || {}; S.metaFor = f.path; if (S.tool === "meta") renderToolPanel(); })
      .catch(() => { S.meta = {}; S.metaFor = f.path; if (S.tool === "meta") renderToolPanel(); });
    return;
  }
  const m = S.meta || {};
  const esc = (v) => (v || "").replace(/"/g, "&quot;");
  const fld = (id, label) => `<div class="field"><label>${label}</label><input type="text" data-meta="${id}" value="${esc(m[id])}"/></div>`;
  const info = (label, val) => (val ? `<div class="meta-info"><span>${label}</span><b title="${esc(val)}">${val}</b></div>` : "");
  const extra = [info("Creator", m.creator), info("Producer", m.producer), info("Created", m.creationDate), info("Modified", m.modDate)].join("");
  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("shield")}<span>Editing <b>${f.name}</b></span></div>` +
    fld("title", "Title") + fld("author", "Author") + fld("subject", "Subject") + fld("keywords", "Keywords") +
    (extra ? `<div class="meta-extra">${extra}</div>` : "") +
    `<button class="run-btn" id="meta-save">Save changes</button>
     <button class="btn-soft" id="meta-clear" style="width:100%;margin-top:8px">Remove all metadata</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-meta]").forEach((inp) => inp.addEventListener("input", () => (S.meta[inp.dataset.meta] = inp.value)));
  document.getElementById("meta-save")?.addEventListener("click", runMetaSave);
  document.getElementById("meta-clear")?.addEventListener("click", runMetaClear);
}
async function runMetaSave() {
  const f = S.files[S.active];
  if (!f) return;
  const out = await invoke("save_pdf", { defaultName: "metadata.pdf" });
  if (!out) return;
  S.metaFor = null;
  await doRun("run_set_metadata", { input: f.path, output: out, meta: S.meta || {} }, out, "file");
}
async function runMetaClear() {
  const f = S.files[S.active];
  if (!f) return;
  const out = await invoke("save_pdf", { defaultName: "clean.pdf" });
  if (!out) return;
  S.metaFor = null;
  await doRun("run_scrub", { input: f.path, output: out }, out, "file");
}

// ----- Watermark -----
function renderWatermark(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("edit", "Open a PDF to watermark it.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  const o = S.opt;
  o.style = o.style || "diagonal";
  o.opacity = o.opacity || "0.3";
  o.color = o.color || "#808080";
  o.size = o.size || "48";
  toolsBody.innerHTML = head +
    targetCard("edit", `Stamp <b>${f.name}</b>`) +
    `<div class="field"><label>Watermark text</label><input type="text" data-opt="text" value="${esc(o.text)}" placeholder="e.g. CONFIDENTIAL"/></div>
     <div class="field"><label>Style</label><select data-opt="style">
        <option value="diagonal" ${o.style === "diagonal" ? "selected" : ""}>Diagonal</option>
        <option value="horizontal" ${o.style === "horizontal" ? "selected" : ""}>Horizontal</option>
        <option value="tiled" ${o.style === "tiled" ? "selected" : ""}>Tiled (repeat)</option>
     </select></div>
     <div class="field-row">
       <div class="field"><label>Size</label><input type="number" data-opt="size" value="${o.size}" min="8" max="200"/></div>
       <div class="field"><label>Opacity</label><select data-opt="opacity">
          <option value="0.15" ${o.opacity === "0.15" ? "selected" : ""}>Light</option>
          <option value="0.3" ${o.opacity === "0.3" ? "selected" : ""}>Medium</option>
          <option value="0.5" ${o.opacity === "0.5" ? "selected" : ""}>Strong</option>
       </select></div>
       <div class="field field-color"><label>Color</label><input type="color" data-opt="color" value="${o.color}"/></div>
     </div>
     <div class="field"><label>Pages (leave blank for all)</label><input type="text" data-opt="pages" value="${esc(o.pages)}" placeholder="e.g. 1-3, 5"/></div>
     <button class="run-btn" id="run">Add watermark</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) => inp.addEventListener("input", () => (o[inp.dataset.opt] = inp.value)));
  document.getElementById("run")?.addEventListener("click", runWatermark);
}
function hexToRgb(hex) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex || "");
  if (!m) return [0.5, 0.5, 0.5];
  return [parseInt(m[1], 16) / 255, parseInt(m[2], 16) / 255, parseInt(m[3], 16) / 255];
}
async function runWatermark() {
  const f = S.files[S.active];
  if (!f) return;
  const o = S.opt;
  if (!(o.text || "").trim()) return toast("Enter watermark text.", "err");
  const out = await invoke("save_pdf", { defaultName: "watermarked.pdf" });
  if (!out) return;
  const style = o.style || "diagonal";
  const [r, g, b] = hexToRgb(o.color || "#808080");
  const opts = {
    text: o.text, fontSize: parseFloat(o.size || "48"), opacity: parseFloat(o.opacity || "0.3"),
    colorR: r, colorG: g, colorB: b,
    rotation: style === "horizontal" ? 0 : 45, tiled: style === "tiled",
    pages: (o.pages || "").trim() || null,
  };
  await doRun("run_watermark", { input: f.path, output: out, opts }, out, "file", true);
}

// ----- Protect (encrypt) -----
function renderEncrypt(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("lock", "Open a PDF to protect it.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  const o = S.opt;
  const type = o.show ? "text" : "password";
  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("lock")}<span>Protect <b>${f.name}</b></span></div>
     <div class="field"><label>Password to open</label>
       <div class="pw-row"><input type="${type}" data-opt="pw" value="${esc(o.pw)}" placeholder="Enter a password"/>
       <button class="eye" data-eye type="button">${o.show ? "Hide" : "Show"}</button></div></div>
     <div class="field"><label>Confirm password</label>
       <input type="${type}" data-opt="pw2" value="${esc(o.pw2)}" placeholder="Re-enter password"/></div>
     <div class="field"><label>Encryption</label>
       <select data-opt="bits">
         <option value="256" ${(o.bits || "256") === "256" ? "selected" : ""}>256-bit AES (recommended)</option>
         <option value="128" ${o.bits === "128" ? "selected" : ""}>128-bit AES</option>
       </select></div>
     <button class="adv-toggle" data-adv type="button">${svg(o.adv ? "down" : "next")} Restrict permissions</button>
     ${o.adv ? `<div class="adv-box">
        <div class="field"><label>Owner password (to change permissions)</label>
          <input type="${type}" data-opt="owner" value="${esc(o.owner)}" placeholder="Optional but recommended"/></div>
        <label class="chk"><input type="checkbox" data-chk="print" ${o.print !== false ? "checked" : ""}/> Allow printing</label>
        <label class="chk"><input type="checkbox" data-chk="copy" ${o.copy !== false ? "checked" : ""}/> Allow copying text</label>
        <label class="chk"><input type="checkbox" data-chk="modify" ${o.modify !== false ? "checked" : ""}/> Allow editing</label>
        <div class="adv-hint">For restrictions to hold, use an owner password different from the open password.</div>
       </div>` : ""}
     <button class="run-btn" id="run" style="margin-top:14px">Protect PDF</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) => inp.addEventListener("input", () => (o[inp.dataset.opt] = inp.value)));
  toolsBody.querySelectorAll("[data-chk]").forEach((c) => c.addEventListener("change", () => (o[c.dataset.chk] = c.checked)));
  toolsBody.querySelector("[data-eye]")?.addEventListener("click", () => { o.show = !o.show; renderToolPanel(); });
  toolsBody.querySelector("[data-adv]")?.addEventListener("click", () => { o.adv = !o.adv; renderToolPanel(); });
  document.getElementById("run")?.addEventListener("click", runEncrypt);
}
async function runEncrypt() {
  const f = S.files[S.active];
  if (!f) return;
  const o = S.opt;
  const pw = o.pw || "", pw2 = o.pw2 || "", owner = o.owner || "";
  if (pw && pw !== pw2) return toast("Passwords don’t match.", "err");
  if (!pw && !owner) return toast("Enter a password.", "err");
  const out = await invoke("save_pdf", { defaultName: "protected.pdf" });
  if (!out) return;
  const opts = {
    userPassword: pw, ownerPassword: owner, bits: parseInt(o.bits || "256", 10),
    allowPrint: o.print !== false, allowCopy: o.copy !== false, allowModify: o.modify !== false,
  };
  await doRun("run_encrypt", { input: f.path, output: out, opts }, out, "file", false);
}

// ----- Remove password (decrypt) -----
function renderDecrypt(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("unlock", "Open a protected PDF to unlock it.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }
  const o = S.opt;
  const type = o.show ? "text" : "password";
  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("unlock")}<span>Unlock <b>${f.name}</b></span></div>
     <div class="field"><label>Current password</label>
       <div class="pw-row"><input type="${type}" data-opt="pw" value="${esc(o.pw)}" placeholder="The PDF’s password"/>
       <button class="eye" data-eye type="button">${o.show ? "Hide" : "Show"}</button></div></div>
     <div class="tp-note">Creates an unprotected copy. Only works if you know the password.</div>
     <button class="run-btn" id="run">Remove password</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) => inp.addEventListener("input", () => (o[inp.dataset.opt] = inp.value)));
  toolsBody.querySelector("[data-eye]")?.addEventListener("click", () => { o.show = !o.show; renderToolPanel(); });
  document.getElementById("run")?.addEventListener("click", runDecrypt);
}
async function runDecrypt() {
  const f = S.files[S.active];
  if (!f) return;
  const out = await invoke("save_pdf", { defaultName: "unlocked.pdf" });
  if (!out) return;
  await doRun("run_decrypt", { input: f.path, output: out, password: S.opt.pw || "" }, out, "file", true);
}

// shared run for custom tools
async function doRun(cmd, args, output, kind, addView = true) {
  S.busy = true;
  S.progress = null;
  renderToolPanel();
  try {
    const msg = await invoke(cmd, args);
    S.result = { ok: true, msg, output, outputKind: kind };
    if (addView && kind === "file" && isPdf(output)) addToWorkspace(output);
  } catch (e) {
    S.result = { ok: false, msg: String(e) };
  }
  S.busy = false;
  S.progress = null;
  renderToolPanel();
}
function addToWorkspace(path) {
  const idx = S.files.findIndex((f) => f.path === path);
  if (idx >= 0) setActive(idx);
  else addPdfs([path]);
}
function promptCard(icon, msg, label, id) {
  return `<div class="tp-prompt"><div class="tp-prompt-ic">${svg(icon)}</div><p>${msg}</p><button class="run-btn" id="${id}">${label}</button></div>`;
}

function resultHTML() {
  const r = S.result;
  let actions;
  if (!r.ok) actions = `<button class="btn-soft" data-again>Try again</button>`;
  else if (r.outputKind === "file")
    actions = `<button class="run-btn" data-openview>Open in viewer</button>
       <button class="btn-soft" data-reveal>Show in folder</button>
       <button class="btn-soft" data-again>Do another</button>`;
  else
    actions = `<button class="run-btn" data-openfolder>Open folder</button>
       <button class="btn-soft" data-again>Do another</button>`;
  return `<div class="tp-result ${r.ok ? "ok" : "err"}">
    <div class="badge">${svg(r.ok ? "check" : "alert")}</div>
    <h3>${r.ok ? "Done" : "Couldn’t finish"}</h3><p>${r.msg}</p>
    <div class="ra">${actions}</div></div>`;
}
function wireResult() {
  const r = S.result;
  toolsBody.querySelector("[data-openview]")?.addEventListener("click", () => {
    addToWorkspace(r.output);   // open inside our app, not the OS viewer
    S.tool = null;
    renderTools();
  });
  toolsBody.querySelector("[data-openfolder]")?.addEventListener("click", () => invoke("open_path", { path: r.output }));
  toolsBody.querySelector("[data-reveal]")?.addEventListener("click", () => invoke("reveal", { path: r.output }));
  toolsBody.querySelector("[data-again]")?.addEventListener("click", () => { S.result = null; renderToolPanel(); });
}

async function pickImages() {
  const imgs = await invoke("browse_images");
  if (imgs && imgs.length) { S.images = imgs; renderToolPanel(); }
}

async function runTool() {
  const t = TOOLS[S.tool];
  let inputs;
  if (t.input === "active") { const f = S.files[S.active]; if (!f) return; inputs = [f.path]; }
  else if (t.input === "allpdfs") { if (S.files.length < 2) return; inputs = S.files.map((f) => f.path); }
  else { if (!S.images.length) return; inputs = S.images; }

  for (const o of t.options)
    if (o.required && !(S.opt[o.id] || "").trim()) { toast(`Please fill in “${o.label}”.`, "err"); return; }

  let output;
  if (t.output === "savePdf") output = await invoke("save_pdf", { defaultName: t.name });
  else output = await invoke("choose_folder");
  if (!output) return;

  S.busy = true;
  renderToolPanel();
  try {
    const msg = await invoke(t.cmd, t.args(inputs, output, S.opt));
    S.result = { ok: true, msg, output, outputKind: t.output === "folder" ? "folder" : "file" };
    // if we just produced a PDF, offer it in the workspace
    if (t.output === "savePdf" && isPdf(output)) addPdfs([output]);
  } catch (e) {
    S.result = { ok: false, msg: String(e) };
  }
  S.busy = false;
  renderToolPanel();
}

// ----- Make searchable (OCR) -----
function renderOcr(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("ocr", "Open a scanned PDF to make its text searchable.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }

  // The engine and its language files ship with the app; check once per panel.
  if (S.ocrLangs === null) {
    toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Checking the OCR engine…</div>`;
    wireBack();
    Promise.all([invoke("ocr_languages"), invoke("ocr_available")])
      .then(([langs, ok]) => { S.ocrLangs = ok ? langs : []; S.ocrReady = ok; if (S.tool === "ocr") renderToolPanel(); })
      .catch(() => { S.ocrLangs = []; S.ocrReady = false; if (S.tool === "ocr") renderToolPanel(); });
    return;
  }
  if (!S.ocrReady || !S.ocrLangs.length) {
    toolsBody.innerHTML = head + `<div class="tp-prompt"><div class="tp-prompt-ic">${svg("alert")}</div>
      <p>The OCR engine isn’t installed next to folio.<br/>
      Everything else still works — see the README for how to add it.</p></div>`;
    wireBack();
    return;
  }

  // Warn before turning good vector text into pictures of text. Re-checked
  // whenever the active file changes, not just the first time.
  if (S.ocrFor !== f.path) {
    S.ocrFor = f.path;
    S.ocrHasText = null;
    invoke("pdf_has_text", { path: f.path })
      .then((has) => { S.ocrHasText = has; if (S.tool === "ocr") renderToolPanel(); })
      .catch(() => { S.ocrHasText = false; if (S.tool === "ocr") renderToolPanel(); });
  }

  const o = S.opt;
  const langOpts = S.ocrLangs
    .map(([code, name]) => `<option value="${code}" ${(o.lang || "eng") === code ? "selected" : ""}>${name}</option>`)
    .join("");

  const warn = S.ocrHasText
    ? `<div class="warn-box">${svg("alert")}<span>This PDF already has selectable text. OCR replaces every page
         with an image, so the file gets bigger and the original text quality is lost. Only continue if the text
         you need is missing.</span></div>
       <label class="chk"><input type="checkbox" data-chk="force" ${o.force ? "checked" : ""}/> OCR anyway</label>`
    : "";

  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("ocr")}<span>Recognize the text in <b>${f.name}</b></span></div>
     ${warn}
     <div class="field"><label>Language</label><select data-opt="lang">${langOpts}</select></div>
     <div class="field"><label>Quality</label>
       <select data-opt="dpi">
         <option value="300" ${(o.dpi || "300") === "300" ? "selected" : ""}>Best (300 DPI)</option>
         <option value="200" ${o.dpi === "200" ? "selected" : ""}>Faster (200 DPI)</option>
         <option value="400" ${o.dpi === "400" ? "selected" : ""}>Maximum (400 DPI — slow)</option>
       </select></div>
     <div class="adv-hint">Recognition runs entirely on this computer. Long documents can take a few minutes.</div>
     <button class="run-btn" id="run" style="margin-top:14px" ${S.ocrHasText && !o.force ? "disabled" : ""}>Make searchable</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) =>
    inp.addEventListener("change", () => { o[inp.dataset.opt] = inp.value; }));
  toolsBody.querySelectorAll("[data-chk]").forEach((c) =>
    c.addEventListener("change", () => { o[c.dataset.chk] = c.checked; renderToolPanel(); }));
  document.getElementById("run")?.addEventListener("click", runOcr);
}

async function runOcr() {
  const f = S.files[S.active];
  if (!f) return;
  const o = S.opt;
  const out = await invoke("save_pdf", { defaultName: "searchable.pdf" });
  if (!out) return;
  await doRun(
    "run_ocr",
    { input: f.path, output: out, lang: o.lang || "eng", dpi: parseFloat(o.dpi || "300"), force: !!o.force },
    out, "file", true
  );
}

// Colours offered for the covering box. The choice is cosmetic — whatever you
// pick, the content underneath is deleted from the file.
const REDACT_COLORS = [
  ["#ffffff", "White"],
  ["#000000", "Black"],
  ["#2b2be0", "Blue"],
  ["#6b7280", "Grey"],
];

// ----- Redact: draw boxes, permanently remove what is underneath -----
function renderRedact(head) {
  const f = S.files[S.active];
  if (!f) {
    toolsBody.innerHTML = head + promptCard("redact", "Open a PDF to black out parts of it.", "Open PDF", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }

  if (S.redFor !== f.path) {
    S.redFor = f.path;
    S.redPages = null;
    S.redPage = 0;
    S.regions = [];
    toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Loading pages…</div>`;
    wireBack();
    const cap = Math.max(1, f.pages || 300);
    invoke("render_view", { path: f.path, maxPages: cap, width: 700 })
      .then((pages) => { if (S.tool === "redact" && S.redFor === f.path) { S.redPages = pages; renderToolPanel(); } })
      .catch(() => { if (S.tool === "redact" && S.redFor === f.path) { S.redPages = []; renderToolPanel(); } });
    return;
  }

  if (!S.redPages) { toolsBody.innerHTML = head + `<div class="busy"><div class="spinner"></div>Loading pages…</div>`; wireBack(); return; }
  if (!S.redPages.length) {
    toolsBody.innerHTML = head + promptCard("alert", "Couldn’t render this PDF’s pages.", "Back to tools", "cta-back2");
    wireBack();
    document.getElementById("cta-back2")?.addEventListener("click", () => { S.tool = null; renderTools(); });
    return;
  }

  const total = S.redPages.length;
  const page = Math.min(S.redPage, total - 1);
  const color = S.opt.color || REDACT_COLORS[0][0];
  const here = S.regions.filter((r) => r.page === page + 1);
  const boxes = here
    .map((r) => `<div class="red-box" style="left:${r.x * 100}%;top:${r.y * 100}%;width:${r.w * 100}%;height:${r.h * 100}%;background:${color}">
        <button class="red-x" data-drop="${S.regions.indexOf(r)}" title="Remove this box">${svg("x")}</button></div>`)
    .join("");

  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("redact")}<span>Drag across anything you want <b>gone</b> — it is deleted from the file, not covered.</span></div>
     <div class="red-nav">
       <button class="iconbtn" id="red-prev" ${page === 0 ? "disabled" : ""}>${svg("prev")}</button>
       <span class="pagi">Page ${page + 1} / ${total}</span>
       <button class="iconbtn" id="red-next" ${page >= total - 1 ? "disabled" : ""}>${svg("next")}</button>
     </div>
     <div class="red-stage" id="red-stage">
       <img src="${S.redPages[page]}" alt="page ${page + 1}" draggable="false"/>
       <div class="red-overlay" id="red-overlay">${boxes}</div>
     </div>
     <div class="field" style="margin-top:12px"><label>Box colour</label>
       <div class="swatches">${REDACT_COLORS.map(([hex, name]) =>
         `<button class="swatch ${color === hex ? "on" : ""}" data-color="${hex}" title="${name}"
            style="background:${hex}"></button>`).join("")}
         <input type="color" class="swatch-custom" data-color-custom value="${color}" title="Custom colour"/>
       </div></div>
     <div class="red-count">${S.regions.length ? `<b>${S.regions.length}</b> area(s) marked${here.length ? "" : " (none on this page)"}` : "No areas marked yet."}
       ${S.regions.length ? `<button class="linkbtn" id="red-clear">Clear all</button>` : ""}</div>
     <button class="run-btn" id="run" style="margin-top:12px" ${S.regions.length ? "" : "disabled"}>Redact &amp; save</button>`;
  wireBack();
  document.getElementById("red-prev")?.addEventListener("click", () => { S.redPage = page - 1; renderToolPanel(); });
  document.getElementById("red-next")?.addEventListener("click", () => { S.redPage = page + 1; renderToolPanel(); });
  document.getElementById("red-clear")?.addEventListener("click", () => { S.regions = []; renderToolPanel(); });
  toolsBody.querySelectorAll("[data-color]").forEach((b) =>
    b.addEventListener("click", () => { S.opt.color = b.dataset.color; renderToolPanel(); }));
  toolsBody.querySelector("[data-color-custom]")?.addEventListener("change", (e) => {
    S.opt.color = e.target.value; renderToolPanel();
  });
  toolsBody.querySelectorAll("[data-drop]").forEach((b) =>
    b.addEventListener("click", (e) => { e.stopPropagation(); S.regions.splice(+b.dataset.drop, 1); renderToolPanel(); }));
  document.getElementById("run")?.addEventListener("click", runRedact);
  wireRedactDraw(page);
}

// Pointer events, not HTML5 drag: the webview's OS file-drop handler swallows
// in-page drag events (same reason the Organize grid rolls its own).
function wireRedactDraw(page) {
  const overlay = document.getElementById("red-overlay");
  if (!overlay) return;
  let box = null, x0 = 0, y0 = 0;

  const at = (ev) => {
    const r = overlay.getBoundingClientRect();
    return [
      Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width)),
      Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height)),
    ];
  };

  overlay.addEventListener("pointerdown", (ev) => {
    if (ev.target.closest(".red-x")) return;
    ev.preventDefault();
    overlay.setPointerCapture(ev.pointerId);
    [x0, y0] = at(ev);
    box = document.createElement("div");
    box.className = "red-box drawing";
    box.style.borderColor = S.opt.color || REDACT_COLORS[0][0];
    overlay.appendChild(box);
  });

  overlay.addEventListener("pointermove", (ev) => {
    if (!box) return;
    const [x, y] = at(ev);
    box.style.left = Math.min(x0, x) * 100 + "%";
    box.style.top = Math.min(y0, y) * 100 + "%";
    box.style.width = Math.abs(x - x0) * 100 + "%";
    box.style.height = Math.abs(y - y0) * 100 + "%";
  });

  const finish = (ev) => {
    if (!box) return;
    const [x, y] = at(ev);
    box.remove();
    box = null;
    const w = Math.abs(x - x0), h = Math.abs(y - y0);
    // Ignore stray clicks; a real selection covers a visible area.
    if (w < 0.01 || h < 0.005) return;
    S.regions.push({ page: page + 1, x: Math.min(x0, x), y: Math.min(y0, y), w, h });
    renderToolPanel();
  };
  overlay.addEventListener("pointerup", finish);
  overlay.addEventListener("pointercancel", () => { box?.remove(); box = null; });
}

async function runRedact() {
  const f = S.files[S.active];
  if (!f || !S.regions.length) return;
  const out = await invoke("save_pdf", { defaultName: "redacted.pdf" });
  if (!out) return;
  await doRun(
    "run_redact",
    { input: f.path, output: out, regions: S.regions, color: S.opt.color || REDACT_COLORS[0][0] },
    out, "file", true
  );
}

// ----- Batch: one operation across every open PDF -----
const BATCH_OPS = [
  ["compress", "Compress"],
  ["scrub", "Remove metadata"],
  ["rotate", "Rotate"],
  ["watermark", "Watermark"],
  ["protect", "Protect with a password"],
  ["unlock", "Remove password"],
  ["ocr", "Make searchable (OCR)"],
];

function renderBatch(head) {
  if (S.batchReport) return renderBatchReport(head);

  const pdfs = S.files.filter((f) => isPdf(f.path));
  if (pdfs.length < 1) {
    toolsBody.innerHTML = head + promptCard("batch", "Open the PDFs you want to process — batch acts on every open file.", "Add PDFs", "cta-open");
    wireBack();
    document.getElementById("cta-open")?.addEventListener("click", openPdfs);
    return;
  }

  const o = S.opt;
  const op = o.op || "compress";
  const opOpts = BATCH_OPS.map(([v, l]) => `<option value="${v}" ${op === v ? "selected" : ""}>${l}</option>`).join("");

  let extra = "";
  if (op === "compress")
    extra = `<div class="field"><label>Strength</label><select data-opt="level">
      <option value="balanced" ${(o.level || "balanced") === "balanced" ? "selected" : ""}>Balanced</option>
      <option value="light" ${o.level === "light" ? "selected" : ""}>Light (best quality)</option>
      <option value="strong" ${o.level === "strong" ? "selected" : ""}>Strong (smallest)</option></select></div>`;
  else if (op === "rotate")
    extra = `<div class="field"><label>Rotation</label><select data-opt="degrees">
      <option value="90" ${(o.degrees || "90") === "90" ? "selected" : ""}>90° clockwise</option>
      <option value="180" ${o.degrees === "180" ? "selected" : ""}>180°</option>
      <option value="270" ${o.degrees === "270" ? "selected" : ""}>90° counter-clockwise</option></select></div>`;
  else if (op === "watermark")
    extra = `<div class="field"><label>Watermark text</label>
        <input type="text" data-opt="text" value="${esc(o.text)}" placeholder="e.g. CONFIDENTIAL"/></div>
      <label class="chk"><input type="checkbox" data-chk="tiled" ${o.tiled ? "checked" : ""}/> Tile across the page</label>`;
  else if (op === "protect" || op === "unlock")
    extra = `<div class="field"><label>${op === "protect" ? "Password to set" : "Current password"}</label>
        <input type="password" data-opt="password" value="${esc(o.password)}" placeholder="Applied to every file"/></div>`;
  else if (op === "ocr")
    extra = `<div class="adv-hint">Uses English at 300 DPI. Files that already have searchable text are skipped and listed in the report. OCR is slow — expect about a second per page.</div>`;

  toolsBody.innerHTML = head +
    `<div class="tp-target">${svg("batch")}<span>Applies to all <b>${pdfs.length}</b> open PDF(s)</span></div>
     <div class="field"><label>Operation</label><select data-opt="op">${opOpts}</select></div>
     ${extra}
     <div class="adv-hint">Each result is saved into a folder you choose, named after the original. Originals are never changed.</div>
     <button class="run-btn" id="run" style="margin-top:14px">Choose folder &amp; run</button>`;
  wireBack();
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) =>
    inp.addEventListener("change", () => { S.opt[inp.dataset.opt] = inp.value; renderToolPanel(); }));
  toolsBody.querySelectorAll("[data-opt]").forEach((inp) =>
    inp.addEventListener("input", () => { S.opt[inp.dataset.opt] = inp.value; }));
  toolsBody.querySelectorAll("[data-chk]").forEach((c) =>
    c.addEventListener("change", () => { S.opt[c.dataset.chk] = c.checked; }));
  document.getElementById("run")?.addEventListener("click", runBatchOp);
}

function renderBatchReport(head) {
  const r = S.batchReport;
  const rows = r.items
    .map((i) => `<div class="batch-row ${i.ok ? "ok" : "err"}">
        <span class="bic">${svg(i.ok ? "check" : "alert")}</span>
        <span class="bname" title="${esc(i.name)}">${i.name}</span>
        ${i.ok ? "" : `<span class="bwhy">${i.detail}</span>`}
      </div>`)
    .join("");
  toolsBody.innerHTML = head +
    `<div class="tp-result ${r.failed ? "err" : "ok"}">
       <div class="badge">${svg(r.failed ? "alert" : "check")}</div>
       <h3>${r.succeeded} of ${r.succeeded + r.failed} done</h3>
       <p>${r.failed ? `${r.failed} file(s) couldn’t be processed — the rest were saved.` : "Every file was processed."}</p>
     </div>
     <div class="batch-list">${rows}</div>
     <button class="run-btn" id="b-folder" style="margin-top:12px">Open folder</button>
     <button class="btn-soft" id="b-again" style="width:100%;margin-top:8px">Run another batch</button>`;
  wireBack();
  document.getElementById("b-folder")?.addEventListener("click", () => invoke("open_path", { path: r.outdir }));
  document.getElementById("b-again")?.addEventListener("click", () => { S.batchReport = null; renderToolPanel(); });
}

async function runBatchOp() {
  const pdfs = S.files.filter((f) => isPdf(f.path));
  if (!pdfs.length) return;
  const o = S.opt;
  const op = o.op || "compress";
  if (op === "watermark" && !(o.text || "").trim()) return toast("Enter the watermark text.", "err");
  if (op === "protect" && !(o.password || "")) return toast("Enter a password.", "err");

  const outdir = await invoke("choose_folder");
  if (!outdir) return;

  S.busy = true;
  S.progress = { done: 0, total: pdfs.length, label: "Processing" };
  renderToolPanel();
  try {
    S.batchReport = await invoke("run_batch", {
      req: {
        op,
        inputs: pdfs.map((f) => f.path),
        outdir,
        level: o.level || "balanced",
        degrees: parseInt(o.degrees || "90", 10),
        range: null,
        password: o.password || "",
        ownerPassword: "",
        bits: 256,
        text: o.text || "",
        opacity: 0.3,
        tiled: !!o.tiled,
        lang: "eng",
        dpi: 300,
        // Not forced: a file that already has text is reported as skipped in
        // the batch list rather than silently turned into page images.
        force: false,
      },
    });
  } catch (e) {
    S.result = { ok: false, msg: String(e) };
  }
  S.busy = false;
  S.progress = null;
  renderToolPanel();
}

// ----- About: the privacy claim, stated in the app -----
async function showAbout() {
  let info = { version: "—", license: "GPL-3.0-or-later", ocrAvailable: false, ocrLanguages: 0 };
  try { info = await invoke("about"); } catch (e) { /* show the static text anyway */ }

  const el = document.createElement("div");
  el.className = "modal-back";
  el.innerHTML = `<div class="modal" role="dialog" aria-label="About folio">
      <button class="modal-x" data-close>${svg("x")}</button>
      <h2>folio <span class="ver">${info.version}</span></h2>
      <p class="lead">A free, open-source PDF app that runs entirely on this computer.</p>
      <ul class="claims">
        <li>${svg("check")}<span><b>Nothing is uploaded.</b> There is no networking code in this app at all —
          not for updates, not for telemetry. It works with the network cable unplugged.</span></li>
        <li>${svg("check")}<span><b>No account, no tracking, no paywall.</b> Every feature is free for
          everyone, including businesses, forever.</span></li>
        <li>${svg("check")}<span><b>You can verify it.</b> The source is ${info.license}, and the build fails
          if any networking library ever enters the dependency tree.</span></li>
      </ul>
      <div class="about-foot">
        <span>OCR engine: ${info.ocrAvailable ? `ready · ${info.ocrLanguages} language(s)` : "not installed"}</span>
      </div>
    </div>`;
  document.body.appendChild(el);
  const close = () => el.remove();
  el.querySelector("[data-close]").addEventListener("click", close);
  el.addEventListener("click", (e) => { if (e.target === el) close(); });
  document.addEventListener("keydown", function esc(e) {
    if (e.key === "Escape") { close(); document.removeEventListener("keydown", esc); }
  });
}

// ----- shared progress for the long-running tools -----
function busyHTML() {
  const p = S.progress;
  if (!p || !p.total) return `<div class="busy"><div class="spinner"></div>Working…</div>`;
  const pct = Math.round((p.done / p.total) * 100);
  return `<div class="busy"><div class="spinner"></div>
    <div class="prog">
      <div class="prog-label" id="prog-label">${p.label} ${p.done} of ${p.total}</div>
      <div class="prog-track"><div class="prog-fill" id="prog-fill" style="width:${pct}%"></div></div>
    </div></div>`;
}

function updateProgress(payload) {
  if (!payload || !S.busy) return;
  S.progress = payload;
  const fill = document.getElementById("prog-fill");
  const label = document.getElementById("prog-label");
  if (!fill) { renderToolPanel(); return; }
  fill.style.width = Math.round((payload.done / payload.total) * 100) + "%";
  if (label) label.textContent = `${payload.label} ${payload.done} of ${payload.total}`;
}

// ============ open / drag ============
async function openPdfs() {
  const ps = await invoke("browse_pdfs");
  if (ps && ps.length) addPdfs(ps);
}
async function openImagesTool() {
  openTool("img2pdf");
  await pickImages();
}

function handleDrop(paths) {
  if (!paths || !paths.length) return;
  const pdfs = paths.filter(isPdf), imgs = paths.filter(isImg);
  if (S.tool && TOOLS[S.tool].input === "images") {
    if (imgs.length) { S.images = imgs; renderToolPanel(); } else toast("This tool needs images.", "err");
    return;
  }
  if (pdfs.length) addPdfs(pdfs);
  else if (imgs.length) { openTool("img2pdf"); S.images = imgs; renderToolPanel(); }
  else toast("Drop a PDF or images.", "err");
}

// ============ wire global ============
document.getElementById("open-pdf").addEventListener("click", openPdfs);
document.getElementById("open-images").addEventListener("click", openImagesTool);
document.getElementById("home-link").addEventListener("click", () => { S.tool = null; renderTools(); });
searchEl.addEventListener("input", () => { S.search = searchEl.value; if (!S.tool) renderTools(); });

// ============ resizable panes ============
function setupSplit(id, side) {
  const el = document.getElementById(id);
  const ws = document.querySelector(".workspace");
  el.addEventListener("mousedown", (e) => {
    e.preventDefault();
    el.classList.add("dragging");
    document.body.style.userSelect = "none";
    document.body.style.cursor = "col-resize";
    const rect = ws.getBoundingClientRect();
    const move = (ev) => {
      if (side === "left") {
        const w = Math.max(190, Math.min(400, ev.clientX - rect.left));
        ws.style.setProperty("--left", w + "px");
      } else {
        const w = Math.max(250, Math.min(460, rect.right - ev.clientX));
        ws.style.setProperty("--right", w + "px");
      }
    };
    const up = () => {
      el.classList.remove("dragging");
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
  });
}
setupSplit("split-left", "left");
setupSplit("split-right", "right");

listen("ocr-progress", (e) => updateProgress(e.payload));
listen("batch-progress", (e) => updateProgress(e.payload));
document.getElementById("about-link").addEventListener("click", showAbout);

listen("tauri://drag-enter", () => dragOverlay.classList.add("show"));
listen("tauri://drag-leave", () => dragOverlay.classList.remove("show"));
listen("tauri://drag-drop", (e) => {
  dragOverlay.classList.remove("show");
  handleDrop((e.payload && (e.payload.paths || e.payload)) || []);
});

renderFiles();
renderViewer();
renderTools();

// A PDF opened with folio (or dropped on the .exe) arrives as an argument.
invoke("startup_files")
  .then((paths) => { if (paths && paths.length) handleDrop(paths); })
  .catch(() => {});
