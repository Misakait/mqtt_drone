use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use mongodb::bson::serde_helpers::serialize_object_id_as_hex_string;
#[derive(Debug, Serialize, Deserialize)]
pub struct Flight {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(rename = "trackId")]
    pub track_id: ObjectId,// 关联的航迹ID
    #[serde(rename = "batteryCapacity")]
    pub battery_capacity: Vec<f64>, // 电池容量
    #[serde(rename = "estimatedRemainingUsageTime")]
    pub estimated_remaining_usage_time: Vec<f64>,
    #[serde(rename = "cabinTemperature")]
    pub cabin_temperature: Vec<f64>,
    #[serde(rename = "aircraftAltitude")]
    pub aircraft_altitude: Vec<f64>,
    #[serde(rename = "distanceToFan")]
    pub distance_to_fan: Vec<f64>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlightDto {
    // #[serde(serialize_with = "serialize_object_id_as_hex_string")]
    // pub track_id: ObjectId, // 航迹ID
    
    pub battery_capacity: f64, // 电池容量
  
    pub estimated_remaining_usage_time: f64,

    pub cabin_temperature: f64,

    pub aircraft_altitude: f64,

    pub distance_to_fan: f64,
}