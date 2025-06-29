pub mod auth {
    include!("protocol_auth.rs");
}
pub mod common {
    include!("protocol_common.rs");
}
pub mod friend {
    include!("protocol_friend.rs");
}
pub mod entity {
    include!("protocol_entity.rs");
}
pub mod group {
    include!("protocol_group.rs");
}

pub mod message {
    include!("protocol_message.rs");
}

pub mod status {
    include!("protocol_status.rs");
}
pub mod system {
    include!("protocol_system.rs");
}
pub mod user {
    include!("protocol_user.rs");
}
