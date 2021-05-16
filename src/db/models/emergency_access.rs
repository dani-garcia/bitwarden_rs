use chrono::{NaiveDateTime, Utc};
use serde_json::Value;

db_object! {
    #[derive(Debug, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
    #[table_name = "emergency_accesses"]
    #[changeset_options(treat_none_as_null="true")]
    #[belongs_to(User, foreign_key = "grantor_uuid")]
    #[primary_key(uuid)]
    pub struct EmergencyAccess {
        pub uuid: String,
        pub grantor_uuid: String,
        pub grantee_uuid: Option<String>,
        pub email: Option<String>,
        pub key_encrypted: Option<String>,
        pub atype: i32, //EmergencyAccessType
        pub status: i32, //EmergencyAccessStatus
        pub wait_time_days: i32,
        pub recovery_initiated_at: Option<NaiveDateTime>,
        pub last_notification_at: Option<NaiveDateTime>,
        pub updated_at: NaiveDateTime,
        pub created_at: NaiveDateTime,
    }
}

/// Local methods

impl EmergencyAccess {
    pub fn new(grantor_uuid: String, email: Option<String>, status: i32, atype: i32, wait_time_days: i32) -> Self {
        Self {
            uuid: crate::util::get_uuid(),
            grantor_uuid,
            grantee_uuid: None,
            email,
            status,
            atype,
            wait_time_days,
            recovery_initiated_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            key_encrypted: None,
            last_notification_at: None,
        }
    }

    pub fn get_atype_as_str(&self) -> &'static str {
        if self.atype == EmergencyAccessType::View as i32 {
            "View"
        } else {
            "Takeovver"
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "Id": self.uuid,
            "Status": self.status,
            "Type": self.atype,
            "WaitTimeDays": self.wait_time_days,
            "Object": "emergencyAccess",
        })
    }

    pub fn to_json_grantor_details(&self, conn: &DbConn) -> Value {
        // find grantor
        let grantor_user = User::find_by_uuid(&self.grantor_uuid, conn).unwrap();
        json!({
             "Id": self.uuid,
            "Status": self.status,
            "Type": self.atype,
            "WaitTimeDays": self.wait_time_days,
            "GrantorId": grantor_user.uuid,
            "Email": grantor_user.email,
            "Name": grantor_user.name,
            "Object": "emergencyAccessGrantorDetails",})
    }

    pub fn to_json_grantee_details(&self, conn: &DbConn) -> Value {
        if self.grantee_uuid.is_some() {
            let grantee_user =
                User::find_by_uuid(&self.grantee_uuid.clone().unwrap(), conn).expect("Grantee user not found.");

            json!({
                "Id": self.uuid,
                "Status": self.status,
                "Type": self.atype,
                "WaitTimeDays": self.wait_time_days,
                "GranteeId": grantee_user.uuid,
                "Email": grantee_user.email,
                "Name": grantee_user.name,
                "Object": "emergencyAccessGranteeDetails",})
        } else if self.email.is_some() {
            let grantee_user = User::find_by_mail(&self.email.clone().unwrap(), conn).expect("Grantee user not found.");
            json!({
                    "Id": self.uuid,
                    "Status": self.status,
                    "Type": self.atype,
                    "WaitTimeDays": self.wait_time_days,
                    "GranteeId": grantee_user.uuid,
                    "Email": grantee_user.email,
                    "Name": grantee_user.name,
                    "Object": "emergencyAccessGranteeDetails",})
        } else {
            json!({
                "Id": self.uuid,
                "Status": self.status,
                "Type": self.atype,
                "WaitTimeDays": self.wait_time_days,
                "GranteeId": "",
                "Email": "",
                "Name": "",
                "Object": "emergencyAccessGranteeDetails",})
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, num_derive::FromPrimitive)]
pub enum EmergencyAccessType {
    View = 0,
    Takeover = 1,
}

impl EmergencyAccessType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "0" | "View" => Some(EmergencyAccessType::View),
            "1" | "Takeover" => Some(EmergencyAccessType::Takeover),
            _ => None,
        }
    }
}

impl PartialEq<i32> for EmergencyAccessType {
    fn eq(&self, other: &i32) -> bool {
        *other == *self as i32
    }
}

impl PartialEq<EmergencyAccessType> for i32 {
    fn eq(&self, other: &EmergencyAccessType) -> bool {
        *self == *other as i32
    }
}

pub enum EmergencyAccessStatus {
    Invited = 0,
    Accepted = 1,
    Confirmed = 2,
    RecoveryInitiated = 3,
    RecoveryApproved = 4,
}

// region Database methods

use crate::db::DbConn;

use crate::api::EmptyResult;
use crate::db::models::User;
use crate::error::MapResult;

impl EmergencyAccess {
    pub fn save(&mut self, conn: &DbConn) -> EmptyResult {
        User::update_uuid_revision(&self.grantor_uuid, conn);
        self.updated_at = Utc::now().naive_utc();

        db_run! { conn:
            sqlite, mysql {
                match diesel::replace_into(emergency_accesses::table)
                    .values(EmergencyAccessDb::to_db(self))
                    .execute(conn)
                {
                    Ok(_) => Ok(()),
                    // Record already exists and causes a Foreign Key Violation because replace_into() wants to delete the record first.
                    Err(diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation, _)) => {
                        diesel::update(emergency_accesses::table)
                            .filter(emergency_accesses::uuid.eq(&self.uuid))
                            .set(EmergencyAccessDb::to_db(self))
                            .execute(conn)
                            .map_res("Error updating emergency access")
                    }
                    Err(e) => Err(e.into()),
                }.map_res("Error saving emergency access")
            }
            postgresql {
                let value = EmergencyAccessDb::to_db(self);
                diesel::insert_into(emergency_accesses::table)
                    .values(&value)
                    .on_conflict(emergency_accesses::uuid)
                    .do_update()
                    .set(&value)
                    .execute(conn)
                    .map_res("Error saving emergency access")
            }
        }
    }

    pub fn delete(self, conn: &DbConn) -> EmptyResult {
        User::update_uuid_revision(&self.grantor_uuid, conn);

        db_run! { conn: {
            diesel::delete(emergency_accesses::table.filter(emergency_accesses::uuid.eq(self.uuid)))
                .execute(conn)
                .map_res("Error removing user from organization")
        }}
    }

    pub fn find_by_uuid(uuid: &str, conn: &DbConn) -> Option<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::uuid.eq(uuid))
                .first::<EmergencyAccessDb>(conn)
                .ok().from_db()
        }}
    }

    pub fn find_by_grantor_uuid_and_grantee_uuid_or_email(
        grantor_uuid: &str,
        grantee_uuid: &str,
        email: &str,
        conn: &DbConn,
    ) -> Option<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::grantor_uuid.eq(grantor_uuid))
                .filter(emergency_accesses::grantee_uuid.eq(grantee_uuid).or(emergency_accesses::email.eq(email)))
                .first::<EmergencyAccessDb>(conn)
                .ok().from_db()
        }}
    }

    pub fn find_all_recoveries(conn: &DbConn) -> Vec<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::status.eq(EmergencyAccessStatus::RecoveryInitiated as i32))
                .load::<EmergencyAccessDb>(conn).expect("Error loading emergency_accesses").from_db()

        }}
    }

    pub fn find_by_uuid_and_grantor_uuid(uuid: &str, grantor_uuid: &str, conn: &DbConn) -> Option<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::uuid.eq(uuid))
                .filter(emergency_accesses::grantor_uuid.eq(grantor_uuid))
                .first::<EmergencyAccessDb>(conn)
                .ok().from_db()
        }}
    }

    pub fn find_all_by_grantee_uuid(grantee_uuid: &str, conn: &DbConn) -> Vec<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::grantee_uuid.eq(grantee_uuid))
                .load::<EmergencyAccessDb>(conn).expect("Error loading emergency_accesses").from_db()
        }}
    }
    pub fn find_all_by_grantor_uuid(grantor_uuid: &str, conn: &DbConn) -> Vec<Self> {
        db_run! { conn: {
            emergency_accesses::table
                .filter(emergency_accesses::grantor_uuid.eq(grantor_uuid))
                .load::<EmergencyAccessDb>(conn).expect("Error loading emergency_accesses").from_db()
        }}
    }
}

// endregion
