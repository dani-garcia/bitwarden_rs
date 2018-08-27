use serde_json::Value as JsonValue;

use super::Cipher;
use CONFIG;

#[derive(Debug, Identifiable, Queryable, Insertable, Associations)]
#[table_name = "attachments"]
#[belongs_to(Cipher, foreign_key = "cipher_uuid")]
#[primary_key(id)]
pub struct Attachment {
    pub id: String,
    pub cipher_uuid: String,
    pub file_name: String,
    pub file_size: i32,
}

/// Local methods
impl Attachment {
    pub fn new(id: String, cipher_uuid: String, file_name: String, file_size: i32) -> Self {
        Self {
            id,
            cipher_uuid,
            file_name,
            file_size,
        }
    }

    pub fn get_file_path(&self) -> String {
        format!("{}/{}/{}", CONFIG.attachments_folder, self.cipher_uuid, self.id)
    }

    pub fn to_json(&self, host: &str) -> JsonValue {
        use util::get_display_size;

        let web_path = format!("{}/attachments/{}/{}", host, self.cipher_uuid, self.id);
        let display_size = get_display_size(self.file_size);

        json!({
            "Id": self.id,
            "Url": web_path,
            "FileName": self.file_name,
            "Size": self.file_size.to_string(),
            "SizeName": display_size,
            "Object": "attachment"
        })
    }
}

use diesel;
use diesel::prelude::*;
use db::DbConn;
use db::schema::attachments;

/// Database methods
impl Attachment {
    pub fn save(&self, conn: &DbConn) -> bool {
        match diesel::replace_into(attachments::table)
            .values(self)
            .execute(&**conn) {
            Ok(1) => true, // One row inserted
            _ => false,
        }
    }

    pub fn delete(self, conn: &DbConn) -> QueryResult<()> {
        use util;
        use std::{thread, time};

        let mut retries = 10;
        
        loop {
            match diesel::delete(
                attachments::table.filter(
                    attachments::id.eq(&self.id)
                )
            ).execute(&**conn) {
                Ok(_) => break,
                Err(err) => {
                    if retries < 1 {
                        println!("ERROR: Failed with 10 retries");
                        return Err(err)
                    } else {
                        retries = retries - 1;
                        println!("Had to retry! Retries left: {}", retries);
                        thread::sleep(time::Duration::from_millis(500));
                        continue
                    }
                }
            }
        }

        util::delete_file(&self.get_file_path());
        Ok(())
    }

    pub fn delete_all_by_cipher(cipher_uuid: &str, conn: &DbConn) -> QueryResult<()> {
        for attachement in Attachment::find_by_cipher(&cipher_uuid, &conn) {
            attachement.delete(&conn)?;
        }
        Ok(())
    }

    pub fn find_by_id(id: &str, conn: &DbConn) -> Option<Self> {
        attachments::table
            .filter(attachments::id.eq(id))
            .first::<Self>(&**conn).ok()
    }

    pub fn find_by_cipher(cipher_uuid: &str, conn: &DbConn) -> Vec<Self> {
        attachments::table
            .filter(attachments::cipher_uuid.eq(cipher_uuid))
            .load::<Self>(&**conn).expect("Error loading attachments")
    }

    pub fn find_by_ciphers(cipher_uuids: Vec<String>, conn: &DbConn) -> Vec<Self> {
        attachments::table
            .filter(attachments::cipher_uuid.eq_any(cipher_uuids))
            .load::<Self>(&**conn).expect("Error loading attachments")
    }
}
