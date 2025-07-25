// This file is @generated by prost-build.
/// Generated client implementations.
pub mod arb_server_rpc_service_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// ============================
    /// 仲裁服务接口定义
    /// ============================
    #[derive(Debug, Clone)]
    pub struct ArbServerRpcServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ArbServerRpcServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ArbServerRpcServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ArbServerRpcServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::Body>,
            >>::Error: Into<StdError> + std::marker::Send + std::marker::Sync,
        {
            ArbServerRpcServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn update_shard_state(
            &mut self,
            request: impl tonic::IntoRequest<
                super::super::rpc_arb_models::UpdateShardStateRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::common::CommonResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_arb_server.ArbServerRpcService/UpdateShardState",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "rpc_arb_server.ArbServerRpcService",
                        "UpdateShardState",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// === 节点注册与生命周期 ===
        pub async fn register_node(
            &mut self,
            request: impl tonic::IntoRequest<super::super::rpc_arb_models::BaseRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::rpc_arb_models::NodeInfo>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_arb_server.ArbServerRpcService/RegisterNode",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("rpc_arb_server.ArbServerRpcService", "RegisterNode"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn list_all_nodes(
            &mut self,
            request: impl tonic::IntoRequest<super::super::rpc_arb_models::QueryNodeReq>,
        ) -> std::result::Result<
            tonic::Response<super::super::rpc_arb_models::ListAllNodesResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_arb_server.ArbServerRpcService/ListAllNodes",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("rpc_arb_server.ArbServerRpcService", "ListAllNodes"),
                );
            self.inner.unary(req, path, codec).await
        }
        /// === 节点状态与控制 ===
        pub async fn graceful_leave(
            &mut self,
            request: impl tonic::IntoRequest<super::super::rpc_arb_models::BaseRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::common::CommonResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_arb_server.ArbServerRpcService/GracefulLeave",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "rpc_arb_server.ArbServerRpcService",
                        "GracefulLeave",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn heartbeat(
            &mut self,
            request: impl tonic::IntoRequest<super::super::rpc_arb_models::BaseRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::common::CommonResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc_arb_server.ArbServerRpcService/heartbeat",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("rpc_arb_server.ArbServerRpcService", "heartbeat"),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
