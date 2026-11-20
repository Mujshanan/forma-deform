//! Host-independent document, rectangle geometry, and timeline model.
//!
//! This crate is the single authoritative implementation of the former TypeScript
//! prototype's document and geometry behavior. UI and WASM adapters should call it.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const DOCUMENT_VERSION: u32 = 1;
const MIN_RECTANGLE_EXTENT: f64 = 1.0;
const MAX_FPS: f64 = 240.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point { pub x: f64, pub y: f64 }

impl Point {
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rectangle { pub points: [Point; 4] }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub id: String,
    pub rectangle: Rectangle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerIndex { TopLeft, TopRight, BottomRight, BottomLeft }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub frame: u32,
    pub shape: Shape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    pub fps: f64,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeformDocument {
    pub version: u32,
    pub timeline: Timeline,
    pub shape: Option<Shape>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentError {
    EmptyShapeId,
    NonFinitePoint,
    RectangleTooSmall,
    InvalidCornerIndex,
    ShapeIdentityMismatch,
    NonFiniteInterpolation,
    InvalidFrame,
    InvalidFrameRate,
    UnsortedOrDuplicateKeyframes,
    UnsupportedVersion(u32),
    InvalidDocument(String),
    Serialization(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyShapeId => write!(f, "shape ID must not be empty"),
            Self::NonFinitePoint => write!(f, "point coordinates must be finite"),
            Self::RectangleTooSmall => write!(f, "rectangle must be at least 1 document unit wide and high"),
            Self::InvalidCornerIndex => write!(f, "corner index must be between 0 and 3"),
            Self::ShapeIdentityMismatch => write!(f, "cannot interpolate different shape identities"),
            Self::NonFiniteInterpolation => write!(f, "interpolation amount must be finite"),
            Self::InvalidFrame => write!(f, "frame must be a non-negative finite value"),
            Self::InvalidFrameRate => write!(f, "FPS must be greater than 0 and no greater than 240"),
            Self::UnsortedOrDuplicateKeyframes => write!(f, "keyframes must be unique and sorted by frame"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported project version: {v}"),
            Self::InvalidDocument(message) => write!(f, "invalid project document: {message}"),
            Self::Serialization(message) => write!(f, "project serialization failed: {message}"),
        }
    }
}
impl std::error::Error for DocumentError {}

impl Rectangle {
    /// Create a normalized rectangle from any drag direction.
    pub fn from_drag(start: Point, end: Point) -> Result<Self, DocumentError> {
        validate_point(start)?;
        validate_point(end)?;
        let left = start.x.min(end.x);
        let right = start.x.max(end.x);
        let top = start.y.min(end.y);
        let bottom = start.y.max(end.y);
        if right - left < MIN_RECTANGLE_EXTENT || bottom - top < MIN_RECTANGLE_EXTENT {
            return Err(DocumentError::RectangleTooSmall);
        }
        Ok(Self { points: [
            Point { x: left, y: top }, Point { x: right, y: top },
            Point { x: right, y: bottom }, Point { x: left, y: bottom },
        ] })
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        for point in self.points { validate_point(point)?; }
        Ok(())
    }

    pub fn translated(&self, delta: Point) -> Result<Self, DocumentError> {
        self.validate()?;
        validate_point(delta)?;
        let mut result = *self;
        for point in &mut result.points {
            point.x += delta.x;
            point.y += delta.y;
            validate_point(*point)?;
        }
        Ok(result)
    }

    pub fn with_corner(&self, corner: CornerIndex, point: Point) -> Result<Self, DocumentError> {
        self.validate()?;
        validate_point(point)?;
        let mut result = *self;
        let index = match corner {
            CornerIndex::TopLeft => 0,
            CornerIndex::TopRight => 1,
            CornerIndex::BottomRight => 2,
            CornerIndex::BottomLeft => 3,
        };
        result.points[index] = point;
        Ok(result)
    }

    pub fn interpolate(&self, other: &Self, amount: f64) -> Result<Self, DocumentError> {
        self.validate()?;
        other.validate()?;
        if !amount.is_finite() { return Err(DocumentError::NonFiniteInterpolation); }
        let t = amount.clamp(0.0, 1.0);
        let mut result = *self;
        for (index, point) in result.points.iter_mut().enumerate() {
            point.x += (other.points[index].x - point.x) * t;
            point.y += (other.points[index].y - point.y) * t;
            validate_point(*point)?;
        }
        Ok(result)
    }

    /// Ray-casting point-in-polygon test, matching the former prototype behavior.
    pub fn contains(&self, point: Point) -> Result<bool, DocumentError> {
        self.validate()?;
        validate_point(point)?;
        let mut inside = false;
        let mut j = 3;
        for i in 0..4 {
            let a = self.points[i];
            let b = self.points[j];
            let crosses = (a.y > point.y) != (b.y > point.y)
                && point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x;
            if crosses { inside = !inside; }
            j = i;
        }
        Ok(inside)
    }
}

impl Shape {
    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.id.is_empty() { return Err(DocumentError::EmptyShapeId); }
        self.rectangle.validate()
    }
}

impl Timeline {
    pub fn new(fps: f64) -> Result<Self, DocumentError> {
        validate_fps(fps)?;
        Ok(Self { fps, keyframes: Vec::new() })
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        validate_fps(self.fps)?;
        let mut previous = None;
        for key in &self.keyframes {
            key.shape.validate()?;
            if previous.is_some_and(|frame| key.frame <= frame) {
                return Err(DocumentError::UnsortedOrDuplicateKeyframes);
            }
            previous = Some(key.frame);
        }
        Ok(())
    }

    /// Insert or replace a frame, preserving immutable snapshots and sorted order.
    pub fn set_keyframe(&self, frame: u32, shape: &Shape) -> Result<Self, DocumentError> {
        self.validate()?;
        shape.validate()?;
        let mut result = self.clone();
        result.keyframes.retain(|key| key.frame != frame);
        result.keyframes.push(Keyframe { frame, shape: shape.clone() });
        result.keyframes.sort_by_key(|key| key.frame);
        Ok(result)
    }

    pub fn remove_keyframe(&self, frame: u32) -> Result<Self, DocumentError> {
        self.validate()?;
        let mut result = self.clone();
        result.keyframes.retain(|key| key.frame != frame);
        Ok(result)
    }

    /// Hold endpoint poses and linearly interpolate corresponding rectangle points.
    pub fn evaluate(&self, frame: f64) -> Result<Option<Shape>, DocumentError> {
        self.validate()?;
        if !frame.is_finite() || frame < 0.0 { return Err(DocumentError::InvalidFrame); }
        let keys = &self.keyframes;
        let Some(first) = keys.first() else { return Ok(None); };
        if frame <= first.frame as f64 { return Ok(Some(first.shape.clone())); }
        let last = keys.last().expect("first key guarantees last key");
        if frame >= last.frame as f64 { return Ok(Some(last.shape.clone())); }
        for pair in keys.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            if frame == left.frame as f64 { return Ok(Some(left.shape.clone())); }
            if frame <= right.frame as f64 {
                if left.shape.id != right.shape.id { return Err(DocumentError::ShapeIdentityMismatch); }
                let t = (frame - left.frame as f64) / (right.frame - left.frame) as f64;
                return Ok(Some(Shape {
                    id: left.shape.id.clone(),
                    rectangle: left.shape.rectangle.interpolate(&right.shape.rectangle, t)?,
                }));
            }
        }
        Ok(Some(last.shape.clone()))
    }
}

impl DeformDocument {
    pub fn new(fps: f64) -> Result<Self, DocumentError> {
        Ok(Self { version: DOCUMENT_VERSION, timeline: Timeline::new(fps)?, shape: None })
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.version != DOCUMENT_VERSION { return Err(DocumentError::UnsupportedVersion(self.version)); }
        self.timeline.validate()?;
        if let Some(shape) = &self.shape { shape.validate()?; }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, DocumentError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| DocumentError::Serialization(error.to_string()))
    }

    pub fn from_json(source: &str) -> Result<Self, DocumentError> {
        let document: Self = serde_json::from_str(source)
            .map_err(|error| DocumentError::Serialization(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }
}

fn validate_point(point: Point) -> Result<(), DocumentError> {
    if point.is_finite() { Ok(()) } else { Err(DocumentError::NonFinitePoint) }
}
fn validate_fps(fps: f64) -> Result<(), DocumentError> {
    if fps.is_finite() && fps > 0.0 && fps <= MAX_FPS {
        Ok(())
    } else {
        Err(DocumentError::InvalidFrameRate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(id: &str, start: Point, end: Point) -> Shape {
        Shape { id: id.into(), rectangle: Rectangle::from_drag(start, end).unwrap() }
    }

    #[test]
    fn drag_normalizes_all_directions_and_corner_edit_is_immutable() {
        let rect = Rectangle::from_drag(Point { x: 90.0, y: 80.0 }, Point { x: 10.0, y: 20.0 }).unwrap();
        assert_eq!(rect.points[0], Point { x: 10.0, y: 20.0 });
        let edited = rect.with_corner(CornerIndex::TopRight, Point { x: 30.0, y: 4.0 }).unwrap();
        assert_eq!(edited.points[1], Point { x: 30.0, y: 4.0 });
        assert_eq!(rect.points[1], Point { x: 90.0, y: 20.0 });
    }

    #[test]
    fn translation_and_hit_test() {
        let rect = Rectangle::from_drag(Point { x: 0.0, y: 0.0 }, Point { x: 20.0, y: 10.0 }).unwrap();
        let moved = rect.translated(Point { x: 5.0, y: -2.0 }).unwrap();
        assert_eq!(moved.points[2], Point { x: 25.0, y: 8.0 });
        assert!(moved.contains(Point { x: 10.0, y: 2.0 }).unwrap());
        assert!(!moved.contains(Point { x: 100.0, y: 2.0 }).unwrap());
    }

    #[test]
    fn timeline_holds_endpoints_interpolates_and_replaces() {
        let a = shape("s", Point { x: 0.0, y: 0.0 }, Point { x: 20.0, y: 20.0 });
        let b = shape("s", Point { x: 40.0, y: 20.0 }, Point { x: 60.0, y: 40.0 });
        let timeline = Timeline::new(24.0).unwrap().set_keyframe(0, &a).unwrap()
            .set_keyframe(20, &b).unwrap();
        let mid = timeline.evaluate(10.0).unwrap().unwrap();
        assert_eq!(mid.rectangle.points[0], Point { x: 20.0, y: 10.0 });
        assert_eq!(timeline.evaluate(30.0).unwrap().unwrap(), b);
        let replacement = shape("s", Point { x: 0.0, y: 0.0 }, Point { x: 30.0, y: 30.0 });
        assert_eq!(timeline.set_keyframe(0, &replacement).unwrap().keyframes.len(), 2);
        assert_eq!(timeline.remove_keyframe(20).unwrap().keyframes.len(), 1);
    }

    #[test]
    fn document_json_round_trip_and_version_validation() {
        let mut document = DeformDocument::new(24.0).unwrap();
        let item = shape("s", Point { x: 0.0, y: 0.0 }, Point { x: 20.0, y: 20.0 });
        document.shape = Some(item.clone());
        document.timeline = document.timeline.set_keyframe(0, &item).unwrap();
        let restored = DeformDocument::from_json(&document.to_json().unwrap()).unwrap();
        assert_eq!(restored, document);
        let mut invalid = document;
        invalid.version = 9;
        assert_eq!(invalid.validate(), Err(DocumentError::UnsupportedVersion(9)));
    }

    #[test]
    fn rejects_invalid_geometry_and_frame_rate() {
        assert_eq!(Rectangle::from_drag(Point { x: 0.0, y: 0.0 }, Point { x: 0.5, y: 2.0 }), Err(DocumentError::RectangleTooSmall));
        assert_eq!(Timeline::new(0.0), Err(DocumentError::InvalidFrameRate));
        assert_eq!(Rectangle::from_drag(Point { x: f64::NAN, y: 0.0 }, Point { x: 2.0, y: 2.0 }), Err(DocumentError::NonFinitePoint));
    }
}
