import { describe, expect, test } from "bun:test";
import { createRectangle, interpolateRectangle, setRectangleCorner, translateRectangle } from "../src/core/geometry";

describe("rectangle geometry", () => {
  test("normalizes all drag directions into ordered corners", () => {
    const rectangle = createRectangle({ x: 90, y: 80 }, { x: 10, y: 20 });
    expect(rectangle.points).toEqual([
      { x: 10, y: 20 }, { x: 90, y: 20 }, { x: 90, y: 80 }, { x: 10, y: 80 }
    ]);
  });
  test("translation preserves shape offsets", () => {
    const rectangle = createRectangle({ x: 0, y: 0 }, { x: 20, y: 10 });
    expect(translateRectangle(rectangle, { x: 5, y: -2 }).points[2]).toEqual({ x: 25, y: 8 });
  });
  test("corner editing changes only the selected point", () => {
    const rectangle = createRectangle({ x: 0, y: 0 }, { x: 20, y: 10 });
    const edited = setRectangleCorner(rectangle, 1, { x: 30, y: 4 });
    expect(edited.points[1]).toEqual({ x: 30, y: 4 });
    expect(edited.points[0]).toEqual(rectangle.points[0]);
    expect(rectangle.points[1]).toEqual({ x: 20, y: 0 });
  });
  test("interpolates corresponding points", () => {
    const a = createRectangle({ x: 0, y: 0 }, { x: 20, y: 10 });
    const b = { ...a, points: a.points.map(p => ({ x: p.x + 20, y: p.y + 10 })) as typeof a.points };
    expect(interpolateRectangle(a, b, 0.5).points[0]).toEqual({ x: 10, y: 5 });
  });
});
