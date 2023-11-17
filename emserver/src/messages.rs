#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ping {
    #[prost(string, tag = "1")]
    pub content: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pong {
    #[prost(string, tag = "1")]
    pub server: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub content: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetFrame {
    /// Maps some key of your choice (ex. MARIO_X) to a memory address to be fetched.
    /// Key will be repeated in FrameDetails.
    #[prost(map = "string, uint32", tag = "2")]
    pub memory_requests: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        u32,
    >,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FrameContents {
    #[prost(bytes = "vec", tag = "1")]
    pub frame: ::prost::alloc::vec::Vec<u8>,
    /// Maps some key of your choice (ex. MARIO_X) to the associated byte.
    /// Missing key in the map means the fetch failed.
    #[prost(map = "string, uint32", tag = "2")]
    pub memory_values: ::std::collections::HashMap<::prost::alloc::string::String, u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FrameDetails {
    #[prost(message, optional, tag = "2")]
    pub frame: ::core::option::Option<FrameContents>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ControllerInput {
    #[prost(bool, tag = "1")]
    pub a: bool,
    #[prost(bool, tag = "2")]
    pub b: bool,
    #[prost(bool, tag = "3")]
    pub select: bool,
    #[prost(bool, tag = "4")]
    pub start: bool,
    #[prost(bool, tag = "5")]
    pub up: bool,
    #[prost(bool, tag = "6")]
    pub down: bool,
    #[prost(bool, tag = "7")]
    pub left: bool,
    #[prost(bool, tag = "8")]
    pub right: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TakeAction {
    /// Should be at least 1. # of frames to hold this input for before returning.
    #[prost(uint64, tag = "2")]
    pub skip_frames: u64,
    #[prost(message, optional, tag = "3")]
    pub input: ::core::option::Option<ControllerInput>,
    #[prost(map = "string, uint32", tag = "4")]
    pub memory_requests: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        u32,
    >,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ActionError {
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ActionResult {
    #[prost(message, optional, tag = "2")]
    pub frame: ::core::option::Option<FrameContents>,
    #[prost(message, optional, tag = "3")]
    pub error: ::core::option::Option<ActionError>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetState {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StateDetails {
    #[prost(bytes = "vec", tag = "1")]
    pub state: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetState {
    #[prost(bytes = "vec", tag = "1")]
    pub state: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetStateResult {
    #[prost(string, optional, tag = "2")]
    pub parse_error: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(oneof = "request::Contents", tags = "1, 3, 4, 5, 6")]
    pub contents: ::core::option::Option<request::Contents>,
}
/// Nested message and enum types in `Request`.
pub mod request {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Contents {
        #[prost(message, tag = "1")]
        Ping(super::Ping),
        #[prost(message, tag = "3")]
        GetFrame(super::GetFrame),
        #[prost(message, tag = "4")]
        TakeAction(super::TakeAction),
        #[prost(message, tag = "5")]
        GetState(super::GetState),
        #[prost(message, tag = "6")]
        SetState(super::SetState),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof = "response::Contents", tags = "1, 3, 4, 5, 6")]
    pub contents: ::core::option::Option<response::Contents>,
}
/// Nested message and enum types in `Response`.
pub mod response {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Contents {
        #[prost(message, tag = "1")]
        Pong(super::Pong),
        #[prost(message, tag = "3")]
        FrameDetails(super::FrameDetails),
        #[prost(message, tag = "4")]
        ActionResult(super::ActionResult),
        #[prost(message, tag = "5")]
        StateDetails(super::StateDetails),
        #[prost(message, tag = "6")]
        SetStateResult(super::SetStateResult),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Renderer {
    Software = 0,
    Hardware = 1,
}
impl Renderer {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Renderer::Software => "RENDERER_SOFTWARE",
            Renderer::Hardware => "RENDERER_HARDWARE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "RENDERER_SOFTWARE" => Some(Self::Software),
            "RENDERER_HARDWARE" => Some(Self::Hardware),
            _ => None,
        }
    }
}
