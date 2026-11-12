import init, { generate, verify_scans } from "./pkg/qrkit_wasm.js";

const TYPE_FIELDS = {
  text:   [{ name: "text", label: "Text", type: "textarea", value: "Hello, vector world" }],
  url:    [{ name: "url", label: "URL", type: "text", value: "example.com" }],
  email:  [{ name: "to", label: "To", type: "email", value: "" },
           { name: "subject", label: "Subject", type: "text", value: "" },
           { name: "body", label: "Body", type: "textarea", value: "" }],
  sms:    [{ name: "number", label: "Number", type: "text", value: "" },
           { name: "message", label: "Message", type: "text", value: "" }],
  phone:  [{ name: "number", label: "Number", type: "text", value: "" }],
  wifi:   [{ name: "ssid", label: "SSID", type: "text", value: "" },
           { name: "password", label: "Password", type: "password", value: "" },
           { name: "encryption", label: "Encryption", type: "select", options: ["wpa", "wep", "nopass"], value: "wpa" },
           { name: "hidden", label: "Hidden network", type: "checkbox", value: false }],
  vcard:  [{ name: "name", label: "Full name", type: "text", value: "" },
           { name: "org", label: "Organization", type: "text", value: "" },
           { name: "title", label: "Title", type: "text", value: "" },
           { name: "phones", label: "Phones (comma-sep)", type: "text", value: "" },
           { name: "emails", label: "Emails (comma-sep)", type: "text", value: "" },
           { name: "url", label: "URL", type: "text", value: "" }],
  geo:    [{ name: "lat", label: "Latitude", type: "number", value: "51.5007" },
           { name: "lon", label: "Longitude", type: "number", value: "-0.1246" }],
  calendar: [{ name: "title", label: "Title", type: "text", value: "" },
             { name: "start_utc", label: "Start (YYYYMMDDTHHMMSSZ)", type: "text", value: "" },
             { name: "end_utc", label: "End (YYYYMMDDTHHMMSSZ)", type: "text", value: "" },
             { name: "location", label: "Location", type: "text", value: "" },
             { name: "description", label: "Description", type: "text", value: "" }],
};

// One-click style presets (fill, gradient, module + eye shapes). Everything
// else (logo, ecc, margin) is left as-is; "Classic" resets to defaults.
const PRESETS = {
  classic: { fg: "#000000", bg: "#ffffff", gradient: "", gradientTo: "#42a5f5", angle: 45,
             shape: "square", dotRadius: 0.5, eye: "square", eyeAuto: true, eyeColor: "#000000" },
  ocean:   { fg: "#0d47a1", bg: "#ffffff", gradient: "linear", gradientTo: "#42a5f5", angle: 45,
             shape: "rounded", dotRadius: 0.5, eye: "circle", eyeAuto: true, eyeColor: "#000000" },
  sunset:  { fg: "#ff6f00", bg: "#ffffff", gradient: "radial", gradientTo: "#f50057", angle: 45,
             shape: "dots", dotRadius: 0.5, eye: "rounded", eyeAuto: true, eyeColor: "#000000" },
  forest:  { fg: "#1b5e20", bg: "#ffffff", gradient: "linear", gradientTo: "#81c784", angle: 90,
             shape: "classy", dotRadius: 0.5, eye: "leaf", eyeAuto: true, eyeColor: "#000000" },
  berry:   { fg: "#880e4f", bg: "#ffffff", gradient: "linear", gradientTo: "#ff5252", angle: 25,
             shape: "dots", dotRadius: 0.5, eye: "circle", eyeAuto: false, eyeColor: "#880e4f" },
};

const $ = (id) => document.getElementById(id);
let logoBase64 = null;
let verifyTimer = null;

function renderTypeSelect() {
  $("type").innerHTML = Object.keys(TYPE_FIELDS)
    .map((t) => `<option value="${t}">${t}</option>`).join("");
}

function renderContentForm(type) {
  const fields = TYPE_FIELDS[type].map((f) => {
    if (f.type === "textarea") {
      return `<label>${f.label}<textarea data-field="${f.name}">${f.value}</textarea></label>`;
    }
    if (f.type === "select") {
      return `<label>${f.label}<select data-field="${f.name}">${
        f.options.map((o) => `<option${o === f.value ? " selected" : ""}>${o}</option>`).join("")
      }</select></label>`;
    }
    if (f.type === "checkbox") {
      return `<label class="check"><input type="checkbox" data-field="${f.name}" ${f.value ? "checked" : ""}/> ${f.label}</label>`;
    }
    const step = f.type === "number" ? ' step="any"' : "";
    return `<label>${f.label}<input type="${f.type}" data-field="${f.name}" value="${f.value}"${step}/></label>`;
  }).join("");
  $("content-form").innerHTML = fields;
  $("content-form").querySelectorAll("[data-field]").forEach((el) => {
    el.addEventListener("input", update);
  });
}

function collectContent() {
  const type = $("type").value;
  const content = { type };
  // Empty strings are only meaningful for the single required field of the
  // text/url types (the library reports "must not be empty"); everywhere else
  // empty means "unset" — including vcard's optional `url` field.
  const primary = { text: "text", url: "url" };
  $("content-form").querySelectorAll("[data-field]").forEach((el) => {
    let v = el.type === "checkbox" ? el.checked : el.value.trim();
    if (el.type === "number") v = parseFloat(v);
    if (typeof v === "string" && v === "" && primary[type] !== el.dataset.field) return;
    content[el.dataset.field] = v;
  });
  if (type === "vcard") {
    content.phones = (content.phones || "").split(",").map((s) => s.trim()).filter(Boolean);
    content.emails = (content.emails || "").split(",").map((s) => s.trim()).filter(Boolean);
  }
  return content;
}

function collectStyle() {
  const style = {
    fg: $("fg").value,
    bg: $("bg-transparent").checked ? "transparent" : $("bg").value,
    gradient: $("gradient").value || undefined,
    gradientTo: $("gradient").value ? $("gradient-to").value : undefined,
    gradientAngle: parseFloat($("gradient-angle").value),
    shape: $("shape").value,
    dotRadius: parseFloat($("dot-radius").value),
    eye: $("eye").value,
    // "auto" leaves eyeColor unset so the eyes follow fg / the gradient.
    eyeColor: $("eye-auto").checked ? undefined : $("eye-color").value,
    decorativeEyes: $("eye-decorative").checked || undefined,
    ecc: $("ecc").value,
    margin: parseInt($("margin").value, 10),
    // PNG resolution (ignored by the vector formats).
    px: parseInt($("px-per-module").value, 10) || 24,
  };
  if (logoBase64) {
    style.logo = { dataBase64: logoBase64, sizeRatio: parseFloat($("logo-size").value) };
  }
  return style;
}

function showError(e) {
  $("warnings").innerHTML = `<p class="error">${e}</p>`;
}

function update() {
  const config = { content: collectContent(), style: collectStyle() };
  let out;
  try {
    out = generate(JSON.stringify(config), "svg");
  } catch (e) {
    showError(e);
    $("scan-badge").className = "";
    $("scan-badge").textContent = "";
    $("info-badge").textContent = "";
    scheduleVerify(null);
    return;
  }
  $("qr").innerHTML = out.svg || "";
  $("info-badge").textContent = out.info || "";
  $("warnings").innerHTML = (out.warnings || []).map((w) => `<p class="warn">${w}</p>`).join("");
  syncPermalink(config);
  scheduleVerify(config);
}

// ---- live scannability badge -------------------------------------------
function scheduleVerify(config) {
  clearTimeout(verifyTimer);
  if (!config) return;
  $("scan-badge").className = "pending";
  $("scan-badge").textContent = "checking…";
  verifyTimer = setTimeout(() => {
    let ok = false;
    try {
      ok = verify_scans(JSON.stringify(config));
    } catch {
      ok = false;
    }
    $("scan-badge").className = ok ? "ok" : "bad";
    $("scan-badge").textContent = ok ? "✓ scans cleanly" : "✗ does not decode";
  }, 350);
}

// ---- permalinks ----------------------------------------------------------
// Content + style live in the URL hash (logo bytes excluded — too large).
function syncPermalink(config) {
  const shareable = { ...config, style: { ...config.style } };
  delete shareable.style.logo;
  const encoded = encodeURIComponent(JSON.stringify(shareable));
  // Some engines (WebKit) rate-limit replaceState and throw; a failed hash
  // sync must never break rendering.
  try {
    history.replaceState(null, "", `#${encoded}`);
  } catch {
    /* hash stays stale until the next accepted call */
  }
}

function applyConfig(cfg) {
  if (!cfg || !cfg.content || !TYPE_FIELDS[cfg.content.type]) return false;
  $("type").value = cfg.content.type;
  renderContentForm(cfg.content.type);
  $("content-form").querySelectorAll("[data-field]").forEach((el) => {
    const v = cfg.content[el.dataset.field];
    if (v === undefined) return;
    if (el.type === "checkbox") el.checked = !!v;
    else el.value = Array.isArray(v) ? v.join(",") : v;
  });
  const s = cfg.style || {};
  if (s.fg) $("fg").value = s.fg;
  if (s.bg === "transparent") { $("bg-transparent").checked = true; }
  else if (s.bg) { $("bg-transparent").checked = false; $("bg").value = s.bg; }
  $("gradient").value = s.gradient || "";
  if (s.gradientTo) $("gradient-to").value = s.gradientTo;
  if (s.gradientAngle !== undefined) $("gradient-angle").value = s.gradientAngle;
  if (s.shape) $("shape").value = s.shape;
  if (s.dotRadius !== undefined) $("dot-radius").value = s.dotRadius;
  if (s.eye) $("eye").value = s.eye;
  const auto = s.eyeColor === undefined || s.eyeColor === null;
  $("eye-auto").checked = auto;
  $("eye-color").disabled = auto;
  if (s.eyeColor) $("eye-color").value = s.eyeColor;
  $("eye-decorative").checked = !!s.decorativeEyes;
  if (s.margin !== undefined) $("margin").value = s.margin;
  if (s.ecc) $("ecc").value = s.ecc;
  if (s.px !== undefined) $("px-per-module").value = s.px;
  return true;
}

// ---- downloads & clipboard ----------------------------------------------
function slug(pieces) {
  const s = pieces
    .filter(Boolean).join("-")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  return s || "qr";
}

function smartBase() {
  const content = collectContent();
  const primary = content.text || content.url || content.ssid || content.name ||
    content.title || content.number || content.to || content.lat || "qr";
  return slug([content.type, String(primary)]);
}

function generateBytes(fmt) {
  const config = { content: collectContent(), style: collectStyle() };
  let out;
  try {
    out = generate(JSON.stringify(config), fmt);
  } catch (e) {
    showError(e);
    return null;
  }
  if (fmt === "svg") return new TextEncoder().encode(out.svg);
  return new Uint8Array(out[fmt]);
}

function download(fmt) {
  const bytes = generateBytes(fmt);
  if (!bytes) return;
  const blob = new Blob([bytes], { type: {
    svg: "image/svg+xml",
    pdf: "application/pdf",
    eps: "application/postscript",
    png: "image/png",
  }[fmt] });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `${smartBase()}.${fmt}`;
  a.click();
  URL.revokeObjectURL(a.href);
}

function flash(btn, text) {
  const old = btn.textContent;
  btn.textContent = text;
  setTimeout(() => { btn.textContent = old; }, 1200);
}

async function copySvg(btn) {
  const bytes = generateBytes("svg");
  if (!bytes) return;
  try {
    await navigator.clipboard.writeText(new TextDecoder().decode(bytes));
    flash(btn, "Copied ✓");
  } catch {
    flash(btn, "Copy blocked — downloading");
    download("svg");
  }
}

async function copyPng(btn) {
  const bytes = generateBytes("png");
  if (!bytes) return;
  try {
    const blob = new Blob([bytes], { type: "image/png" });
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    flash(btn, "Copied ✓");
  } catch {
    // Firefox and some contexts don't support image clipboard writes.
    flash(btn, "Not supported — downloading");
    download("png");
  }
}

async function copyLink(btn) {
  try {
    await navigator.clipboard.writeText(location.href);
    flash(btn, "Link copied ✓");
  } catch {
    flash(btn, "Copy blocked");
  }
}

// ---- presets --------------------------------------------------------------
function applyPreset(name) {
  const p = PRESETS[name];
  if (!p) return;
  $("fg").value = p.fg;
  $("bg").value = p.bg;
  $("bg-transparent").checked = false;
  $("gradient").value = p.gradient;
  $("gradient-to").value = p.gradientTo;
  $("gradient-angle").value = p.angle;
  $("shape").value = p.shape;
  $("dot-radius").value = p.dotRadius;
  $("eye").value = p.eye;
  $("eye-auto").checked = p.eyeAuto;
  $("eye-color").disabled = p.eyeAuto;
  $("eye-color").value = p.eyeColor;
  $("eye-decorative").checked = false;
  update();
}

// ---- logo ------------------------------------------------------------------
function loadLogoFile(file) {
  if (!file) { logoBase64 = null; syncLogoClear(); update(); return; }
  const r = new FileReader();
  r.onerror = () => showError("could not read logo file");
  r.onload = () => {
    logoBase64 = r.result.split(",")[1];
    syncLogoClear(); // only meaningful once the data has actually landed
    update();
  };
  r.readAsDataURL(file);
}

function syncLogoClear() {
  $("logo-clear").hidden = !logoBase64;
}

async function main() {
  await init();
  renderTypeSelect();
  renderContentForm("text");

  $("type").addEventListener("change", (e) => {
    renderContentForm(e.target.value);
    update();
  });
  // Content-form fields get their listeners from renderContentForm (it
  // re-attaches on every type switch); the type select is handled above and
  // the logo input below, so both are skipped here to avoid duplicate or
  // stale-form update() calls.
  document.querySelectorAll("#panel input, #panel select, #panel textarea")
    .forEach((el) => {
      if (el.id === "logo" || el.id === "type" || el.dataset.field) return;
      el.addEventListener("input", update);
    });
  // "auto" eye color: toggling it enables/disables the color picker; the
  // generic loop above already wired the checkbox itself to update().
  $("eye-auto").addEventListener("input", () => {
    $("eye-color").disabled = $("eye-auto").checked;
  });
  $("logo").addEventListener("change", (e) => {
    loadLogoFile(e.target.files[0]);
    syncLogoClear();
  });
  // Remove an uploaded logo: drop the data, reset the file input, re-render.
  $("logo-clear").addEventListener("click", () => {
    logoBase64 = null;
    $("logo").value = "";
    syncLogoClear();
    update();
  });

  // Drag-and-drop a logo anywhere on the page.
  window.addEventListener("dragover", (e) => e.preventDefault());
  window.addEventListener("drop", (e) => {
    e.preventDefault();
    const file = [...(e.dataTransfer?.files || [])].find((f) => f.type.startsWith("image/"));
    if (file) {
      loadLogoFile(file);
      syncLogoClear();
    }
  });

  // Presets.
  const presetsRow = $("presets");
  if (presetsRow) {
    Object.keys(PRESETS).forEach((name) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "preset";
      b.textContent = name;
      b.addEventListener("click", () => applyPreset(name));
      presetsRow.appendChild(b);
    });
  }

  // Downloads / clipboard act on their dropdown selections.
  $("dl-btn").addEventListener("click", () => download($("dl-format").value));
  $("copy-btn").addEventListener("click", (e) => {
    switch ($("copy-format").value) {
      case "svg": return copySvg(e.currentTarget);
      case "png": return copyPng(e.currentTarget);
      case "link": return copyLink(e.currentTarget);
    }
  });

  // Restore from a permalink (#...), if present. applyConfig only sets DOM
  // values (no input events), so update() must ALWAYS run afterwards — the
  // DOM state is the config.
  if (location.hash.length > 1) {
    try {
      applyConfig(JSON.parse(decodeURIComponent(location.hash.slice(1))));
    } catch {
      // malformed hash — fall through to defaults
    }
  }
  update();
}

main();
