pub mod common;
pub mod rpc;

// pub mod rpc {
//     pub mod rpc_arb_models {
//         include!("./rpc/arb_models.rs");
//     }
//     pub mod rpc_arb_group {
//         include!("./rpc/arb_group.rs");
//     }
//     pub mod rpc_arb_server {
//         include!("./rpc/arb_server.rs");
//     }
//     pub mod rpc_arb_socket {
//         include!("./rpc/arb_socket.rs");
//     }
//     pub mod rpc_shard_server {
//         include!("./rpc/shard_service.rs");
//     }
// }

pub mod msg {
    pub mod auth {
        include!("msg/auth.rs");
    }
    pub mod group {
        include!("msg/group.rs");
    }
    pub mod friend {
        include!("msg/friend.rs");
    }
    pub mod system {
        include!("msg/system.rs");
    }
    pub mod user {
        include!("msg/user.rs");
    }
    pub mod message {
        include!("msg/message.rs");
    }

    pub mod status {
        include!("msg/status.rs");
    }
}
