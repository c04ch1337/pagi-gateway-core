pub mod bus;
pub mod canonical;
pub mod config;
#[cfg(feature = "digital-twin")]
pub mod digital_twin;
pub mod middleware;
pub mod protocols;
pub mod registry;

pub mod proto {
    tonic::include_proto!("pagi.v1");
}

