//
// Shared library for sending mail via Gmail
//

use core::panic;
use std::ffi::{
    c_char,
    CStr,
    CString,
};
use serde::{Deserialize, Serialize};
use hyper::HeaderMap;
use lettre::transport::smtp;
use lettre::Message;
use lettre::message::{SinglePart, header::ContentType};
use lettre::SmtpTransport;
use lettre::Transport;
use once_cell::sync::Lazy;

static VERSION: &'static str = "0.1.0";

// mandatory struct
#[derive(Debug, Serialize)]
struct PluginRoute {
    path: &'static str,
    function: &'static str,
    method_router: &'static str,
    response_type: &'static str,
}

// add here all available routes for this plugin
static ROUTES: &[PluginRoute] = &[
    PluginRoute {
        path: "/sendmail",
        function: "sendmail",
        method_router: "post",
        response_type: "json",
    },
    PluginRoute {
        path: "/about",
        function: "about",
        method_router: "get",
        response_type: "text",
    },
];

#[derive(Clone, Deserialize, Serialize)]
struct Mail {
    from: String,
    to: String,
    cc: Option<String>,
    bcc: Option<String>,
    reply_to: Option<String>,
    sender_name: Option<String>,
    sender_email: Option<String>,
    subject: String,
    message: String,
    attachments: Option<Vec<String>>,
}

#[derive(Clone, Deserialize)]
struct SmtpSettings {
    username: String,
    password: String,
    server: String,
}

#[derive(Clone, Serialize)]
struct Response {
    status: String,
    message: String,
}

static SMTP_CLIENT: Lazy<SmtpSettings> = Lazy::new(|| {

    let config_file = match || -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {

        let plugins_dir = std::env::var("PLUGINS_DIR")
            .map(|val| val.is_empty()
                .then_some("plugins".to_string()
            )
            .or(Some(val)).unwrap())
            .unwrap_or("plugins".to_string());

        let plugins_path = std::path::Path::new(&plugins_dir);
        if !plugins_path.is_dir() {
            return Err(format!("Error: PLUGINS_DIR does not exist or is not set correctly: {}", plugins_dir).into());
        }

        let config_file = plugins_path.join("arp-gmail/config.json");
        if !config_file.is_file() {
            return Err("Error: Config file not found: arp-gmail/config.json".into());
        }
        Ok(config_file)
    }() {
        Ok(config_file) => config_file,
        Err(err) => {
            panic!("Error: {}", err);
        },
    };

    let file = std::fs::File::open(config_file).unwrap();
    let reader = std::io::BufReader::new(file);

    // Deserialize the JSON data into the struct
    match serde_json::from_reader(reader) {
        Ok(config) => config,
        Err(e) => {
            panic!("Error parsing config.json: {}", e);
        },
    }
});

fn to_c_response(r: &Response) -> *const c_char {
    let pretty_json = serde_json::to_string_pretty(&r)
        .unwrap();
    let c_response = CString::new(pretty_json)
        .unwrap();

    c_response.into_raw()
}

fn send_via_gmail(
    mail: &Mail,
) -> Result<smtp::response::Response, smtp::Error> {

    let email = Message::builder()
        .from(mail.from.parse().unwrap())
        .to(mail.to.parse().unwrap())
        .subject(&mail.subject)
        .singlepart(SinglePart::builder()
        .header(ContentType::TEXT_PLAIN)
        .body(mail.message.clone()))
        .unwrap();

    // Set up the SMTP client
    let credentials = smtp::authentication::Credentials::new(
        SMTP_CLIENT.username.to_owned(),
        SMTP_CLIENT.password.to_owned(),
    );

    let mailer = SmtpTransport::relay(&SMTP_CLIENT.server)
        .unwrap()
        .credentials(credentials)
        .build();

    // Send the email
    mailer.send(&email)
}

#[no_mangle]
pub extern "C" fn sendmail(
    headers: *mut HeaderMap,
    body: *const c_char,
) -> *const c_char {

    if headers.is_null() || body.is_null() {
        // Handle the null pointer case
        return std::ptr::null_mut();
    }

    // Convert headers pointer to a reference
    let headers = unsafe { &*headers };

    println!("Headers: {:?}", headers);

    let mut response = Response {
        status: "error".to_string(),
        message: "Internal plugin error".to_string(),
    };

    // Check if the content type is JSON
    if match headers.get("content-type") {
        Some(value) => {
            if value.to_str().unwrap_or("").to_string() != "application/json" {
                response.message = format!("Invalid content type: {:?}", value);
                true
            } else {
                false
            }
        },
        None => {
            response.message = "No content type".to_string();
            true
        },
    } {
        return to_c_response(&response);
    }

    // Convert body pointer to a Rust string
    let body_str = unsafe {
        CStr::from_ptr(body)
            .to_str()
            .unwrap_or("Invalid UTF-8 sequence") // Handle possible UTF-8 errors
    };

    // println!("Body Str: {}", body_str);

    let mail: Mail = match serde_json::from_str(body_str) {
        Ok(m) => m,
        Err(e) => {
            response.message = format!("Invalid JSON: {:?}", e);
            return to_c_response(&response);
        },
    };

    for (field, message) in vec![
        (&mail.from, "No from address"),
        (&mail.to, "No to address"),
        (&mail.subject, "No subject"),
        (&mail.message, "No message"),
    ] {
        if field.is_empty() {
            response.message = message.to_string();
            return to_c_response(&response);
        }
    }

    // https://myaccount.google.com/apppasswords

    match send_via_gmail(&mail) {
        Ok(success) => {
            response.status = "success".to_string();
            response.message = format!("Email sent successfully: {:?}", success);
        },
        Err(error) => {
            response.message = format!("Failed to send email: {:?}", error);
        },
    };

    to_c_response(&response)
}

// mandatory function
#[no_mangle]
pub extern "C" fn routes() -> *const c_char {

    let json_routes = serde_json::to_string_pretty(ROUTES)
        .unwrap_or("[]".to_string());

    let c_response = CString::new(json_routes.as_str())
        .unwrap();
    c_response
        .into_raw()
}

#[no_mangle]
pub extern "C" fn about(
    _headers: *mut HeaderMap,
    _body: *const c_char,
) -> *const c_char {

    let info = format!(r#"Name: arp-gmail
Version: {}
authors = "Henrique Dias <mrhdias@gmail.com>"
Description: Shared library for sending mail via Gmail
License: MIT"#, VERSION);

    let c_response = CString::new(info).unwrap();
    c_response.into_raw()
}

// mandatory function
#[no_mangle]
pub extern "C" fn free(ptr: *mut c_char) {
    if ptr.is_null() { // Avoid dereferencing null pointers
        return;
    }

    // Convert the raw pointer back to a CString and drop it to free the memory
    unsafe {
        drop(CString::from_raw(ptr)); // Takes ownership of the memory and frees it when dropped
    }
}