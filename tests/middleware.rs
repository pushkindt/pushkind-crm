use actix_web::{
    App, HttpResponse,
    http::{StatusCode, header},
    test, web,
};

use pushkind_crm::{middleware::RedirectUnauthorized, models::config::ServerConfig};

#[actix_web::test]
async fn redirects_unauthorized_to_signin() {
    let server_config = ServerConfig {
        secret: "secret".to_string(),
        auth_service_url: "http://auth.test.me/".to_string(),
    };

    let app = test::init_service(
        App::new()
            .wrap(RedirectUnauthorized)
            .app_data(web::Data::new(server_config.clone()))
            .default_service(web::to(|| async { HttpResponse::Unauthorized().finish() })),
    )
    .await;

    let req = test::TestRequest::default().to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers().get(header::LOCATION).unwrap(),
        "http://auth.test.me/"
    );
}

#[actix_web::test]
async fn success_response_passes_through() {
    let server_config = ServerConfig {
        secret: "secret".to_string(),
        auth_service_url: "http://auth.test.me/".to_string(),
    };
    let app = test::init_service(
        App::new()
            .wrap(RedirectUnauthorized)
            .app_data(web::Data::new(server_config.clone()))
            .default_service(web::to(|| async { HttpResponse::Ok().finish() })),
    )
    .await;

    let req = test::TestRequest::default().to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
