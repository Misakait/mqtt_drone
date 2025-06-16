use crate::model::ship_track::ShipTrack;
use bson::{Bson, DateTime};
use chrono::Utc;
use mongodb::results::UpdateResult;
use mongodb::{bson::{doc, oid::ObjectId}, options::FindOneOptions, Collection};

pub struct ShipTrackService {
    pub collection: Collection<ShipTrack>,
}

impl ShipTrackService{
    pub fn new(collection: Collection<ShipTrack>) -> Self {
        Self { collection }
    }

    pub async fn create(&self, track: ShipTrack) -> mongodb::error::Result<()> {
        self.collection.insert_one(track).await?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> mongodb::error::Result<Option<ShipTrack>> {
        let obj_id = ObjectId::parse_str(id).unwrap();
        self.collection.find_one(doc! {"_id": obj_id}).await
    }

    pub async fn update(&self, id: &str, track: ShipTrack) -> mongodb::error::Result<()> {
        let obj_id = ObjectId::parse_str(id).unwrap();
        self.collection.replace_one(doc! {"_id": obj_id}, track).await?;
        Ok(())
    }
    // 新增方法：追加坐标并更新相关字段
    pub async fn append_coordinates_and_update(
        &self,
        id: &str,
        coordinates_to_add: Vec<[f64; 2]>,
    ) -> mongodb::error::Result<UpdateResult> {
        let obj_id = ObjectId::parse_str(id).map_err(|e| mongodb::error::Error::custom(format!("Invalid ObjectId: {}", e)))?;

        let current_time: DateTime = Utc::now().into();
        let mut update_document_parts = doc! { "$set": { "lastUpdate": current_time } };

        if !coordinates_to_add.is_empty() {
            let bson_coordinates_to_add: Vec<Bson> = coordinates_to_add
                .into_iter()
                .map(|coord_pair| Bson::Array(vec![Bson::Double(coord_pair[0]), Bson::Double(coord_pair[1])]))
                .collect();

            update_document_parts.insert("$push", doc! { "coordinates": { "$each": bson_coordinates_to_add } });
            update_document_parts.insert("$inc", doc! { "totalPoints": 1i32 }); // totalPoints 增加 1
        }
        // self.collection
        //     .find_one_and_update(doc! {"_id": obj_id}, update_document_parts)
        //     .with_options(options)
        //     .await
        self.collection
            .update_one(doc! {"_id": obj_id}, update_document_parts)
            .await
    }
    pub async fn delete(&self, id: &str) -> mongodb::error::Result<()> {
        let obj_id = ObjectId::parse_str(id).unwrap();
        self.collection.delete_one(doc! {"_id": obj_id}).await?;
        Ok(())
    }

    pub async fn get_latest(&self) -> mongodb::error::Result<Option<ShipTrack>> {
        let find_options = FindOneOptions::builder().sort(doc! {"lastUpdate": -1}).build();
        self.collection.find_one(doc! {}).with_options(find_options).await
    }
}





