import {
  createDocument, createRectangle, evaluateTimeline, pointInRectangle,
  setKeyframe, setRectangleCorner, translateRectangle, type CornerIndex,
  type DeformDocument, type Point, type Rectangle, parseDocument, serializeDocument
} from "../index";

const canvas = document.querySelector<HTMLCanvasElement>("#stage")!;
const context = canvas.getContext("2d")!;
const frameInput = document.querySelector<HTMLInputElement>("#frame")!;
const frameValue = document.querySelector<HTMLOutputElement>("#frame-value")!;
const modeLabel = document.querySelector<HTMLElement>("#mode-label")!;
const hint = document.querySelector<HTMLElement>("#hint")!;
const setKeyButton = document.querySelector<HTMLButtonElement>("#set-key")!;
const removeKeyButton = document.querySelector<HTMLButtonElement>("#remove-key")!;
const playButton = document.querySelector<HTMLButtonElement>("#play")!;
const track = document.querySelector<HTMLElement>("#track")!;
const playhead = document.querySelector<HTMLElement>("#playhead")!;
const keyCount = document.querySelector<HTMLElement>("#key-count")!;

let documentState: DeformDocument = createDocument(24);
let currentFrame = 0;
let currentRectangle: Rectangle | null = null;
let tool: "draw" | "select" = "draw";
let gesture: { kind: "draw"; start: Point } | { kind: "move"; start: Point; original: Rectangle } | { kind: "corner"; start: Point; original: Rectangle; corner: CornerIndex } | null = null;
let selected = false;
let playing = false;
let animationId = 0;
let lastTime = 0;
let playbackFrame = 0;

const colors = { background: "#111923", grid: "#1b2734", shape: "#e4a14b", line: "#f1c27d", selected: "#9bb8d5" };

function canvasPoint(event: PointerEvent): Point {
  const bounds = canvas.getBoundingClientRect();
  return {
    x: (event.clientX - bounds.left) * canvas.width / bounds.width,
    y: (event.clientY - bounds.top) * canvas.height / bounds.height
  };
}

function activeRectangle(): Rectangle | null {
  return currentRectangle ?? evaluateTimeline(documentState.timeline, currentFrame);
}

function render(): void {
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = colors.background;
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.strokeStyle = colors.grid;
  context.lineWidth = 1;
  for (let x = 0; x <= canvas.width; x += 40) {
    context.beginPath(); context.moveTo(x, 0); context.lineTo(x, canvas.height); context.stroke();
  }
  for (let y = 0; y <= canvas.height; y += 40) {
    context.beginPath(); context.moveTo(0, y); context.lineTo(canvas.width, y); context.stroke();
  }
  const rectangle = activeRectangle();
  if (!rectangle) return;
  context.beginPath();
  rectangle.points.forEach((point, index) => index === 0 ? context.moveTo(point.x, point.y) : context.lineTo(point.x, point.y));
  context.closePath();
  context.fillStyle = "#d9974520";
  context.fill();
  context.strokeStyle = selected ? colors.selected : colors.shape;
  context.lineWidth = 2;
  context.stroke();
  if (selected && tool === "select") {
    rectangle.points.forEach((point, index) => {
      context.beginPath(); context.arc(point.x, point.y, 7, 0, Math.PI * 2);
      context.fillStyle = "#111923"; context.fill();
      context.strokeStyle = colors.line; context.lineWidth = 2; context.stroke();
      context.fillStyle = "#e8edf4"; context.font = "10px sans-serif"; context.fillText(String(index + 1), point.x + 10, point.y - 9);
    });
  }
}

function refreshTimeline(): void {
  frameInput.value = String(currentFrame);
  frameValue.value = String(currentFrame);
  playhead.style.left = `${currentFrame / Number(frameInput.max) * 100}%`;
  track.querySelectorAll(".key-marker").forEach(node => node.remove());
  for (const key of documentState.timeline.keyframes) {
    const marker = document.createElement("span");
    marker.className = "key-marker";
    marker.style.left = `${key.frame / Number(frameInput.max) * 100}%`;
    marker.title = `Keyframe ${key.frame}`;
    track.append(marker);
  }
  keyCount.textContent = documentState.timeline.keyframes.length
    ? `${documentState.timeline.keyframes.length} keyframe(s)`
    : "No keyframes";
  setKeyButton.disabled = !documentState.rectangle || playing;
  removeKeyButton.disabled = !documentState.timeline.keyframes.some(key => key.frame === currentFrame) || playing;
  render();
}

function setFrame(frame: number): void {
  currentFrame = Math.max(0, Math.min(Number(frameInput.max), Math.round(frame)));
  currentRectangle = evaluateTimeline(documentState.timeline, currentFrame);
  refreshTimeline();
}

function setTool(next: "draw" | "select"): void {
  tool = next;
  document.querySelector("#draw-tool")!.classList.toggle("active", tool === "draw");
  document.querySelector("#select-tool")!.classList.toggle("active", tool === "select");
  modeLabel.textContent = tool === "draw" ? "Draw a rectangle" : "Select, move or edit a corner";
  hint.textContent = tool === "draw"
    ? "Click and drag on the stage to create a rectangle."
    : "Drag inside the shape to move it, or drag one of its four corner handles.";
  render();
}

function nearestCorner(point: Point, rectangle: Rectangle): CornerIndex | null {
  const tolerance = 14;
  let best: CornerIndex | null = null, distance = tolerance * tolerance;
  rectangle.points.forEach((corner, index) => {
    const d = (corner.x - point.x) ** 2 + (corner.y - point.y) ** 2;
    if (d <= distance) { distance = d; best = index as CornerIndex; }
  });
  return best;
}

canvas.addEventListener("pointerdown", event => {
  if (playing) return;
  const point = canvasPoint(event);
  const rectangle = activeRectangle();
  if (tool === "draw") {
    gesture = { kind: "draw", start: point };
    canvas.setPointerCapture(event.pointerId);
    return;
  }
  if (!rectangle) return;
  const corner = selected ? nearestCorner(point, rectangle) : null;
  if (corner !== null) {
    gesture = { kind: "corner", start: point, original: rectangle, corner };
  } else if (pointInRectangle(point, rectangle)) {
    selected = true;
    gesture = { kind: "move", start: point, original: rectangle };
  } else {
    selected = false;
    gesture = null;
  }
  canvas.setPointerCapture(event.pointerId);
  render();
});

canvas.addEventListener("pointermove", event => {
  if (!gesture) return;
  const point = canvasPoint(event);
  if (gesture.kind === "draw") {
    try { currentRectangle = createRectangle(gesture.start, point); } catch { currentRectangle = null; }
  } else if (gesture.kind === "move") {
    currentRectangle = translateRectangle(gesture.original, { x: point.x - gesture.start.x, y: point.y - gesture.start.y });
  } else {
    currentRectangle = setRectangleCorner(gesture.original, gesture.corner, point);
  }
  render();
});

canvas.addEventListener("pointerup", event => {
  if (!gesture) return;
  if (gesture.kind === "draw" && currentRectangle) {
    documentState.rectangle = currentRectangle;
    documentState.timeline = setKeyframe(documentState.timeline, currentFrame, currentRectangle);
    selected = true;
    setTool("select");
  } else if (gesture.kind !== "draw" && currentRectangle) {
    documentState.rectangle = currentRectangle;
  }
  gesture = null;
  try { canvas.releasePointerCapture(event.pointerId); } catch { /* pointer capture may already be released */ }
  refreshTimeline();
});

canvas.addEventListener("pointercancel", () => { gesture = null; render(); });
document.querySelector("#draw-tool")!.addEventListener("click", () => setTool("draw"));
document.querySelector("#select-tool")!.addEventListener("click", () => setTool("select"));
frameInput.addEventListener("input", () => { stopPlayback(); setFrame(Number(frameInput.value)); });

setKeyButton.addEventListener("click", () => {
  if (!documentState.rectangle) return;
  const shape = currentRectangle ?? documentState.rectangle;
  documentState.rectangle = shape;
  documentState.timeline = setKeyframe(documentState.timeline, currentFrame, shape);
  currentRectangle = shape;
  refreshTimeline();
});
removeKeyButton.addEventListener("click", () => {
  documentState.timeline.keyframes = documentState.timeline.keyframes.filter(key => key.frame !== currentFrame);
  currentRectangle = evaluateTimeline(documentState.timeline, currentFrame);
  refreshTimeline();
});
document.querySelector("#first-frame")!.addEventListener("click", () => { stopPlayback(); setFrame(0); });

function stopPlayback(): void {
  playing = false;
  cancelAnimationFrame(animationId);
  playButton.textContent = "▶ Play";
  refreshTimeline();
}
function tick(time: number): void {
  if (!playing) return;
  if (!lastTime) lastTime = time;
  const elapsed = time - lastTime;
  if (elapsed >= 1000 / documentState.timeline.fps) {
    playbackFrame += elapsed / (1000 / documentState.timeline.fps);
    lastTime = time;
    const lastKey = documentState.timeline.keyframes.at(-1)?.frame ?? 0;
    if (playbackFrame > Math.max(20, lastKey)) playbackFrame = 0;
    setFrame(Math.floor(playbackFrame));
  }
  animationId = requestAnimationFrame(tick);
}
playButton.addEventListener("click", () => {
  if (playing) { stopPlayback(); return; }
  if (documentState.timeline.keyframes.length < 2) {
    hint.textContent = "Set a second keyframe at a later frame to see the shape deform.";
    return;
  }
  playing = true; lastTime = 0; playbackFrame = currentFrame;
  playButton.textContent = "Ⅱ Pause";
  refreshTimeline();
  animationId = requestAnimationFrame(tick);
});

document.querySelector("#save")!.addEventListener("click", () => {
  const blob = new Blob([serializeDocument(documentState)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url; anchor.download = "forma-deform-project.json"; anchor.click();
  URL.revokeObjectURL(url);
});
document.querySelector<HTMLInputElement>("#load")!.addEventListener("change", async event => {
  const file = (event.currentTarget as HTMLInputElement).files?.[0];
  if (!file) return;
  try {
    stopPlayback();
    documentState = parseDocument(await file.text());
    currentFrame = 0; currentRectangle = evaluateTimeline(documentState.timeline, currentFrame);
    selected = false; refreshTimeline();
  } catch (error) {
    alert(error instanceof Error ? error.message : "Could not open project.");
  }
});

refreshTimeline();
setTool("draw");
