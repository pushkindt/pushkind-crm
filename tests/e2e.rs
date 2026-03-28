use std::{collections::BTreeMap, time::Duration};

use actix_web::rt::time::sleep;
use chrono::Utc;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_emailer::models::zmq::{ZMQReplyMessage, ZMQUnsubscribeMessage};
use pushkind_todo::{
    domain::task::{TaskPriority, TaskStatus},
    dto::zmq::{ZmqTask, ZmqTaskAssignee, ZmqTaskAuthor, ZmqTaskClient},
};
use reqwest::{StatusCode, header, multipart};
use serde_json::Value;

#[allow(dead_code)]
#[path = "../src/bin/check_events.rs"]
mod check_events_bin;
mod common;

use pushkind_crm::{
    domain::{
        client::NewClient,
        client_event::ClientEventType,
        manager::NewManager,
        types::{ClientEmail, HubId, ManagerEmail},
    },
    repository::{
        ClientEventListQuery, ClientEventReader, ClientListQuery, ClientReader, ClientWriter,
        DieselRepository, ImportantFieldReader, ManagerReader, ManagerWriter,
    },
};

const OTHER_HUB_ID: i32 = 8;

async fn response_json(response: reqwest::Response) -> Value {
    let body = response
        .text()
        .await
        .expect("Response body should be readable.");
    serde_json::from_str(&body).expect("Response body should be valid JSON.")
}

fn repo(app: &common::TestApp) -> DieselRepository {
    app.repo()
}

fn hub_id() -> HubId {
    HubId::new(common::HUB_ID).expect("valid hub id")
}

fn other_hub_id() -> HubId {
    HubId::new(OTHER_HUB_ID).expect("valid other hub id")
}

fn form_body(fields: Vec<(impl Into<String>, impl Into<String>)>) -> String {
    let fields = fields
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect::<Vec<(String, String)>>();
    serde_html_form::to_string(&fields).expect("Form body should serialize.")
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_logged_out_user_is_redirected_to_auth() {
    let app = common::spawn_app().await;
    let client = common::build_no_redirect_client();

    let response = client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request CRM index.");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("Redirect location should be present.");
    assert!(location.starts_with("https://users.pushkind.test/auth/signin?next="));

    let api_response = client
        .get(format!("{}/api/v1/client-directory", app.address()))
        .send()
        .await
        .expect("Failed to request CRM directory API.");

    assert_eq!(api_response.status(), StatusCode::UNAUTHORIZED);
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_admin_full_management_story() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();
    let repo = repo(&app);

    common::login_as(
        &client,
        app.address(),
        "admin@example.com",
        "Admin User",
        common::HUB_ID,
        &["crm", "crm_admin"],
    )
    .await;

    let index_response = client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request CRM index.");

    assert_eq!(index_response.status(), StatusCode::OK);
    let index_html = index_response
        .text()
        .await
        .expect("CRM index should be readable.");
    assert!(index_html.contains("<title>CRM</title>"));

    let iam_response = client
        .get(format!("{}/api/v1/iam", app.address()))
        .send()
        .await
        .expect("Failed to request IAM payload.");

    assert_eq!(iam_response.status(), StatusCode::OK);
    let iam_payload = response_json(iam_response).await;
    assert_eq!(iam_payload["current_user"]["email"], "admin@example.com");
    assert!(
        iam_payload["navigation"]
            .as_array()
            .expect("navigation array")
            .iter()
            .any(|item| item["url"] == "/")
    );
    assert!(
        iam_payload["navigation"]
            .as_array()
            .expect("navigation array")
            .iter()
            .any(|item| item["url"] == "/managers")
    );
    assert!(
        iam_payload["local_menu_items"]
            .as_array()
            .expect("menu array")
            .iter()
            .any(|item| item["url"] == "/settings")
    );

    let add_client_response = client
        .post(format!("{}/client/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Alice Admin"),
            ("email", "alice@example.com"),
            ("phone", ""),
        ]))
        .send()
        .await
        .expect("Failed to add client.");

    assert_eq!(add_client_response.status(), StatusCode::CREATED);
    let created_client = repo
        .get_client_by_email(&ClientEmail::new("alice@example.com").unwrap(), hub_id())
        .expect("Client lookup should succeed.")
        .expect("Created client should exist.");
    let created_client_id = created_client.id;

    let upload_response = client
        .post(format!("{}/clients/upload", app.address()))
        .multipart(
            multipart::Form::new().part(
                "csv",
                multipart::Part::bytes(
                    b"name,email,phone,tier\nBob Import,bob@example.com,,silver\nInvalid,,,gold\n"
                        .to_vec(),
                )
                .file_name("clients.csv"),
            ),
        )
        .send()
        .await
        .expect("Failed to upload clients.");

    assert_eq!(upload_response.status(), StatusCode::OK);
    let imported_client = repo
        .get_client_by_email(&ClientEmail::new("bob@example.com").unwrap(), hub_id())
        .expect("Imported client lookup should succeed.")
        .expect("Imported client should exist.");
    let imported_client_with_fields = repo
        .get_client_by_id(imported_client.id, hub_id())
        .expect("Imported client-with-fields lookup should succeed.")
        .expect("Imported client-with-fields should exist.");
    assert_eq!(
        imported_client_with_fields
            .fields
            .as_ref()
            .and_then(|fields| fields.get("tier")),
        Some(&"silver".to_string())
    );

    let add_manager_response = client
        .post(format!("{}/managers/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Manager One"),
            ("email", "manager.one@example.com"),
        ]))
        .send()
        .await
        .expect("Failed to add manager.");

    assert_eq!(add_manager_response.status(), StatusCode::CREATED);
    let manager = repo
        .get_manager_by_email(
            &ManagerEmail::new("manager.one@example.com").unwrap(),
            hub_id(),
        )
        .expect("Manager lookup should succeed.")
        .expect("Manager should exist.");

    let assign_manager_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id", manager.id.get().to_string()),
            ("client_ids", created_client_id.get().to_string()),
            ("client_ids", imported_client.id.get().to_string()),
        ]))
        .send()
        .await
        .expect("Failed to assign manager.");

    assert_eq!(assign_manager_response.status(), StatusCode::OK);

    let managers_page_response = client
        .get(format!("{}/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers page.");

    assert_eq!(managers_page_response.status(), StatusCode::OK);
    let managers_html = managers_page_response
        .text()
        .await
        .expect("Managers page should be readable.");
    assert!(managers_html.contains("<title>CRM Managers</title>"));

    let managers_api_response = client
        .get(format!("{}/api/v1/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers API.");

    assert_eq!(managers_api_response.status(), StatusCode::OK);
    let managers_payload = response_json(managers_api_response).await;
    let manager_item = managers_payload["managers"]
        .as_array()
        .expect("Managers payload should be an array.")
        .iter()
        .find(|item| item["manager"]["email"] == "manager.one@example.com")
        .expect("Manager payload should include the created manager.");
    assert_eq!(
        manager_item["clients"]
            .as_array()
            .expect("Assigned clients array")
            .len(),
        2
    );

    let manager_modal_response = client
        .get(format!(
            "{}/api/v1/managers/{}",
            app.address(),
            manager.id.get()
        ))
        .send()
        .await
        .expect("Failed to request manager modal API.");

    assert_eq!(manager_modal_response.status(), StatusCode::OK);
    let manager_modal_payload = response_json(manager_modal_response).await;
    assert_eq!(
        manager_modal_payload["manager"]["email"],
        "manager.one@example.com"
    );

    let settings_page_response = client
        .get(format!("{}/settings", app.address()))
        .send()
        .await
        .expect("Failed to request settings page.");

    assert_eq!(settings_page_response.status(), StatusCode::OK);
    let settings_html = settings_page_response
        .text()
        .await
        .expect("Settings page should be readable.");
    assert!(settings_html.contains("<title>CRM Settings</title>"));

    let save_important_fields_response = client
        .post(format!("{}/important-fields", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![("fields", "Tier\nCity")]))
        .send()
        .await
        .expect("Failed to save important fields.");

    assert_eq!(save_important_fields_response.status(), StatusCode::OK);
    let stored_fields = repo
        .list_important_fields(hub_id())
        .expect("Important fields lookup should succeed.");
    let stored_field_names: Vec<_> = stored_fields
        .iter()
        .map(|field| field.field.as_str().to_string())
        .collect();
    assert_eq!(
        stored_field_names,
        vec!["City".to_string(), "Tier".to_string()]
    );

    let important_fields_response = client
        .get(format!("{}/api/v1/important-fields", app.address()))
        .send()
        .await
        .expect("Failed to request important fields API.");

    assert_eq!(important_fields_response.status(), StatusCode::OK);
    let important_fields_payload = response_json(important_fields_response).await;
    assert_eq!(important_fields_payload["fields_text"], "City\nTier");

    let directory_response = client
        .get(format!(
            "{}/api/v1/client-directory?search=Alice&page=1",
            app.address()
        ))
        .send()
        .await
        .expect("Failed to request client directory API.");

    assert_eq!(directory_response.status(), StatusCode::OK);
    let directory_payload = response_json(directory_response).await;
    assert_eq!(directory_payload["search_query"], "Alice");
    assert_eq!(
        directory_payload["clients"]["items"]
            .as_array()
            .expect("Directory items should be an array.")
            .len(),
        1
    );

    let client_page_response = client
        .get(format!(
            "{}/client/{}",
            app.address(),
            created_client_id.get()
        ))
        .send()
        .await
        .expect("Failed to request client page.");

    assert_eq!(client_page_response.status(), StatusCode::OK);
    let client_html = client_page_response
        .text()
        .await
        .expect("Client page should be readable.");
    assert!(client_html.contains("<title>CRM Client</title>"));

    let save_client_response = client
        .post(format!(
            "{}/client/{}/save",
            app.address(),
            created_client_id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Alice Updated"),
            ("email", "alice.updated@example.com"),
            ("phone", "+1 (415) 555-2671"),
            ("field", "Tier"),
            ("value", "gold"),
            ("field", "City"),
            ("value", "Paris"),
        ]))
        .send()
        .await
        .expect("Failed to save client.");

    assert_eq!(save_client_response.status(), StatusCode::OK);
    let updated_client = repo
        .get_client_by_id(created_client_id, hub_id())
        .expect("Updated client lookup should succeed.")
        .expect("Updated client should exist.");
    assert_eq!(updated_client.name.as_str(), "Alice Updated");
    assert_eq!(
        updated_client.email.as_ref().map(|email| email.as_str()),
        Some("alice.updated@example.com")
    );
    assert_eq!(
        updated_client.phone.as_ref().map(|phone| phone.as_str()),
        Some("+14155552671")
    );
    assert_eq!(
        updated_client
            .fields
            .as_ref()
            .and_then(|fields| fields.get("Tier")),
        Some(&"gold".to_string())
    );

    let comment_response = client
        .post(format!(
            "{}/client/{}/comment",
            app.address(),
            created_client_id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("subject", ""),
            ("message", "Met at the expo"),
            ("event_type", "comment"),
        ]))
        .send()
        .await
        .expect("Failed to add comment.");

    assert_eq!(comment_response.status(), StatusCode::OK);

    let attachment_response = client
        .post(format!(
            "{}/client/{}/attachment",
            app.address(),
            created_client_id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("text", "Spec"),
            ("url", "https://example.com/spec.pdf"),
        ]))
        .send()
        .await
        .expect("Failed to add attachment.");

    assert_eq!(attachment_response.status(), StatusCode::OK);

    let (total_events, events) = repo
        .list_client_events(ClientEventListQuery::new(created_client_id))
        .expect("Client events lookup should succeed.");
    assert_eq!(total_events, 2);
    assert!(
        events
            .iter()
            .any(|(event, _)| event.event_type == ClientEventType::Comment
                && event.event_data["text"] == "Met at the expo")
    );
    assert!(events.iter().any(
        |(event, _)| event.event_type == ClientEventType::DocumentLink
            && event.event_data["url"] == "https://example.com/spec.pdf"
    ));

    let client_details_response = client
        .get(format!(
            "{}/api/v1/clients/{}",
            app.address(),
            created_client_id.get()
        ))
        .send()
        .await
        .expect("Failed to request client details API.");

    assert_eq!(client_details_response.status(), StatusCode::OK);
    let client_details_payload = response_json(client_details_response).await;
    assert_eq!(client_details_payload["client"]["name"], "Alice Updated");
    assert_eq!(
        client_details_payload["todo_service_url"],
        "https://todo.pushkind.test"
    );
    assert_eq!(
        client_details_payload["files_service_url"],
        "https://files.pushkind.test"
    );
    assert!(
        client_details_payload["important_fields"]
            .as_array()
            .expect("Important fields array")
            .iter()
            .any(|field| field["label"] == "City" && field["value"] == "Paris")
    );
    assert!(
        client_details_payload["documents"]
            .as_array()
            .expect("Documents array")
            .iter()
            .any(|event| event["event_type"] == "DocumentLink")
    );

    let integration_clients_response = client
        .get(format!("{}/api/v1/clients", app.address()))
        .send()
        .await
        .expect("Failed to request integration clients API.");

    assert_eq!(integration_clients_response.status(), StatusCode::OK);
    let integration_clients_payload = response_json(integration_clients_response).await;
    assert_eq!(
        integration_clients_payload
            .as_array()
            .expect("Clients list should be an array.")
            .len(),
        2
    );

    let invalid_public_id_response = client
        .get(format!(
            "{}/api/v1/clients?public_id=not-a-uuid",
            app.address()
        ))
        .send()
        .await
        .expect("Failed to request clients API with invalid public id.");

    assert_eq!(invalid_public_id_response.status(), StatusCode::OK);
    let invalid_public_id_payload = response_json(invalid_public_id_response).await;
    assert!(
        invalid_public_id_payload
            .as_array()
            .expect("Clients list should be an array.")
            .is_empty()
    );

    let cleanup_response = client
        .post(format!("{}/settings/cleanup", app.address()))
        .send()
        .await
        .expect("Failed to cleanup clients.");

    assert_eq!(cleanup_response.status(), StatusCode::OK);
    let (remaining_total, remaining_clients) = repo
        .list_clients(ClientListQuery::new(hub_id()))
        .expect("Remaining clients lookup should succeed.");
    assert_eq!(remaining_total, 0);
    assert!(remaining_clients.is_empty());
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_admin_public_id_pagination_and_cross_hub_isolation_story() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();
    let repo = repo(&app);

    common::login_as(
        &client,
        app.address(),
        "admin.scope@example.com",
        "Scoped Admin",
        common::HUB_ID,
        &["crm", "crm_admin"],
    )
    .await;

    let own_clients = (1..=(DEFAULT_ITEMS_PER_PAGE + 1))
        .map(|index| {
            NewClient::try_new(
                common::HUB_ID,
                format!("Paged Client {index:02}"),
                Some(format!("page{index:02}@example.com")),
                None,
                None,
            )
            .expect("valid client")
        })
        .collect::<Vec<_>>();
    repo.create_or_replace_clients(&own_clients)
        .expect("Paged clients should be created.");

    repo.create_or_replace_clients(&[NewClient::try_new(
        OTHER_HUB_ID,
        "Other Hub Client".to_string(),
        Some("outside@example.com".to_string()),
        None,
        None,
    )
    .expect("valid other-hub client")])
        .expect("Other-hub client should be created.");

    let target_client = repo
        .get_client_by_email(&ClientEmail::new("page05@example.com").unwrap(), hub_id())
        .expect("Target client lookup should succeed.")
        .expect("Target client should exist.");
    let target_public_id = target_client
        .public_id
        .expect("Seeded clients should have public ids")
        .to_string();
    let other_hub_client = repo
        .get_client_by_email(
            &ClientEmail::new("outside@example.com").unwrap(),
            other_hub_id(),
        )
        .expect("Other-hub client lookup should succeed.")
        .expect("Other-hub client should exist.");

    let paged_directory_response = client
        .get(format!("{}/api/v1/client-directory?page=2", app.address()))
        .send()
        .await
        .expect("Failed to request paginated client directory.");

    assert_eq!(paged_directory_response.status(), StatusCode::OK);
    let paged_directory_payload = response_json(paged_directory_response).await;
    assert_eq!(paged_directory_payload["clients"]["page"], 2);
    let paged_items = paged_directory_payload["clients"]["items"]
        .as_array()
        .expect("Directory items should be an array.");
    assert_eq!(paged_items.len(), 1);
    assert_eq!(paged_items[0]["email"], "page21@example.com");

    let directory_public_id_response = client
        .get(format!(
            "{}/api/v1/client-directory?public_id={target_public_id}",
            app.address()
        ))
        .send()
        .await
        .expect("Failed to request directory by public id.");

    assert_eq!(directory_public_id_response.status(), StatusCode::OK);
    let directory_public_id_payload = response_json(directory_public_id_response).await;
    let directory_public_id_items = directory_public_id_payload["clients"]["items"]
        .as_array()
        .expect("Directory items should be an array.");
    assert_eq!(directory_public_id_items.len(), 1);
    assert_eq!(directory_public_id_items[0]["email"], "page05@example.com");
    assert_eq!(directory_public_id_items[0]["public_id"], target_public_id);

    let integration_public_id_response = client
        .get(format!(
            "{}/api/v1/clients?public_id={target_public_id}",
            app.address()
        ))
        .send()
        .await
        .expect("Failed to request integration API by public id.");

    assert_eq!(integration_public_id_response.status(), StatusCode::OK);
    let integration_public_id_payload = response_json(integration_public_id_response).await;
    let integration_public_id_items = integration_public_id_payload
        .as_array()
        .expect("Integration items should be an array.");
    assert_eq!(integration_public_id_items.len(), 1);
    assert_eq!(
        integration_public_id_items[0]["email"],
        "page05@example.com"
    );

    let integration_list_response = client
        .get(format!("{}/api/v1/clients", app.address()))
        .send()
        .await
        .expect("Failed to request integration client list.");

    assert_eq!(integration_list_response.status(), StatusCode::OK);
    let integration_list_payload = response_json(integration_list_response).await;
    assert_eq!(
        integration_list_payload
            .as_array()
            .expect("Integration list should be an array.")
            .len(),
        DEFAULT_ITEMS_PER_PAGE + 1
    );

    let other_hub_client_details_response = client
        .get(format!(
            "{}/api/v1/clients/{}",
            app.address(),
            other_hub_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request other-hub client details.");

    assert_eq!(
        other_hub_client_details_response.status(),
        StatusCode::NOT_FOUND
    );

    let other_hub_client_page_response = client
        .get(format!(
            "{}/client/{}",
            app.address(),
            other_hub_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request other-hub client page.");

    assert_eq!(other_hub_client_page_response.status(), StatusCode::OK);
    assert_eq!(
        other_hub_client_page_response.url().as_str(),
        format!("{}/", app.address())
    );

    let cleanup_response = client
        .post(format!("{}/settings/cleanup", app.address()))
        .send()
        .await
        .expect("Failed to cleanup current-hub clients.");

    assert_eq!(cleanup_response.status(), StatusCode::OK);

    let (remaining_current_total, remaining_current_clients) = repo
        .list_clients(ClientListQuery::new(hub_id()))
        .expect("Current-hub clients lookup should succeed.");
    assert_eq!(remaining_current_total, 0);
    assert!(remaining_current_clients.is_empty());

    let (remaining_other_total, remaining_other_clients) = repo
        .list_clients(ClientListQuery::new(other_hub_id()))
        .expect("Other-hub clients lookup should succeed.");
    assert_eq!(remaining_other_total, 1);
    assert_eq!(
        remaining_other_clients[0]
            .email
            .as_ref()
            .map(|email| email.as_str()),
        Some("outside@example.com")
    );
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_manager_user_scoped_access_story() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();
    let repo = repo(&app);

    repo.create_or_replace_clients(&[
        NewClient::try_new(
            common::HUB_ID,
            "Assigned Client".to_string(),
            Some("assigned@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
        NewClient::try_new(
            common::HUB_ID,
            "Hidden Client".to_string(),
            Some("hidden@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
    ])
    .expect("Seed clients should be created.");

    let assigned_client = repo
        .get_client_by_email(&ClientEmail::new("assigned@example.com").unwrap(), hub_id())
        .expect("Assigned client lookup should succeed.")
        .expect("Assigned client should exist.");
    let hidden_client = repo
        .get_client_by_email(&ClientEmail::new("hidden@example.com").unwrap(), hub_id())
        .expect("Hidden client lookup should succeed.")
        .expect("Hidden client should exist.");

    let manager = repo
        .create_or_update_manager(
            &NewManager::try_new(
                common::HUB_ID,
                "Manager User".to_string(),
                "manager@example.com".to_string(),
                true,
            )
            .unwrap(),
        )
        .expect("Manager should be created.");
    repo.assign_clients_to_manager(manager.id, &[assigned_client.id])
        .expect("Assigned client should be linked to manager.");

    common::login_as(
        &client,
        app.address(),
        "manager@example.com",
        "Manager User",
        common::HUB_ID,
        &["crm", "crm_manager"],
    )
    .await;

    let index_response = client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request CRM index as manager.");

    assert_eq!(index_response.status(), StatusCode::OK);

    let directory_response = client
        .get(format!("{}/api/v1/client-directory", app.address()))
        .send()
        .await
        .expect("Failed to request manager client directory.");

    assert_eq!(directory_response.status(), StatusCode::OK);
    let directory_payload = response_json(directory_response).await;
    let items = directory_payload["clients"]["items"]
        .as_array()
        .expect("Directory items should be an array.");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["email"], "assigned@example.com");

    let allowed_client_response = client
        .get(format!(
            "{}/api/v1/clients/{}",
            app.address(),
            assigned_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request assigned client details.");

    assert_eq!(allowed_client_response.status(), StatusCode::OK);

    let hidden_client_response = client
        .get(format!(
            "{}/api/v1/clients/{}",
            app.address(),
            hidden_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request hidden client details.");

    assert_eq!(hidden_client_response.status(), StatusCode::UNAUTHORIZED);

    let redirected_hidden_page_response = client
        .get(format!(
            "{}/client/{}",
            app.address(),
            hidden_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request hidden client page.");

    assert_eq!(redirected_hidden_page_response.status(), StatusCode::OK);
    assert_eq!(
        redirected_hidden_page_response.url().as_str(),
        format!("{}/", app.address())
    );

    let save_assigned_response = client
        .post(format!(
            "{}/client/{}/save",
            app.address(),
            assigned_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Assigned Updated"),
            ("email", "assigned.updated@example.com"),
            ("phone", "+1 415 555 2671"),
        ]))
        .send()
        .await
        .expect("Failed to save assigned client.");

    assert_eq!(save_assigned_response.status(), StatusCode::OK);
    let updated_assigned = repo
        .get_client_by_id(assigned_client.id, hub_id())
        .expect("Updated assigned client lookup should succeed.")
        .expect("Updated assigned client should exist.");
    assert_eq!(updated_assigned.name.as_str(), "Assigned Updated");

    let comment_assigned_response = client
        .post(format!(
            "{}/client/{}/comment",
            app.address(),
            assigned_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("subject", ""),
            ("message", "Assigned-only note"),
            ("event_type", "comment"),
        ]))
        .send()
        .await
        .expect("Failed to add comment to assigned client.");

    assert_eq!(comment_assigned_response.status(), StatusCode::OK);

    let save_hidden_response = client
        .post(format!(
            "{}/client/{}/save",
            app.address(),
            hidden_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Should Fail"),
            ("email", "hidden@example.com"),
            ("phone", ""),
        ]))
        .send()
        .await
        .expect("Failed to attempt hidden client save.");

    assert_eq!(save_hidden_response.status(), StatusCode::FORBIDDEN);

    let comment_hidden_response = client
        .post(format!(
            "{}/client/{}/comment",
            app.address(),
            hidden_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("subject", ""),
            ("message", "Should be blocked"),
            ("event_type", "comment"),
        ]))
        .send()
        .await
        .expect("Failed to attempt hidden client comment.");

    assert_eq!(comment_hidden_response.status(), StatusCode::FORBIDDEN);

    let attachment_hidden_response = client
        .post(format!(
            "{}/client/{}/attachment",
            app.address(),
            hidden_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("text", "Blocked attachment"),
            ("url", "https://example.com/blocked.pdf"),
        ]))
        .send()
        .await
        .expect("Failed to attempt hidden client attachment.");

    assert_eq!(attachment_hidden_response.status(), StatusCode::FORBIDDEN);

    let managers_page_response = client
        .get(format!("{}/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers page as manager.");

    assert_eq!(managers_page_response.status(), StatusCode::OK);
    assert_eq!(
        managers_page_response.url().as_str(),
        format!("{}/na", app.address())
    );
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_admin_manager_assignment_replacement_and_not_found_story() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();
    let repo = repo(&app);

    common::login_as(
        &client,
        app.address(),
        "admin.assignment@example.com",
        "Assignment Admin",
        common::HUB_ID,
        &["crm", "crm_admin"],
    )
    .await;

    repo.create_or_replace_clients(&[
        NewClient::try_new(
            common::HUB_ID,
            "First Assigned".to_string(),
            Some("first.assigned@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
        NewClient::try_new(
            common::HUB_ID,
            "Second Assigned".to_string(),
            Some("second.assigned@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
        NewClient::try_new(
            common::HUB_ID,
            "Replacement Assigned".to_string(),
            Some("replacement.assigned@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
        NewClient::try_new(
            OTHER_HUB_ID,
            "Cross Hub Assigned".to_string(),
            Some("cross-hub.assigned@example.com".to_string()),
            None,
            None,
        )
        .unwrap(),
    ])
    .expect("Seed clients should be created.");

    let first_client = repo
        .get_client_by_email(
            &ClientEmail::new("first.assigned@example.com").unwrap(),
            hub_id(),
        )
        .expect("First client lookup should succeed.")
        .expect("First client should exist.");
    let second_client = repo
        .get_client_by_email(
            &ClientEmail::new("second.assigned@example.com").unwrap(),
            hub_id(),
        )
        .expect("Second client lookup should succeed.")
        .expect("Second client should exist.");
    let replacement_client = repo
        .get_client_by_email(
            &ClientEmail::new("replacement.assigned@example.com").unwrap(),
            hub_id(),
        )
        .expect("Replacement client lookup should succeed.")
        .expect("Replacement client should exist.");
    let cross_hub_client = repo
        .get_client_by_email(
            &ClientEmail::new("cross-hub.assigned@example.com").unwrap(),
            other_hub_id(),
        )
        .expect("Cross-hub client lookup should succeed.")
        .expect("Cross-hub client should exist.");

    let add_manager_response = client
        .post(format!("{}/managers/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Replacement Manager"),
            ("email", "replacement.manager@example.com"),
        ]))
        .send()
        .await
        .expect("Failed to add manager.");

    assert_eq!(add_manager_response.status(), StatusCode::CREATED);
    let manager = repo
        .get_manager_by_email(
            &ManagerEmail::new("replacement.manager@example.com").unwrap(),
            hub_id(),
        )
        .expect("Manager lookup should succeed.")
        .expect("Manager should exist.");

    let first_assignment_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id", manager.id.get().to_string()),
            ("client_ids", first_client.id.get().to_string()),
            ("client_ids", second_client.id.get().to_string()),
        ]))
        .send()
        .await
        .expect("Failed to assign initial clients.");

    assert_eq!(first_assignment_response.status(), StatusCode::OK);

    let initial_modal_response = client
        .get(format!(
            "{}/api/v1/managers/{}",
            app.address(),
            manager.id.get()
        ))
        .send()
        .await
        .expect("Failed to request initial manager modal.");

    assert_eq!(initial_modal_response.status(), StatusCode::OK);
    let initial_modal_payload = response_json(initial_modal_response).await;
    let initial_client_emails = initial_modal_payload["clients"]
        .as_array()
        .expect("Manager modal clients should be an array.")
        .iter()
        .map(|client| client["email"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(initial_client_emails.len(), 2);
    assert!(initial_client_emails.contains(&"first.assigned@example.com".to_string()));
    assert!(initial_client_emails.contains(&"second.assigned@example.com".to_string()));

    let replacement_assignment_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id", manager.id.get().to_string()),
            ("client_ids", replacement_client.id.get().to_string()),
        ]))
        .send()
        .await
        .expect("Failed to replace manager assignments.");

    assert_eq!(replacement_assignment_response.status(), StatusCode::OK);

    let replacement_modal_response = client
        .get(format!(
            "{}/api/v1/managers/{}",
            app.address(),
            manager.id.get()
        ))
        .send()
        .await
        .expect("Failed to request replacement manager modal.");

    assert_eq!(replacement_modal_response.status(), StatusCode::OK);
    let replacement_modal_payload = response_json(replacement_modal_response).await;
    let replacement_clients = replacement_modal_payload["clients"]
        .as_array()
        .expect("Manager modal clients should be an array.");
    assert_eq!(replacement_clients.len(), 1);
    assert_eq!(
        replacement_clients[0]["email"],
        "replacement.assigned@example.com"
    );

    let cross_hub_assignment_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id", manager.id.get().to_string()),
            ("client_ids", cross_hub_client.id.get().to_string()),
        ]))
        .send()
        .await
        .expect("Failed to attempt cross-hub assignment.");

    assert_eq!(
        cross_hub_assignment_response.status(),
        StatusCode::BAD_REQUEST
    );
    let cross_hub_assignment_payload = response_json(cross_hub_assignment_response).await;
    assert_eq!(
        cross_hub_assignment_payload["message"],
        "Некорректный список клиентов"
    );

    let missing_manager_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id".to_string(), "999999".to_string()),
            (
                "client_ids".to_string(),
                replacement_client.id.get().to_string(),
            ),
        ]))
        .send()
        .await
        .expect("Failed to attempt missing-manager assignment.");

    assert_eq!(missing_manager_response.status(), StatusCode::NOT_FOUND);
    let missing_manager_payload = response_json(missing_manager_response).await;
    assert_eq!(missing_manager_payload["message"], "Менеджер не найден.");
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_client_details_sanitize_and_order_events_story() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();
    let repo = repo(&app);

    common::login_as(
        &client,
        app.address(),
        "admin.client@example.com",
        "Client Admin",
        common::HUB_ID,
        &["crm", "crm_admin"],
    )
    .await;

    let add_client_response = client
        .post(format!("{}/client/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Detail Story Client"),
            ("email", "detail.story@example.com"),
            ("phone", ""),
        ]))
        .send()
        .await
        .expect("Failed to add detail client.");

    assert_eq!(add_client_response.status(), StatusCode::CREATED);
    let detail_client = repo
        .get_client_by_email(
            &ClientEmail::new("detail.story@example.com").unwrap(),
            hub_id(),
        )
        .expect("Detail client lookup should succeed.")
        .expect("Detail client should exist.");

    let add_manager_response = client
        .post(format!("{}/managers/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Detail Manager"),
            ("email", "detail.manager@example.com"),
        ]))
        .send()
        .await
        .expect("Failed to add detail manager.");

    assert_eq!(add_manager_response.status(), StatusCode::CREATED);
    let detail_manager = repo
        .get_manager_by_email(
            &ManagerEmail::new("detail.manager@example.com").unwrap(),
            hub_id(),
        )
        .expect("Detail manager lookup should succeed.")
        .expect("Detail manager should exist.");

    let assign_manager_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("manager_id", detail_manager.id.get().to_string()),
            ("client_ids", detail_client.id.get().to_string()),
        ]))
        .send()
        .await
        .expect("Failed to assign detail manager.");

    assert_eq!(assign_manager_response.status(), StatusCode::OK);

    let email_event_response = client
        .post(format!(
            "{}/client/{}/comment",
            app.address(),
            detail_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("subject", "Quarterly Follow-up"),
            ("message", "Please review the latest deck"),
            ("event_type", "email"),
        ]))
        .send()
        .await
        .expect("Failed to add email event.");

    assert_eq!(email_event_response.status(), StatusCode::OK);

    sleep(Duration::from_secs(1)).await;

    let comment_response = client
        .post(format!(
            "{}/client/{}/comment",
            app.address(),
            detail_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("subject", ""),
            ("message", "<script>alert(1)</script><b>Met at the expo</b>"),
            ("event_type", "comment"),
        ]))
        .send()
        .await
        .expect("Failed to add sanitized comment.");

    assert_eq!(comment_response.status(), StatusCode::OK);

    sleep(Duration::from_secs(1)).await;

    let attachment_response = client
        .post(format!(
            "{}/client/{}/attachment",
            app.address(),
            detail_client.id.get()
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("text", "Expo Notes"),
            ("url", "https://example.com/expo-notes.pdf"),
        ]))
        .send()
        .await
        .expect("Failed to add detail attachment.");

    assert_eq!(attachment_response.status(), StatusCode::OK);

    let client_details_response = client
        .get(format!(
            "{}/api/v1/clients/{}",
            app.address(),
            detail_client.id.get()
        ))
        .send()
        .await
        .expect("Failed to request detail client payload.");

    assert_eq!(client_details_response.status(), StatusCode::OK);
    let client_details_payload = response_json(client_details_response).await;
    assert!(
        client_details_payload["managers"]
            .as_array()
            .expect("Managers array should exist.")
            .iter()
            .any(|manager| manager["email"] == "detail.manager@example.com")
    );
    assert_eq!(client_details_payload["total_events"], 3);

    let ordered_events = client_details_payload["events"]
        .as_array()
        .expect("Events array should exist.");
    assert_eq!(ordered_events.len(), 3);
    assert_eq!(ordered_events[0]["event_type"], "DocumentLink");
    assert_eq!(ordered_events[1]["event_type"], "Comment");
    assert_eq!(ordered_events[2]["event_type"], "Email");

    let sanitized_comment = ordered_events[1]["event_data"]["text"]
        .as_str()
        .expect("Comment text should be a string.");
    assert!(!sanitized_comment.contains("<script"));
    assert!(sanitized_comment.contains("Met at the expo"));
    assert_eq!(
        ordered_events[2]["event_data"]["subject"],
        "Quarterly Follow-up"
    );
    assert_eq!(
        ordered_events[2]["event_data"]["text"],
        "Please review the latest deck"
    );
    assert!(
        client_details_payload["documents"]
            .as_array()
            .expect("Documents array should exist.")
            .iter()
            .any(|event| event["event_data"]["url"] == "https://example.com/expo-notes.pdf")
    );
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_worker_event_ingestion_story() {
    let app = common::spawn_app().await;
    let repo = repo(&app);

    repo.create_or_replace_clients(&[NewClient::try_new(
        common::HUB_ID,
        "Worker Client".to_string(),
        Some("worker.client@example.com".to_string()),
        None,
        None,
    )
    .unwrap()])
        .expect("Worker client should be created.");

    let worker_client = repo
        .get_client_by_email(
            &ClientEmail::new("worker.client@example.com").unwrap(),
            hub_id(),
        )
        .expect("Worker client lookup should succeed.")
        .expect("Worker client should exist.");
    let worker_public_id = worker_client
        .public_id
        .expect("Worker client should have a public id")
        .to_string();

    check_events_bin::process_reply_message(
        ZMQReplyMessage {
            hub_id: common::HUB_ID,
            email: "worker.client@example.com".to_string(),
            message: "<script>alert(1)</script><b>Reply body</b>".to_string(),
            subject: Some("RE: CRM".to_string()),
        },
        repo.clone(),
    )
    .expect("Reply message processing should succeed.");

    check_events_bin::process_unsubscribe_message(
        ZMQUnsubscribeMessage {
            hub_id: common::HUB_ID,
            email: "worker.client@example.com".to_string(),
            reason: Some("No longer interested".to_string()),
        },
        repo.clone(),
    )
    .expect("Unsubscribe processing should succeed.");

    check_events_bin::process_task_message(
        ZmqTask {
            public_id: "task-123".to_string(),
            hub_id: common::HUB_ID,
            title: "Task from worker".to_string(),
            priority: TaskPriority::High,
            status: TaskStatus::Pending,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            due_date: None,
            completed_at: None,
            author: ZmqTaskAuthor {
                name: "Task Manager".to_string(),
                email: "task.manager@example.com".to_string(),
            },
            client: Some(ZmqTaskClient {
                name: "Worker Client".to_string(),
                public_id: worker_public_id,
            }),
            assignee: Some(ZmqTaskAssignee {
                name: "Assignee User".to_string(),
                email: "assignee@example.com".to_string(),
            }),
            description: Some("Follow up with the client".to_string()),
            track: Some("CRM".to_string()),
        },
        repo.clone(),
    )
    .expect("Task message processing should succeed.");

    let (total_events, events) = repo
        .list_client_events(ClientEventListQuery::new(worker_client.id))
        .expect("Worker events lookup should succeed.");
    assert_eq!(total_events, 3);
    assert!(events.iter().any(|(event, _)| {
        event.event_type == ClientEventType::Reply
            && event.event_data["subject"] == "RE: CRM"
            && event.event_data["text"]
                .as_str()
                .is_some_and(|text| !text.contains("<script") && text.contains("Reply body"))
    }));
    assert!(events.iter().any(|(event, _)| {
        event.event_type == ClientEventType::Unsubscribed
            && event.event_data["text"] == "No longer interested"
    }));
    assert!(events.iter().any(|(event, _)| {
        event.event_type == ClientEventType::Task
            && event.event_data["public_id"] == "task-123"
            && event.event_data["subject"] == "Task from worker"
            && event.event_data["text"] == "Follow up with the client"
            && event.event_data["track"] == "CRM"
            && event.event_data["priority"] == "High"
            && event.event_data["status"] == "Pending"
            && event.event_data["assignee"]["email"] == "assignee@example.com"
    }));
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_basic_user_and_admin_only_access_stories() {
    let app = common::spawn_app().await;
    let repo = repo(&app);

    repo.create_or_replace_clients(&[NewClient::try_new(
        common::HUB_ID,
        "Visible Through Integration API".to_string(),
        Some("viewer-seed@example.com".to_string()),
        None,
        Some(BTreeMap::from([(
            String::from("tier"),
            String::from("bronze"),
        )])),
    )
    .unwrap()])
        .expect("Seed client should be created.");

    let basic_client = common::build_reqwest_client();
    common::login_as(
        &basic_client,
        app.address(),
        "viewer@example.com",
        "Basic Viewer",
        common::HUB_ID,
        &["crm"],
    )
    .await;

    let basic_index_response = basic_client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request CRM index as basic user.");

    assert_eq!(basic_index_response.status(), StatusCode::OK);

    let basic_directory_response = basic_client
        .get(format!("{}/api/v1/client-directory", app.address()))
        .send()
        .await
        .expect("Failed to request client directory as basic user.");

    assert_eq!(basic_directory_response.status(), StatusCode::OK);
    let basic_directory_payload = response_json(basic_directory_response).await;
    assert!(
        basic_directory_payload["clients"]["items"]
            .as_array()
            .expect("Directory items should be an array.")
            .is_empty()
    );

    let integration_list_response = basic_client
        .get(format!("{}/api/v1/clients", app.address()))
        .send()
        .await
        .expect("Failed to request integration client list as basic user.");

    assert_eq!(integration_list_response.status(), StatusCode::OK);
    let integration_list_payload = response_json(integration_list_response).await;
    assert_eq!(
        integration_list_payload
            .as_array()
            .expect("Integration list should be an array.")
            .len(),
        1
    );

    let forbidden_add_client_response = basic_client
        .post(format!("{}/client/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Blocked"),
            ("email", "blocked@example.com"),
            ("phone", ""),
        ]))
        .send()
        .await
        .expect("Failed to attempt add client as basic user.");

    assert_eq!(
        forbidden_add_client_response.status(),
        StatusCode::FORBIDDEN
    );

    let basic_managers_response = basic_client
        .get(format!("{}/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers page as basic user.");

    assert_eq!(basic_managers_response.status(), StatusCode::OK);
    assert_eq!(
        basic_managers_response.url().as_str(),
        format!("{}/na", app.address())
    );

    let admin_only_client = common::build_reqwest_client();
    common::login_as(
        &admin_only_client,
        app.address(),
        "admin-only@example.com",
        "Admin Only",
        common::HUB_ID,
        &["crm_admin"],
    )
    .await;

    let admin_only_iam_response = admin_only_client
        .get(format!("{}/api/v1/iam", app.address()))
        .send()
        .await
        .expect("Failed to request IAM as admin-only user.");

    assert_eq!(admin_only_iam_response.status(), StatusCode::OK);
    let admin_only_iam_payload = response_json(admin_only_iam_response).await;
    assert!(
        !admin_only_iam_payload["navigation"]
            .as_array()
            .expect("navigation array")
            .iter()
            .any(|item| item["url"] == "/")
    );
    assert!(
        admin_only_iam_payload["navigation"]
            .as_array()
            .expect("navigation array")
            .iter()
            .any(|item| item["url"] == "/managers")
    );

    let admin_only_index_response = admin_only_client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request CRM index as admin-only user.");

    assert_eq!(admin_only_index_response.status(), StatusCode::OK);
    assert_eq!(
        admin_only_index_response.url().as_str(),
        format!("{}/na", app.address())
    );

    let admin_only_managers_response = admin_only_client
        .get(format!("{}/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers page as admin-only user.");

    assert_eq!(admin_only_managers_response.status(), StatusCode::OK);
    let admin_only_managers_html = admin_only_managers_response
        .text()
        .await
        .expect("Managers page should be readable.");
    assert!(admin_only_managers_html.contains("<title>CRM Managers</title>"));

    let admin_only_directory_response = admin_only_client
        .get(format!("{}/api/v1/client-directory", app.address()))
        .send()
        .await
        .expect("Failed to request directory API as admin-only user.");

    assert_eq!(
        admin_only_directory_response.status(),
        StatusCode::UNAUTHORIZED
    );

    let admin_only_integration_response = admin_only_client
        .get(format!("{}/api/v1/clients", app.address()))
        .send()
        .await
        .expect("Failed to request integration API as admin-only user.");

    assert_eq!(admin_only_integration_response.status(), StatusCode::OK);
    let admin_only_integration_payload = response_json(admin_only_integration_response).await;
    let admin_only_clients = admin_only_integration_payload
        .as_array()
        .expect("Integration list should be an array.");
    assert_eq!(admin_only_clients.len(), 1);
    assert_eq!(admin_only_clients[0]["email"], "viewer-seed@example.com");
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_crm_basic_user_is_blocked_from_admin_management_apis_and_mutations() {
    let app = common::spawn_app().await;
    let client = common::build_reqwest_client();

    common::login_as(
        &client,
        app.address(),
        "viewer.limits@example.com",
        "Viewer Limits",
        common::HUB_ID,
        &["crm"],
    )
    .await;

    let managers_api_response = client
        .get(format!("{}/api/v1/managers", app.address()))
        .send()
        .await
        .expect("Failed to request managers API as basic user.");

    assert_eq!(managers_api_response.status(), StatusCode::UNAUTHORIZED);

    let important_fields_api_response = client
        .get(format!("{}/api/v1/important-fields", app.address()))
        .send()
        .await
        .expect("Failed to request important-fields API as basic user.");

    assert_eq!(
        important_fields_api_response.status(),
        StatusCode::UNAUTHORIZED
    );

    let upload_response = client
        .post(format!("{}/clients/upload", app.address()))
        .multipart(
            multipart::Form::new().part(
                "csv",
                multipart::Part::bytes(
                    b"name,email\nBlocked Upload,blocked.upload@example.com\n".to_vec(),
                )
                .file_name("clients.csv"),
            ),
        )
        .send()
        .await
        .expect("Failed to attempt client upload as basic user.");

    assert_eq!(upload_response.status(), StatusCode::FORBIDDEN);

    let add_manager_response = client
        .post(format!("{}/managers/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Blocked Manager"),
            ("email", "blocked.manager@example.com"),
        ]))
        .send()
        .await
        .expect("Failed to attempt add manager as basic user.");

    assert_eq!(add_manager_response.status(), StatusCode::FORBIDDEN);

    let assign_manager_response = client
        .post(format!("{}/managers/assign", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![("manager_id", "1"), ("client_ids", "1")]))
        .send()
        .await
        .expect("Failed to attempt assign manager as basic user.");

    assert_eq!(assign_manager_response.status(), StatusCode::FORBIDDEN);

    let save_important_fields_response = client
        .post(format!("{}/important-fields", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![("fields", "Blocked Field")]))
        .send()
        .await
        .expect("Failed to attempt save important fields as basic user.");

    assert_eq!(
        save_important_fields_response.status(),
        StatusCode::FORBIDDEN
    );

    let cleanup_response = client
        .post(format!("{}/settings/cleanup", app.address()))
        .send()
        .await
        .expect("Failed to attempt cleanup as basic user.");

    assert_eq!(cleanup_response.status(), StatusCode::FORBIDDEN);
}
