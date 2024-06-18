use std::{error, fmt};

/// Ecs Error
pub struct Error {
    kind: ErrorKind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ECS Error: {}!", self.kind)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ECS Error: {}!", self.kind)
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self { kind: value }
    }
}

impl error::Error for Error {}

/// Types of Ecs Errors
pub enum ErrorKind {
    EntityMaxReached,
    EntityDoesNotExist,
    EntityDoesNotOwnComponent,
    ComponentNotRegistered,
    ComponentArrayDowncastFailure,
    SceneMaxReached,
    SceneDoesNotExist,
    NoCurrentScene,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::EntityMaxReached => "max entity count reached",
            ErrorKind::EntityDoesNotExist => "entity doesn't exist in the scene",
            ErrorKind::EntityDoesNotOwnComponent => "entity doesn't have requested component",
            ErrorKind::ComponentNotRegistered => "unregistered component used",
            ErrorKind::ComponentArrayDowncastFailure => "failed to downcast component array",
            ErrorKind::SceneMaxReached => "max scene count reached",
            ErrorKind::SceneDoesNotExist => "scene doesn't exist",
            ErrorKind::NoCurrentScene => "there is no current scene",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
