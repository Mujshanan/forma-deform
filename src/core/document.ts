import { cloneRectangle, validateRectangle, type Rectangle } from "./geometry";
import { createTimeline, validateTimeline, type Timeline } from "./timeline";

export interface DeformDocument {
  version: 1;
  timeline: Timeline;
  rectangle: Rectangle | null;
}

export function createDocument(fps = 24): DeformDocument {
  return { version: 1, timeline: createTimeline(fps), rectangle: null };
}

export function serializeDocument(document: DeformDocument): string {
  validateDocument(document);
  return JSON.stringify(document, null, 2);
}

export function parseDocument(source: string): DeformDocument {
  const parsed: unknown = JSON.parse(source);
  validateDocument(parsed);
  return {
    version: 1,
    timeline: {
      fps: parsed.timeline.fps,
      keyframes: parsed.timeline.keyframes.map(key => ({
        frame: key.frame,
        rectangle: cloneRectangle(key.rectangle)
      }))
    },
    rectangle: parsed.rectangle ? cloneRectangle(parsed.rectangle) : null
  };
}

export function validateDocument(value: unknown): asserts value is DeformDocument {
  if (!value || typeof value !== "object") throw new TypeError("Project must be an object.");
  const document = value as DeformDocument;
  if (document.version !== 1) throw new TypeError("Unsupported project version.");
  validateTimeline(document.timeline);
  if (document.rectangle !== null) validateRectangle(document.rectangle);
}
