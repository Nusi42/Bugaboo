//! What the pin use cases take and give back (CFG-05).

use bb_core::id::EntityId;
use bb_core::pin::PinKind;
use bb_policy::Masked;
use serde::Serialize;

/// The pins of the person, as they see them.
pub struct PinsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Pin a project or a saved search to the sidebar of the person, or take the pin away.
pub struct PinRequest {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the pin points to.
    pub kind: PinKind,
    /// The ID of the project or the saved search, as the person wrote it.
    pub target: String,
}

/// A pin as a request names it.
pub struct NamedPin {
    /// What the pin points to.
    pub kind: PinKind,
    /// The ID of the project or the saved search, as the person wrote it.
    pub target: String,
}

/// Put the pins in a new order: the pins the person sees, in the order they want them.
pub struct OrderPins {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The pins named, in the new order.
    pub pins: Vec<NamedPin>,
}

/// One pin as the person sees it: what it points to and the name to show.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PinView {
    /// What it points to.
    pub kind: PinKind,
    /// The ID of the project or the saved search.
    pub id: EntityId,
    /// The name of the project or the saved search.
    pub name: String,
    /// The short name of a project, which is in its address.
    pub short_name: Option<String>,
}

/// The pins of the person that they may open now, in their order. A pin to something they may no
/// longer open is not in it, and nothing says it was left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PinList {
    /// The pins.
    pub items: Vec<PinView>,
}

impl Masked for PinList {}
