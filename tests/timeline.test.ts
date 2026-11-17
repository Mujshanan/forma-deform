import { describe, expect, test } from "bun:test";
import { createRectangle } from "../src/core/geometry";
import { createTimeline, evaluateTimeline, setKeyframe } from "../src/core/timeline";

describe("timeline evaluation", () => {
  test("holds endpoints and interpolates between keyframes", () => {
    const start = createRectangle({ x: 0, y: 0 }, { x: 20, y: 20 });
    const end = { ...start, points: start.points.map(p => ({ x: p.x + 40, y: p.y + 20 })) as typeof start.points };
    let timeline = createTimeline();
    timeline = setKeyframe(timeline, 0, start);
    timeline = setKeyframe(timeline, 20, end);
    expect(evaluateTimeline(timeline, 0)?.points[0]).toEqual({ x: 0, y: 0 });
    expect(evaluateTimeline(timeline, 10)?.points[0]).toEqual({ x: 20, y: 10 });
    expect(evaluateTimeline(timeline, 30)?.points[0]).toEqual({ x: 40, y: 20 });
  });
  test("keyframe snapshots do not share mutable points", () => {
    const rectangle = createRectangle({ x: 0, y: 0 }, { x: 10, y: 10 });
    const timeline = setKeyframe(createTimeline(), 0, rectangle);
    rectangle.points[0].x = 99;
    expect(timeline.keyframes[0].rectangle.points[0].x).toBe(0);
  });
  test("setting an occupied frame replaces it", () => {
    const a = createRectangle({ x: 0, y: 0 }, { x: 10, y: 10 });
    const b = createRectangle({ x: 0, y: 0 }, { x: 30, y: 30 });
    const timeline = setKeyframe(setKeyframe(createTimeline(), 0, a), 0, b);
    expect(timeline.keyframes).toHaveLength(1);
    expect(timeline.keyframes[0].rectangle.points[1].x).toBe(30);
  });
});
