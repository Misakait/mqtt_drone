use bson::serde_helpers::serialize_bson_datetime_as_rfc3339_string;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct ShipTrack {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(rename = "startTime")]
    pub start_time: DateTime,
   #[serde(rename = "lastUpdate")]
    pub last_update: DateTime,
    #[serde(rename = "totalPoints")]
    pub total_points: u32,
    pub coordinates: Vec<[f64; 2]>,
}
// 新增：用于更新操作的请求体结构体
#[derive(Debug, Deserialize)]
pub struct UpdateShipTrackPayload {
    #[serde(rename = "coordinatesToAdd")]
    pub coordinates_to_add: Vec<[f64; 2]>,
}
// 新增：用于创建操作的请求体结构体
#[derive(Debug, Deserialize)] // 只需要 Deserialize，因为这是输入载荷
pub struct ShipTrackRequestDto {
    pub coordinates: Vec<[f64; 2]>,
    #[serde(rename = "totalPoints")]
    pub total_points: u32, // 客户端提供 total_points
}

#[derive(Debug, Serialize)] // Only Serialize is needed for responses
pub struct ShipTrackResponseDto {
    #[serde(rename = "_id", serialize_with = "serialize_object_id_as_hex_string")]
    pub id: ObjectId,

    #[serde(rename = "startTime", serialize_with = "serialize_bson_datetime_as_rfc3339_string")]
    pub start_time: DateTime, // mongodb::bson::DateTime

    #[serde(rename = "lastUpdate", serialize_with = "serialize_bson_datetime_as_rfc3339_string")]
    pub last_update: DateTime, // mongodb::bson::DateTime

    pub coordinates: Vec<[f64; 2]>,

    #[serde(rename = "totalPoints")]
    pub total_points: u32,
}

// Implement From trait for easy conversion from ShipTrack model to ShipTrackResponseDto
impl From<ShipTrack> for ShipTrackResponseDto {
    fn from(track_model: ShipTrack) -> Self {
        ShipTrackResponseDto {
            id: track_model.id,
            start_time: track_model.start_time,
            last_update: track_model.last_update,
            coordinates: track_model.coordinates,
            total_points: track_model.total_points,
        }
    }
}
