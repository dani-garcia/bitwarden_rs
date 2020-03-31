//
// JWT Handling
//
use crate::util::read_file;
use chrono::{Duration, Utc};
use once_cell::sync::Lazy;

use jsonwebtoken::{self, Algorithm, Header, EncodingKey, DecodingKey};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::error::{Error, MapResult};
use crate::CONFIG;

const JWT_ALGORITHM: Algorithm = Algorithm::RS256;

pub static DEFAULT_VALIDITY: Lazy<Duration> = Lazy::new(|| Duration::hours(2));
static JWT_HEADER: Lazy<Header> = Lazy::new(|| Header::new(JWT_ALGORITHM));
pub static JWT_LOGIN_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|login", CONFIG.domain_origin()));
static JWT_INVITE_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|invite", CONFIG.domain_origin()));
static JWT_DELETE_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|delete", CONFIG.domain_origin()));
static JWT_VERIFYEMAIL_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|verifyemail", CONFIG.domain_origin()));
static JWT_ADMIN_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|admin", CONFIG.domain_origin()));
static PRIVATE_RSA_KEY: Lazy<Vec<u8>> = Lazy::new(|| match read_file(&CONFIG.private_rsa_key()) {
    Ok(key) => key,
    Err(e) => panic!("Error loading private RSA Key.\n Error: {}", e),
});
static PUBLIC_RSA_KEY: Lazy<Vec<u8>> = Lazy::new(|| match read_file(&CONFIG.public_rsa_key()) {
    Ok(key) => key,
    Err(e) => panic!("Error loading public RSA Key.\n Error: {}", e),
});

pub fn encode_jwt<T: Serialize>(claims: &T) -> String {
    match jsonwebtoken::encode(&JWT_HEADER, claims, &EncodingKey::from_rsa_der(&PRIVATE_RSA_KEY)) {
        Ok(token) => token,
        Err(e) => panic!("Error encoding jwt {}", e),
    }
}

fn decode_jwt<T: DeserializeOwned>(token: &str, issuer: String) -> Result<T, Error> {
    let validation = jsonwebtoken::Validation {
        leeway: 30, // 30 seconds
        validate_exp: true,
        validate_nbf: true,
        aud: None,
        iss: Some(issuer),
        sub: None,
        algorithms: vec![JWT_ALGORITHM],
    };

    let token = token.replace(char::is_whitespace, "");

    jsonwebtoken::decode(&token, &DecodingKey::from_rsa_der(&PUBLIC_RSA_KEY), &validation)
        .map(|d| d.claims)
        .map_res("Error decoding JWT")
}

pub fn decode_login(token: &str) -> Result<LoginJWTClaims, Error> {
    decode_jwt(token, JWT_LOGIN_ISSUER.to_string())
}

pub fn decode_invite(token: &str) -> Result<InviteJWTClaims, Error> {
    decode_jwt(token, JWT_INVITE_ISSUER.to_string())
}

pub fn decode_delete(token: &str) -> Result<DeleteJWTClaims, Error> {
    decode_jwt(token, JWT_DELETE_ISSUER.to_string())
}

pub fn decode_verify_email(token: &str) -> Result<VerifyEmailJWTClaims, Error> {
    decode_jwt(token, JWT_VERIFYEMAIL_ISSUER.to_string())
}

pub fn decode_admin(token: &str) -> Result<AdminJWTClaims, Error> {
    decode_jwt(token, JWT_ADMIN_ISSUER.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginJWTClaims {
    // Not before
    pub nbf: i64,
    // Expiration time
    pub exp: i64,
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,

    pub premium: bool,
    pub name: String,
    pub email: String,
    pub email_verified: bool,

    pub orgowner: Vec<String>,
    pub orgadmin: Vec<String>,
    pub orguser: Vec<String>,
    pub orgmanager: Vec<String>,

    // user security_stamp
    pub sstamp: String,
    // device uuid
    pub device: String,
    // [ "api", "offline_access" ]
    pub scope: Vec<String>,
    // [ "Application" ]
    pub amr: Vec<String>,
}

impl LoginJWTClaims {
    pub fn is_organization_owner(&self, org_uuid: &str) -> bool {
        if self.orgowner.contains(&org_uuid.to_string()) {
            return true;
        }

        false
    }

    pub fn is_organization_admin(&self, org_uuid: &str) -> bool {
        if self.orgadmin.contains(&org_uuid.to_string()) {
            return true;
        }

        self.is_organization_owner(&org_uuid)
    }

    #[allow(dead_code)]
    pub fn is_organization_manager(&self, org_uuid: &str) -> bool {
        if self.orgmanager.contains(&org_uuid.to_string()) {
            return true;
        }

        self.is_organization_admin(&org_uuid)
    }

    #[allow(dead_code)]
    pub fn is_organization_user(&self, org_uuid: &str) -> bool {
        if self.orguser.contains(&org_uuid.to_string()) {
            return true;
        }

        self.is_organization_manager(&org_uuid)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteJWTClaims {
    // Not before
    pub nbf: i64,
    // Expiration time
    pub exp: i64,
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,

    pub email: String,
    pub org_id: Option<String>,
    pub user_org_id: Option<String>,
    pub invited_by_email: Option<String>,
}

pub fn generate_invite_claims(
    uuid: String,
    email: String,
    org_id: Option<String>,
    user_org_id: Option<String>,
    invited_by_email: Option<String>,
) -> InviteJWTClaims {
    let time_now = Utc::now().naive_utc();
    InviteJWTClaims {
        nbf: time_now.timestamp(),
        exp: (time_now + Duration::days(5)).timestamp(),
        iss: JWT_INVITE_ISSUER.to_string(),
        sub: uuid,
        email,
        org_id,
        user_org_id,
        invited_by_email,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteJWTClaims {
    // Not before
    pub nbf: i64,
    // Expiration time
    pub exp: i64,
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,
}

pub fn generate_delete_claims(uuid: String) -> DeleteJWTClaims {
    let time_now = Utc::now().naive_utc();
    DeleteJWTClaims {
        nbf: time_now.timestamp(),
        exp: (time_now + Duration::days(5)).timestamp(),
        iss: JWT_DELETE_ISSUER.to_string(),
        sub: uuid,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyEmailJWTClaims {
    // Not before
    pub nbf: i64,
    // Expiration time
    pub exp: i64,
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,
}

pub fn generate_verify_email_claims(uuid: String) -> DeleteJWTClaims {
    let time_now = Utc::now().naive_utc();
    DeleteJWTClaims {
        nbf: time_now.timestamp(),
        exp: (time_now + Duration::days(5)).timestamp(),
        iss: JWT_VERIFYEMAIL_ISSUER.to_string(),
        sub: uuid,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminJWTClaims {
    // Not before
    pub nbf: i64,
    // Expiration time
    pub exp: i64,
    // Issuer
    pub iss: String,
    // Subject
    pub sub: String,
}

pub fn generate_admin_claims() -> AdminJWTClaims {
    let time_now = Utc::now().naive_utc();
    AdminJWTClaims {
        nbf: time_now.timestamp(),
        exp: (time_now + Duration::minutes(20)).timestamp(),
        iss: JWT_ADMIN_ISSUER.to_string(),
        sub: "admin_panel".to_string(),
    }
}

//
// Bearer token authentication
//
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;

use crate::db::models::{Device, User, UserOrgStatus, UserOrganization};
use crate::db::DbConn;

pub struct Headers {
    pub host: String,
    pub device: Device,
    pub user: User,
    pub claims: LoginJWTClaims,
}

impl<'a, 'r> FromRequest<'a, 'r> for Headers {
    type Error = &'static str;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let headers = request.headers();

        // Get host
        let host = if CONFIG.domain_set() {
            CONFIG.domain()
        } else if let Some(referer) = headers.get_one("Referer") {
            referer.to_string()
        } else {
            // Try to guess from the headers
            use std::env;

            let protocol = if let Some(proto) = headers.get_one("X-Forwarded-Proto") {
                proto
            } else if env::var("ROCKET_TLS").is_ok() {
                "https"
            } else {
                "http"
            };

            let host = if let Some(host) = headers.get_one("X-Forwarded-Host") {
                host
            } else if let Some(host) = headers.get_one("Host") {
                host
            } else {
                ""
            };

            format!("{}://{}", protocol, host)
        };

        // Get access_token
        let access_token: &str = match headers.get_one("Authorization") {
            Some(a) => match a.rsplit("Bearer ").next() {
                Some(split) => split,
                None => err_handler!("No access token provided"),
            },
            None => err_handler!("No access token provided"),
        };

        // Check JWT token is valid and get device and user from it
        let claims = match decode_login(access_token) {
            Ok(claims) => claims,
            Err(_) => err_handler!("Invalid claim"),
        };
        let claim = claims.clone();

        let device_uuid = claim.device;
        let user_uuid = claim.sub;

        let conn = match request.guard::<DbConn>() {
            Outcome::Success(conn) => conn,
            _ => err_handler!("Error getting DB"),
        };

        let device = match Device::find_by_uuid(&device_uuid, &conn) {
            Some(device) => device,
            None => err_handler!("Invalid device id"),
        };

        let user = match User::find_by_uuid(&user_uuid, &conn) {
            Some(user) => user,
            None => err_handler!("Device has no user associated"),
        };

        if user.security_stamp != claim.sstamp {
            err_handler!("Invalid security stamp")
        }

        Outcome::Success(Headers { host, device, user, claims })
    }
}

pub struct OrgHeaders {
    pub host: String,
    pub device: Device,
    pub user: User,
    pub claims: LoginJWTClaims,
    pub context_org_uuid: String,
}

// org_id is usually the second param ("/organizations/<org_id>")
// But there are cases where it is located in a query value.
// First check the param, if this is not a valid uuid, we will try the query value.
fn get_org_id(request: &Request) -> Option<String> {
    if let Some(Ok(org_id)) = request.get_param::<String>(1) {
        if uuid::Uuid::parse_str(&org_id).is_ok() {
            return Some(org_id);
        }
    }

    if let Some(Ok(org_id)) = request.get_query_value::<String>("organizationId") {
        if uuid::Uuid::parse_str(&org_id).is_ok() {
            return Some(org_id);
        }
    }

    None
}

impl<'a, 'r> FromRequest<'a, 'r> for OrgHeaders {
    type Error = &'static str;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match request.guard::<Headers>() {
            Outcome::Forward(_) => Outcome::Forward(()),
            Outcome::Failure(f) => Outcome::Failure(f),
            Outcome::Success(headers) => {
                match get_org_id(request) {
                    Some(org_id) => {
                        // This check would have been sufficient if the claims would be updated.
                        // Since this is not the case, we keep the database check here and disable this check for now.
                        // if !headers.claims.is_organization_user(&org_id) {
                        //     err_handler!("The current user isn't member of the organization")
                        // }

                        let conn = match request.guard::<DbConn>() {
                            Outcome::Success(conn) => conn,
                            _ => err_handler!("Error getting DB"),
                        };

                        let user = headers.user;
                        match UserOrganization::find_by_user_and_org(&user.uuid, &org_id, &conn) {
                            Some(user) => {
                                if user.status == UserOrgStatus::Confirmed as i32 {
                                    user
                                } else {
                                    err_handler!("The current user isn't confirmed member of the organization")
                                }
                            }
                            None => err_handler!("The current user isn't member of the organization"),
                        };

                        Outcome::Success(Self {
                            host: headers.host,
                            device: headers.device,
                            user,
                            claims: headers.claims,
                            context_org_uuid: org_id,
                        })
                    },
                    _ => err_handler!("Error getting the organization id"),
                }
            }
        }
    }
}

pub struct AdminHeaders {
    pub host: String,
    pub device: Device,
    pub user: User,
    pub claims: LoginJWTClaims,
    pub context_org_uuid: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for AdminHeaders {
    type Error = &'static str;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match request.guard::<OrgHeaders>() {
            Outcome::Forward(_) => Outcome::Forward(()),
            Outcome::Failure(f) => Outcome::Failure(f),
            Outcome::Success(headers) => {
                if headers.claims.is_organization_admin(&headers.context_org_uuid) {
                    Outcome::Success(Self {
                        host: headers.host,
                        device: headers.device,
                        user: headers.user,
                        claims: headers.claims,
                        context_org_uuid: headers.context_org_uuid,
                    })
                } else {
                    err_handler!("You need to be Admin or Owner to call this endpoint")
                }
            }
        }
    }
}

impl Into<Headers> for AdminHeaders {    
    fn into(self) -> Headers { 
        Headers {
            host: self.host,
            device: self.device,
            user: self.user,
            claims: self.claims
        }
     }
}

pub struct OwnerHeaders {
    pub host: String,
    pub device: Device,
    pub user: User,
    pub claims: LoginJWTClaims,
    pub context_org_uuid: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for OwnerHeaders {
    type Error = &'static str;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        match request.guard::<OrgHeaders>() {
            Outcome::Forward(_) => Outcome::Forward(()),
            Outcome::Failure(f) => Outcome::Failure(f),
            Outcome::Success(headers) => {
                if headers.claims.is_organization_owner(&headers.context_org_uuid) {
                    Outcome::Success(Self {
                        host: headers.host,
                        device: headers.device,
                        user: headers.user,
                        claims: headers.claims,
                        context_org_uuid: headers.context_org_uuid,
                    })
                } else {
                    err_handler!("You need to be Owner to call this endpoint")
                }
            }
        }
    }
}

//
// Client IP address detection
//
use std::net::IpAddr;

pub struct ClientIp {
    pub ip: IpAddr,
}

impl<'a, 'r> FromRequest<'a, 'r> for ClientIp {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let ip = if CONFIG._ip_header_enabled() {
            req.headers().get_one(&CONFIG.ip_header()).and_then(|ip| {
                match ip.find(',') {
                    Some(idx) => &ip[..idx],
                    None => ip,
                }
                .parse()
                .map_err(|_| warn!("'{}' header is malformed: {}", CONFIG.ip_header(), ip))
                .ok()
            })
        } else {
            None
        };

        let ip = ip
            .or_else(|| req.remote().map(|r| r.ip()))
            .unwrap_or_else(|| "0.0.0.0".parse().unwrap());

        Outcome::Success(ClientIp { ip })
    }
}
