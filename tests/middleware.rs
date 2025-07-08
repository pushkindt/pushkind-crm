use actix_web::{
    App, HttpResponse,
    http::{StatusCode, header},
    test, web,
};

use pushkind_crm::middleware::RedirectUnauthorized;

#[actix_web::test]
async fn redirects_unauthorized_to_signin() {
    let app = test::init_service(
        App::new()
            .wrap(RedirectUnauthorized)
            .default_service(web::to(|| async { HttpResponse::Unauthorized().finish() })),
    )
    .await;

    let req = test::TestRequest::default().to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers().get(header::LOCATION).unwrap(),
        "/auth/signin"
    );
}

#[actix_web::test]
async fn success_response_passes_through() {
    let app = test::init_service(
        App::new()
            .wrap(RedirectUnauthorized)
            .default_service(web::to(|| async { HttpResponse::Ok().finish() })),
    )
    .await;

    let req = test::TestRequest::default().to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
