// CITA-Cloud v6.3.0 proto
// https://github.com/cita-cloud/cita_cloud_proto/tree/v6.3.0

pub mod common {
    tonic::include_proto!("common");
}

pub mod blockchain {
    tonic::include_proto!("blockchain");
}

pub mod controller {
    tonic::include_proto!("controller");
}

pub mod executor {
    tonic::include_proto!("executor");
}

pub mod evm {
    tonic::include_proto!("evm");
}
