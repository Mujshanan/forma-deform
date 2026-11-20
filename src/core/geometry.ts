export interface Point {
  x: number;
  y: number;
}

export type CornerIndex = 0 | 1 | 2 | 3;

export interface Rectangle {
  id: string;
  points: [Point, Point, Point, Point];
}

export function createRectangle(start: Point, end: Point, id = "shape-1"): Rectangle {
  assertPoint(start);
  assertPoint(end);
  const left = Math.min(start.x, end.x);
  const right = Math.max(start.x, end.x);
  const top = Math.min(start.y, end.y);
  const bottom = Math.max(start.y, end.y);
  if (right - left < 1 || bottom - top < 1) {
    throw new RangeError("Rectangle must be at least 1 document unit wide and high.");
  }
  return {
    id,
    points: [
      { x: left, y: top },
      { x: right, y: top },
      { x: right, y: bottom },
      { x: left, y: bottom }
    ]
  };
}

export function cloneRectangle(rectangle: Rectangle): Rectangle {
  validateRectangle(rectangle);
  return {
    id: rectangle.id,
    points: rectangle.points.map(({ x, y }) => ({ x, y })) as Rectangle["points"]
  };
}

export function validateRectangle(value: unknown): asserts value is Rectangle {
  if (!value || typeof value !== "object") throw new TypeError("Rectangle must be an object.");
  const rectangle = value as Rectangle;
  if (typeof rectangle.id !== "string" || rectangle.id.length === 0) {
    throw new TypeError("Rectangle id must be a non-empty string.");
  }
  if (!Array.isArray(rectangle.points) || rectangle.points.length !== 4) {
    throw new TypeError("Rectangle must contain exactly four ordered points.");
  }
  rectangle.points.forEach(assertPoint);
}

export function translateRectangle(rectangle: Rectangle, delta: Point): Rectangle {
  validateRectangle(rectangle);
  assertPoint(delta);
  return {
    id: rectangle.id,
    points: rectangle.points.map(point => ({
      x: point.x + delta.x,
      y: point.y + delta.y
    })) as Rectangle["points"]
  };
}

export function setRectangleCorner(
  rectangle: Rectangle,
  corner: CornerIndex,
  point: Point
): Rectangle {
  validateRectangle(rectangle);
  assertPoint(point);
  if (![0, 1, 2, 3].includes(corner)) throw new RangeError("Corner index must be 0–3.");
  const points = rectangle.points.map(p => ({ ...p })) as Rectangle["points"];
  points[corner] = { ...point };
  return { id: rectangle.id, points };
}

export function interpolateRectangle(
  from: Rectangle,
  to: Rectangle,
  t: number
): Rectangle {
  validateRectangle(from);
  validateRectangle(to);
  if (from.id !== to.id) throw new TypeError("Cannot interpolate different shape identities.");
  if (!Number.isFinite(t)) throw new TypeError("Interpolation amount must be finite.");
  const amount = Math.max(0, Math.min(1, t));
  return {
    id: from.id,
    points: from.points.map((point, index) => ({
      x: point.x + (to.points[index].x - point.x) * amount,
      y: point.y + (to.points[index].y - point.y) * amount
    })) as Rectangle["points"]
  };
}

export function distanceSquared(a: Point, b: Point): number {
  return (a.x - b.x) ** 2 + (a.y - b.y) ** 2;
}

export function pointInRectangle(point: Point, rectangle: Rectangle): boolean {
  const p = rectangle.points;
  let inside = false;
  for (let i = 0, j = 3; i < 4; j = i++) {
    const a = p[i], b = p[j];
    const crosses = (a.y > point.y) !== (b.y > point.y) &&
      point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x;
    if (crosses) inside = !inside;
  }
  return inside;
}

function assertPoint(value: unknown): asserts value is Point {
  if (!value || typeof value !== "object") throw new TypeError("Point must be an object.");
  const point = value as Point;
  if (!Number.isFinite(point.x) || !Number.isFinite(point.y)) {
    throw new TypeError("Point coordinates must be finite numbers.");
  }
}
