type ToolId = "select" | "pin" | "array" | "binding" | "cage" | "bezier" | "tessellate";

interface ToolDefinition {
  id: ToolId;
  label: string;
  eyebrow: string;
  title: string;
  description: string;
  guidance: string;
  status: string;
}

const tools: Record<ToolId, ToolDefinition> = {
  select: {
    id: "select",
    label: "Select",
    eyebrow: "SELECTION",
    title: "Nothing selected",
    description: "Choose a deformation tool. Its properties and workflow guidance will appear here.",
    guidance: "Select a tool from the left rail to see its intended authoring workflow.",
    status: "Select a target after a geometry source is connected."
  },
  pin: {
    id: "pin",
    label: "Pin & Offset",
    eyebrow: "PIN AUTHORING",
    title: "Pin & Offset Drivers",
    description: "Place deformation pins, define neutral positions, and configure explicit offset drivers.",
    guidance: "Planned flow: select a compatible target, add a pin, define its neutral position, configure a driver, then run a non-mutating preview.",
    status: "Pin authoring UI is staged; core adapter is not connected."
  },
  array: {
    id: "array",
    label: "Point Array",
    eyebrow: "POINT ARRAYS",
    title: "Point Arrays",
    description: "Create and inspect parameterized point arrays for repeatable deformation authoring.",
    guidance: "Planned properties include point count, spacing, orientation, coordinate space, and stable point identities.",
    status: "Point-array authoring UI is staged; no array data is being created."
  },
  binding: {
    id: "binding",
    label: "Binding",
    eyebrow: "BINDING",
    title: "Point Binding",
    description: "Associate source points with validated cage features and inspect binding freshness.",
    guidance: "Planned flow: choose source points and a validated cage, run core binding validation, inspect per-point results, then explicitly persist the binding.",
    status: "Binding is unavailable until source and cage contracts are connected."
  },
  cage: {
    id: "cage",
    label: "Cage",
    eyebrow: "CAGE AUTHORING",
    title: "Cages & Topology",
    description: "Author structured lattices and supported irregular cages with topology-aware editing.",
    guidance: "Planned controls include stable control-point selection, topology diagnostics, and separate rest/deformed cage states.",
    status: "Cage editing is staged; no topology is loaded or modified."
  },
  bezier: {
    id: "bezier",
    label: "Bézier",
    eyebrow: "CURVE AUTHORING",
    title: "Bézier Splines",
    description: "Edit authored paths and curve handles without replacing source curves with sampled geometry.",
    guidance: "The editor will preserve canonical path commands and stable anchor/handle identities. Tessellated samples remain derived preview data.",
    status: "Bézier editing is staged; no path data is loaded."
  },
  tessellate: {
    id: "tessellate",
    label: "Tessellate",
    eyebrow: "VECTOR TESSELLATION",
    title: "Vector Tessellation",
    description: "Inspect adaptive curve samples through a non-destructive viewport overlay.",
    guidance: "Planned controls include source-space deviation tolerance, subdivision depth, sample budgets, and capped/budget-exceeded diagnostics.",
    status: "Tessellation preview is staged; no curve is available to sample."
  }
};

const canvas = document.querySelector<HTMLCanvasElement>("#viewport");
const frame = document.querySelector<HTMLElement>("#viewport-frame");
const toolLabel = document.querySelector<HTMLElement>("#active-tool-label");
const inspectorEyebrow = document.querySelector<HTMLElement>("#inspector-eyebrow");
const inspectorTitle = document.querySelector<HTMLElement>("#inspector-title");
const inspectorDescription = document.querySelector<HTMLElement>("#inspector-description");
const propertyTool = document.querySelector<HTMLElement>("#property-tool");
const guidance = document.querySelector<HTMLElement>("#tool-guidance");
const statusMessage = document.querySelector<HTMLElement>("#status-message");
const bottomStatus = document.querySelector<HTMLElement>("#bottom-status");
const zoomLevel = document.querySelector<HTMLOutputElement>("#zoom-level");
const panReadout = document.querySelector<HTMLElement>("#pan-readout");
const coordinateReadout = document.querySelector<HTMLElement>("#coordinate-readout");

if (!canvas || !frame || !toolLabel || !inspectorEyebrow || !inspectorTitle ||
    !inspectorDescription || !propertyTool || !guidance || !statusMessage ||
    !bottomStatus || !zoomLevel || !panReadout || !coordinateReadout) {
  throw new Error("Forma Deform Studio shell is missing a required UI element.");
}

const context = canvas.getContext("2d");
if (!context) throw new Error("The browser does not provide a 2D canvas context.");

let activeTool: ToolId = "select";
let zoom = 1;
let panX = 0;
let panY = 0;
let drag: { pointerId: number; startX: number; startY: number; originX: number; originY: number } | null = null;
let viewportWidth = 0;
let viewportHeight = 0;

function setActiveTool(id: ToolId): void {
  const definition = tools[id];
  activeTool = id;

  document.querySelectorAll<HTMLButtonElement>("[data-tool]").forEach(button => {
    const selected = button.dataset.tool === id;
    button.classList.toggle("active", selected);
    button.setAttribute("aria-pressed", String(selected));
  });

  toolLabel.textContent = definition.label;
  inspectorEyebrow.textContent = definition.eyebrow;
  inspectorTitle.textContent = definition.title;
  inspectorDescription.textContent = definition.description;
  propertyTool.textContent = definition.label;
  guidance.textContent = definition.guidance;
  statusMessage.textContent = definition.status;
  bottomStatus.textContent = definition.status;
}

function updateZoomLabel(): void {
  zoomLevel.value = `${Math.round(zoom * 100)}%`;
  panReadout.textContent = `${Math.round(panX)}, ${Math.round(panY)}`;
}

function drawViewport(): void {
  const bounds = frame.getBoundingClientRect();
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const width = Math.max(1, Math.floor(bounds.width));
  const height = Math.max(1, Math.floor(bounds.height));
  if (width !== viewportWidth || height !== viewportHeight || canvas.width !== Math.floor(width * dpr) || canvas.height !== Math.floor(height * dpr)) {
    viewportWidth = width;
    viewportHeight = height;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
  }

  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  context.clearRect(0, 0, width, height);
  context.fillStyle = "#0d131b";
  context.fillRect(0, 0, width, height);

  const spacing = Math.max(12, 32 * zoom);
  const offsetX = ((width / 2 + panX) % spacing + spacing) % spacing;
  const offsetY = ((height / 2 + panY) % spacing + spacing) % spacing;
  context.beginPath();
  context.strokeStyle = "#1b2734";
  context.lineWidth = 1;
  for (let x = offsetX; x < width; x += spacing) {
    context.moveTo(Math.round(x) + 0.5, 0);
    context.lineTo(Math.round(x) + 0.5, height);
  }
  for (let y = offsetY; y < height; y += spacing) {
    context.moveTo(0, Math.round(y) + 0.5);
    context.lineTo(width, Math.round(y) + 0.5);
  }
  context.stroke();

  context.beginPath();
  context.strokeStyle = "#344355";
  context.lineWidth = 1;
  context.moveTo(width / 2 + panX, 0);
  context.lineTo(width / 2 + panX, height);
  context.moveTo(0, height / 2 + panY);
  context.lineTo(width, height / 2 + panY);
  context.stroke();

  coordinateReadout.textContent = `X ${Math.round(-panX / zoom)} · Y ${Math.round(-panY / zoom)}`;
  updateZoomLabel();
}

function changeZoom(next: number): void {
  zoom = Math.max(0.25, Math.min(4, next));
  drawViewport();
}

document.querySelectorAll<HTMLButtonElement>("[data-tool]").forEach(button => {
  button.addEventListener("click", () => {
    const id = button.dataset.tool as ToolId | undefined;
    if (id && id in tools) setActiveTool(id);
  });
});

document.querySelector<HTMLButtonElement>("#zoom-in")?.addEventListener("click", () => changeZoom(zoom * 1.2));
document.querySelector<HTMLButtonElement>("#zoom-out")?.addEventListener("click", () => changeZoom(zoom / 1.2));
document.querySelector<HTMLButtonElement>("#fit-view")?.addEventListener("click", () => {
  zoom = 1;
  panX = 0;
  panY = 0;
  drawViewport();
});

canvas.addEventListener("pointerdown", event => {
  if (event.button !== 0) return;
  drag = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, originX: panX, originY: panY };
  canvas.setPointerCapture(event.pointerId);
  canvas.classList.add("is-panning");
});

canvas.addEventListener("pointermove", event => {
  if (!drag || drag.pointerId !== event.pointerId) return;
  panX = drag.originX + event.clientX - drag.startX;
  panY = drag.originY + event.clientY - drag.startY;
  drawViewport();
});

function endPan(event: PointerEvent): void {
  if (!drag || drag.pointerId !== event.pointerId) return;
  drag = null;
  canvas.classList.remove("is-panning");
  if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
}

canvas.addEventListener("pointerup", endPan);
canvas.addEventListener("pointercancel", endPan);
canvas.addEventListener("wheel", event => {
  if (!event.ctrlKey && !event.metaKey) return;
  event.preventDefault();
  changeZoom(zoom * (event.deltaY < 0 ? 1.08 : 1 / 1.08));
}, { passive: false });

window.addEventListener("keydown", event => {
  if (event.altKey || event.ctrlKey || event.metaKey || event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
  const shortcuts: Record<string, ToolId> = { v: "select", p: "pin", a: "array", b: "binding", g: "cage", e: "bezier", t: "tessellate" };
  const id = shortcuts[event.key.toLowerCase()];
  if (id) setActiveTool(id);
});

const resizeObserver = new ResizeObserver(drawViewport);
resizeObserver.observe(frame);
window.addEventListener("beforeunload", () => resizeObserver.disconnect());

setActiveTool("select");
drawViewport();
