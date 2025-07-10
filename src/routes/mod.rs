use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_flash_messages::{FlashMessage, Level};
use lazy_static::lazy_static;
use log::error;
use tera::{Context, Tera};

use crate::models::auth::AuthenticatedUser;

pub mod client;
pub mod main;
pub mod settings;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {e}");
                ::std::process::exit(1);
            }
        }
    };
}

pub fn alert_level_to_str(level: &Level) -> &'static str {
    match level {
        Level::Error => "danger",
        Level::Warning => "warning",
        Level::Success => "success",
        _ => "info",
    }
}

fn redirect(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location))
        .finish()
}

fn check_role<I, S>(role: &str, roles: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    roles.into_iter().any(|r| r.as_ref() == role)
}

fn ensure_role(
    user: &AuthenticatedUser,
    role: &str,
    redirect_url: Option<&str>,
) -> Result<(), HttpResponse> {
    if check_role(role, &user.roles) {
        Ok(())
    } else {
        FlashMessage::error("Недостаточно прав.").send();
        Err(redirect(redirect_url.unwrap_or("/")))
    }
}

fn render_template(template: &str, context: &Context) -> HttpResponse {
    HttpResponse::Ok().body(TEMPLATES.render(template, context).unwrap_or_else(|e| {
        error!("Failed to render template '{template}': {e}");
        String::new()
    }))
}
