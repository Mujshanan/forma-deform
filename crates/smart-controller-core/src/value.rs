//! Finite, host-independent values used by controller inputs and outputs.

use crate::error::CoreError;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// A finite floating-point scalar.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Scalar(f64);

impl Scalar {
    pub fn new(value: f64) -> Result<Self, CoreError> {
        if !value.is_finite() {
            return Err(CoreError::NonFiniteValue { context: "scalar" });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Scalar {
    type Error = CoreError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for Scalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for Scalar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// A finite two-dimensional vector.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector2 {
    pub x: Scalar,
    pub y: Scalar,
}

impl Vector2 {
    pub fn new(x: f64, y: f64) -> Result<Self, CoreError> {
        Ok(Self {
            x: Scalar::new(x)?,
            y: Scalar::new(y)?,
        })
    }

    pub const fn x(self) -> f64 {
        self.x.get()
    }

    pub const fn y(self) -> f64 {
        self.y.get()
    }
}

/// An angle whose interpolation semantics are explicit.
///
/// Wrapped angles use shortest-path interpolation in the mapping layer;
/// unwrapped angles preserve authored revolutions and interpolate linearly.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Angle {
    radians: Scalar,
    wrapped: bool,
}

impl Angle {
    pub fn new(radians: f64, wrapped: bool) -> Result<Self, CoreError> {
        Ok(Self {
            radians: Scalar::new(radians)?,
            wrapped,
        })
    }

    pub const fn radians(self) -> f64 {
        self.radians.get()
    }

    pub const fn is_wrapped(self) -> bool {
        self.wrapped
    }
}

/// The canonical finite value union. Angle remains distinct from scalar so
/// consumers cannot accidentally apply ordinary scalar interpolation to it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TypedValue {
    Scalar(Scalar),
    Vector2(Vector2),
    Angle(Angle),
}

impl TypedValue {
    pub fn scalar(value: f64) -> Result<Self, CoreError> {
        Ok(Self::Scalar(Scalar::new(value)?))
    }

    pub fn vector2(x: f64, y: f64) -> Result<Self, CoreError> {
        Ok(Self::Vector2(Vector2::new(x, y)?))
    }

    pub fn angle(radians: f64, wrapped: bool) -> Result<Self, CoreError> {
        Ok(Self::Angle(Angle::new(radians, wrapped)?))
    }

    pub const fn kind_name(self) -> &'static str {
        match self {
            Self::Scalar(_) => "scalar",
            Self::Vector2(_) => "vector2",
            Self::Angle(_) => "angle",
        }
    }

    pub const fn as_scalar(self) -> Option<f64> {
        match self {
            Self::Scalar(value) => Some(value.get()),
            _ => None,
        }
    }

    pub const fn as_vector2(self) -> Option<Vector2> {
        match self {
            Self::Vector2(value) => Some(value),
            _ => None,
        }
    }

    pub const fn as_angle(self) -> Option<Angle> {
        match self {
            Self::Angle(value) => Some(value),
            _ => None,
        }
    }
}
