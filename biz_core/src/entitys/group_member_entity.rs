use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
/// *
/// 群组成员详细信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, MongoDeriveMongoIndex)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, Debug)]
#[mongo_index(name("idx_group_uid"),fields["group_id","uid"], unique)]
pub struct GroupMemberEntity {
    /// 成员记录ID（内部持久化用）
    pub id: ::prost::alloc::string::String,
    /// 所属群组ID
    pub group_id: ::prost::alloc::string::String,
    /// 用户唯一ID
    pub uid: ::prost::alloc::string::String,
    /// 群内别名 / 昵称
    pub alias: ::prost::alloc::string::String,
    /// 成员角色
    pub role: i32,
    /// 是否禁言中（true=被禁言）
    pub is_muted: bool,
    /// 成员头像URL（前端展示用）
    pub avatar: ::prost::alloc::string::String,
    /// 加入时间
    pub create_time: u64,
    /// 更新时间
    pub update_time: u64,
}
