import {
  cloneRectangle,
  interpolateRectangle,
  type Rectangle,
  validateRectangle
} from "./geometry";

export interface Keyframe {
  frame: number;
  rectangle: Rectangle;
}

export interface Timeline {
  fps: number;
  keyframes: Keyframe[];
}

export function createTimeline(fps = 24): Timeline {
  assertFrameRate(fps);
  return { fps, keyframes: [] };
}

export function setKeyframe(timeline: Timeline, frame: number, rectangle: Rectangle): Timeline {
  validateTimeline(timeline);
  assertFrame(frame);
  validateRectangle(rectangle);
  const keyframes = timeline.keyframes
    .filter(keyframe => keyframe.frame !== frame)
    .map(keyframe => ({ frame: keyframe.frame, rectangle: cloneRectangle(keyframe.rectangle) }));
  keyframes.push({ frame, rectangle: cloneRectangle(rectangle) });
  keyframes.sort((a, b) => a.frame - b.frame);
  return { fps: timeline.fps, keyframes };
}

export function removeKeyframe(timeline: Timeline, frame: number): Timeline {
  validateTimeline(timeline);
  assertFrame(frame);
  return {
    fps: timeline.fps,
    keyframes: timeline.keyframes
      .filter(keyframe => keyframe.frame !== frame)
      .map(keyframe => ({ frame: keyframe.frame, rectangle: cloneRectangle(keyframe.rectangle) }))
  };
}

export function evaluateTimeline(timeline: Timeline, frame: number): Rectangle | null {
  validateTimeline(timeline);
  if (!Number.isFinite(frame) || frame < 0) throw new RangeError("Frame must be a non-negative finite number.");
  const keys = timeline.keyframes;
  if (keys.length === 0) return null;
  if (frame <= keys[0].frame) return cloneRectangle(keys[0].rectangle);
  const last = keys[keys.length - 1];
  if (frame >= last.frame) return cloneRectangle(last.rectangle);

  for (let index = 0; index < keys.length - 1; index++) {
    const left = keys[index], right = keys[index + 1];
    if (frame === left.frame) return cloneRectangle(left.rectangle);
    if (frame <= right.frame) {
      const t = (frame - left.frame) / (right.frame - left.frame);
      return interpolateRectangle(left.rectangle, right.rectangle, t);
    }
  }
  return cloneRectangle(last.rectangle);
}

export function validateTimeline(value: unknown): asserts value is Timeline {
  if (!value || typeof value !== "object") throw new TypeError("Timeline must be an object.");
  const timeline = value as Timeline;
  assertFrameRate(timeline.fps);
  if (!Array.isArray(timeline.keyframes)) throw new TypeError("Timeline keyframes must be an array.");
  let previous = -1;
  for (const keyframe of timeline.keyframes) {
    if (!keyframe || typeof keyframe !== "object") throw new TypeError("Invalid keyframe.");
    assertFrame(keyframe.frame);
    if (keyframe.frame <= previous) throw new TypeError("Keyframes must be unique and sorted by frame.");
    validateRectangle(keyframe.rectangle);
    previous = keyframe.frame;
  }
}

function assertFrame(frame: number): void {
  if (!Number.isInteger(frame) || frame < 0) throw new RangeError("Frame must be a non-negative integer.");
}
function assertFrameRate(fps: number): void {
  if (!Number.isFinite(fps) || fps <= 0 || fps > 240) throw new RangeError("FPS must be between 0 and 240.");
}
