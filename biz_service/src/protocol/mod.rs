pub mod arb;

pub mod msg {
    pub mod auth{
        include!("auth.rs");
    }
    pub mod entity {
        include!("entity.rs");
    }
    pub mod group {
        include!("group.rs");
    }
    pub mod friend {
        include!("friend.rs");
    }
    pub mod system {
        include!("system.rs");
    }
    pub mod user {
        include!("user.rs");
    }
    pub mod message {
        include!("message.rs");
    }
    
    pub mod status {
        include!("status.rs");
    }
    pub mod common {
        include!("common.rs");
    }
    pub mod group_models {
        include!("group_models.rs");
    }
}
pub mod common {
    include!("common.rs");
}