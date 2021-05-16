// Scheduler, and trait for .seconds(), .minutes(), etc.
use chrono::{Duration, Utc};
use clokwerk::{Scheduler, TimeUnits};

use std::process::exit;

use crate::{
    db::{models::*, DbConn, DbPool},
    mail, util, CONFIG,
};

pub fn init_jobs(scheduler: &mut Scheduler) {
    info!("Initiating jobs");

    // Add some tasks to it

    scheduler.every(CONFIG.job_frequency_hour().hour()).run(emergency_request_timed_out_job);
    scheduler.every(CONFIG.job_frequency_hour().hour()).run(emergency_notification_reminder_job);
}

pub fn init_db_job() -> DbConn {
    let pool_jobs = match util::retry_db(DbPool::from_config, CONFIG.db_connection_retries()) {
        Ok(p) => p,
        Err(e) => {
            error!("Error creating database pool: {:?}", e);
            exit(1);
        }
    };
    match pool_jobs.get() {
        Ok(conn) => conn,
        Err(e) => {
            error!("Error get db connection: {:?}", e);
            exit(1);
        }
    }
}

pub fn emergency_request_timed_out_job() {
    info!("Start emergency_request_timeout_job");
    let conn = init_db_job();

    let emergency_accesses = EmergencyAccess::find_all_recoveries(&conn);

    if emergency_accesses.is_empty() {
        info!("No emergency request timeout to approve");
    }

    for mut emer in emergency_accesses {
        if emer.recovery_initiated_at.is_some()
            && Utc::now().naive_utc()
                >= emer.recovery_initiated_at.unwrap() + Duration::days(emer.wait_time_days as i64)
        {
            emer.status = EmergencyAccessStatus::RecoveryApproved as i32;
            emer.save(&conn).expect("Cannot save emergency access on job");

            if CONFIG.mail_enabled() {
                // get grantor user to send Accepted email
                let grantor_user = User::find_by_uuid(&emer.grantor_uuid, &conn).expect("Grantor user not found.");

                // get grantee user to send Accepted email
                let grantee_user =
                    User::find_by_uuid(&emer.grantee_uuid.clone().expect("Grantee user invalid."), &conn)
                        .expect("Grantee user not found.");

                if !CONFIG.is_email_domain_allowed(&grantor_user.email) {
                    error!("Email domain not valid.")
                }

                mail::send_emergency_access_recovery_timed_out(
                    &grantor_user.email,
                    &grantee_user.name.clone(),
                    &emer.get_atype_as_str(),
                )
                .expect("Error on sending email");

                if !CONFIG.is_email_domain_allowed(&grantee_user.email) {
                    error!("Email not valid.")
                }

                mail::send_emergency_access_recovery_approved(&grantee_user.email, &grantor_user.name.clone())
                    .expect("Error on sending email");
            }
        }
    }
}

pub fn emergency_notification_reminder_job() {
    info!("Start emergency_notification_job");
    let conn = init_db_job();

    let emergency_accesses = EmergencyAccess::find_all_recoveries(&conn);

    if emergency_accesses.is_empty() {
        info!("No emergency request reminder notification to send");
    }

    for mut emer in emergency_accesses {
        if (emer.recovery_initiated_at.is_some()
            && Utc::now().naive_utc()
                >= emer.recovery_initiated_at.unwrap() + Duration::days((emer.wait_time_days as i64) - 1))
            && (emer.last_notification_at.is_none()
                || (emer.last_notification_at.is_some()
                    && Utc::now().naive_utc() >= emer.last_notification_at.unwrap() + Duration::days(1)))
        {
            emer.save(&conn).expect("Cannot save emergency access on job");

            if CONFIG.mail_enabled() {
                // get grantor user to send Accepted email
                let grantor_user = User::find_by_uuid(&emer.grantor_uuid, &conn).expect("Grantor user not found.");

                // get grantee user to send Accepted email
                let grantee_user =
                    User::find_by_uuid(&emer.grantee_uuid.clone().expect("Grantee user invalid."), &conn)
                        .expect("Grantee user not found.");

                if !CONFIG.is_email_domain_allowed(&grantor_user.email) {
                    error!("Email not valid.")
                }
                mail::send_emergency_access_recovery_reminder(
                    &grantor_user.email,
                    &grantee_user.name.clone(),
                    &emer.get_atype_as_str(),
                    &emer.wait_time_days.to_string(),
                )
                .expect("Error on sending email");
            }
        }
    }
}
