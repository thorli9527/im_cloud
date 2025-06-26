use mongodb::bson::{DateTime, oid::ObjectId};
use utoipa::ToSchema;
use utoipa::openapi::schema::{ObjectBuilder, RefOr, Schema, SchemaType};

impl ToSchema for DateTime {
    fn schema() -> RefOr<Schema> {
        Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::String)
                .format(Some("date-time".into()))
                .description(Some("MongoDB DateTime as ISO8601 string"))
                .build(),
        )
        .into()
    }
}

impl ToSchema for ObjectId {
    fn schema() -> RefOr<Schema> {
        Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::String)
                .format(Some("objectid".into()))
                .description(Some("MongoDB ObjectId as 24-char hex string"))
                .build(),
        )
        .into()
    }
}
