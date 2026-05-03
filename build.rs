fn main() {
    connectrpc_build::Config::new()
        .files(&[
            "proto/workers/auth/v1/auth.proto",
            "proto/workers/billing/v1/billing.proto",
            "proto/workers/org/v1/org.proto",
            "proto/workers/invitation/v1/invitation.proto",
        ])
        .includes(&["proto"])
        .include_file("_connectrpc.rs")
        .compile()
        .expect("failed to compile protos");
}
