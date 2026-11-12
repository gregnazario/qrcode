import init, { generate } from "./pkg/qrkit_wasm.js";

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

const $ = (id) => document.getElementById(id);
let logoBase64 = null;

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
    ecc: $("ecc").value,
    margin: parseInt($("margin").value, 10),
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
    return;
  }
  $("qr").innerHTML = out.svg || "";
  $("warnings").innerHTML = (out.warnings || []).map((w) => `<p class="warn">${w}</p>`).join("");
}

function download(fmt) {
  const config = { content: collectContent(), style: collectStyle() };
  let out;
  try {
    out = generate(JSON.stringify(config), fmt);
  } catch (e) {
    showError(e);
    return;
  }
  const bytes = fmt === "svg"
    ? new TextEncoder().encode(out.svg)
    : new Uint8Array(out[fmt]);
  const blob = new Blob([bytes], { type: { svg: "image/svg+xml", pdf: "application/pdf", eps: "application/postscript" }[fmt] });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `qr.${fmt}`;
  a.click();
  URL.revokeObjectURL(a.href);
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
  $("logo").addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) { logoBase64 = null; update(); return; }
    const dataUrl = await new Promise((res) => {
      const r = new FileReader();
      r.onload = () => res(r.result);
      r.readAsDataURL(file);
    });
    logoBase64 = dataUrl.split(",")[1];
    update();
  });
  document.querySelectorAll("#downloads button").forEach((b) =>
    b.addEventListener("click", () => download(b.dataset.fmt)));
  update();
}

main();
