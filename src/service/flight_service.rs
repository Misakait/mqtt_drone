use bson::doc;
use log::error;
use mongodb::bson::oid::ObjectId;
use mongodb::Collection;
use mongodb::results::UpdateResult;
use crate::model::flight::{Flight, FlightDto};

pub struct FlightService{
    pub collection: Collection<Flight>,
}
impl FlightService {
    pub fn new(collection: Collection<Flight>) -> Self {
        Self { collection }
    }

    pub async fn create(&self, flight: Flight) -> mongodb::error::Result<()> {
        self.collection.insert_one(flight).await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> mongodb::error::Result<Option<Flight>> {
        let obj_id = ObjectId::parse_str(id).map_err(|e| {
            error!("{:?}", e);
            mongodb::error::Error::custom(e)
        })?;
        self.collection.find_one(doc! {"_id": obj_id}).await
    }

    pub async fn update(&self, id: &str, flight: Flight) -> mongodb::error::Result<()> {
        let obj_id = ObjectId::parse_str(id).map_err(|e| {
            error!("{:?}", e);
            mongodb::error::Error::custom(e)
        })?;
        self.collection.replace_one(doc! {"_id": obj_id}, flight).await?;
        Ok(())
    }
    pub async fn append_data_and_update(&self, id: &str, payload: FlightDto) -> mongodb::error::Result<UpdateResult> {
        let obj_id = ObjectId::parse_str(id).map_err(|e| {
            error!("{:?}",e);
            mongodb::error::Error::custom(e)})?;
        let update = doc! {
            "$push": {
                "batteryCapacity": payload.battery_capacity,
                "estimatedRemainingUsageTime": payload.estimated_remaining_usage_time,
                "cabinTemperature": payload.cabin_temperature,
                "aircraftAltitude": payload.aircraft_altitude,
                "distanceToFan": payload.distance_to_fan,
            }
        };
        self.collection.update_one(doc! {"_id": obj_id}, update).await
    }
}