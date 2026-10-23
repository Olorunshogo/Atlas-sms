mod common;

use axum::http::{Method, StatusCode};
use common::{bare_world, d, send, store_cashier, store_user, world, ACCOUNTANT_ROLE_ID};
use serde_json::{json, Value};
use uuid::Uuid;

fn code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or_default()
}

fn id(body: &Value) -> String {
    body["data"]["id"].as_str().unwrap().to_string()
}

fn names(body: &Value) -> Vec<String> {
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["full_name"].as_str().unwrap().to_string())
        .collect()
}

// === Groups

#[tokio::test]
async fn customer_groups_can_be_created_renamed_and_deleted() {
    let w = bare_world().await;

    let (status, group) = w
        .post(
            "/api/v1/customer-groups",
            json!({ "name": "  Wholesale  " }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{group}");
    assert_eq!(group["data"]["name"], "Wholesale");
    let group_id = id(&group);
    w.post("/api/v1/customer-groups", json!({ "name": "Staff" }))
        .await;

    let (_, list) = w.get("/api/v1/customer-groups").await;
    let listed: Vec<&str> = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["name"].as_str().unwrap())
        .collect();
    assert_eq!(listed, ["Staff", "Wholesale"], "sorted by name");

    let uri = format!("/api/v1/customer-groups/{group_id}");
    let (status, renamed) = w.patch(&uri, json!({ "name": "Trade" })).await;
    assert_eq!(
        (status, renamed["data"]["name"].as_str()),
        (StatusCode::OK, Some("Trade"))
    );

    assert_eq!(w.delete(&uri).await.0, StatusCode::OK);
    assert_eq!(w.delete(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.patch(&uri, json!({ "name": "Ghost" })).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, list) = w.get("/api/v1/customer-groups").await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn customer_group_input_is_validated() {
    let w = bare_world().await;
    w.post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    let (_, other) = w
        .post("/api/v1/customer-groups", json!({ "name": "Staff" }))
        .await;

    for name in ["", "   ", &"x".repeat(101)] {
        let (status, body) = w
            .post("/api/v1/customer-groups", json!({ "name": name }))
            .await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::BAD_REQUEST, "VALIDATION_FAILED")
        );
    }
    let (status, body) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "ALREADY_EXISTS")
    );
    let (status, _) = w
        .patch(
            &format!("/api/v1/customer-groups/{}", id(&other)),
            json!({ "name": "Wholesale" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // The same name is fine in another tenant.
    let elsewhere = bare_world().await;
    assert_eq!(
        elsewhere
            .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_group_in_use_cannot_be_deleted() {
    let w = world().await;
    let (_, group) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    let uri = format!("/api/v1/customer-groups/{}", id(&group));

    // Referenced by a price rule: deleting would silently cascade the rule away.
    let (_, rule) = w
        .post(
            "/api/v1/price-rules",
            json!({ "product_id": w.rice, "unit_price_amount_minor": 100_000, "customer_group_id": id(&group) }),
        )
        .await;
    let (status, body) = w.delete(&uri).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "GROUP_IN_USE")
    );
    assert_eq!(w.count("price_rules").await, 2);
    w.delete(&format!("/api/v1/price-rules/{}", id(&rule)))
        .await;

    // Referenced by a customer.
    let (_, customer) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "Bulk Buyer", "customer_group_id": id(&group) }),
        )
        .await;
    let (status, body) = w.delete(&uri).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "GROUP_IN_USE")
    );

    // Once the customer leaves the group, it can go.
    w.patch(
        &format!("/api/v1/customers/{}", id(&customer)),
        json!({ "customer_group_id": null }),
    )
    .await;
    assert_eq!(w.delete(&uri).await.0, StatusCode::OK);
}

#[tokio::test]
async fn another_tenants_group_is_invisible() {
    let w = bare_world().await;
    let other = bare_world().await;
    let (_, theirs) = other
        .post("/api/v1/customer-groups", json!({ "name": "Theirs" }))
        .await;
    let uri = format!("/api/v1/customer-groups/{}", id(&theirs));

    assert_eq!(
        w.patch(&uri, json!({ "name": "Mine" })).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(w.delete(&uri).await.0, StatusCode::NOT_FOUND);
    let (status, _) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "A", "customer_group_id": id(&theirs) }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = w.get("/api/v1/customer-groups").await;
    assert!(list["data"].as_array().unwrap().is_empty());
    let (_, still) = other.get("/api/v1/customer-groups").await;
    assert_eq!(still["data"][0]["name"], "Theirs");
}

// === Customers

#[tokio::test]
async fn a_customer_can_be_created_fetched_edited_and_deleted() {
    let w = bare_world().await;
    let (_, group) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;

    let (status, created) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "  Ada Obi  ", "phone": "+234 801 234 5678", "email": "  Ada@Example.COM ",
                    "customer_group_id": id(&group) }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let data = &created["data"];
    assert_eq!(data["full_name"], "Ada Obi");
    assert_eq!(
        data["email"], "ada@example.com",
        "emails are stored lowercase"
    );
    assert_eq!(data["phone"], "+234 801 234 5678");
    assert_eq!(data["customer_group_name"], "Wholesale");
    assert_eq!(data["store_credit_amount_minor"], 0);
    let uri = format!("/api/v1/customers/{}", id(&created));

    assert_eq!(w.get(&uri).await.1["data"]["full_name"], "Ada Obi");

    // Absent leaves alone; null clears; blank clears; the group moves independently.
    let (_, body) = w.patch(&uri, json!({ "full_name": "Ada Okafor" })).await;
    assert_eq!(
        (
            body["data"]["email"].as_str(),
            body["data"]["phone"].as_str()
        ),
        (Some("ada@example.com"), Some("+234 801 234 5678"))
    );
    let (_, body) = w.patch(&uri, json!({ "email": null })).await;
    assert!(body["data"]["email"].is_null());
    let (_, body) = w.patch(&uri, json!({ "phone": "  " })).await;
    assert!(body["data"]["phone"].is_null());
    let (_, body) = w.patch(&uri, json!({ "customer_group_id": null })).await;
    assert!(body["data"]["customer_group_id"].is_null());
    assert!(body["data"]["customer_group_name"].is_null());
    assert_eq!(body["data"]["full_name"], "Ada Okafor");

    assert_eq!(w.delete(&uri).await.0, StatusCode::OK);
    assert_eq!(w.get(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.delete(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.patch(&uri, json!({ "full_name": "Back" })).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, list) = w.get("/api/v1/customers").await;
    assert!(list["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn customer_input_is_validated() {
    let w = bare_world().await;
    let (_, existing) = w
        .post("/api/v1/customers", json!({ "full_name": "Existing" }))
        .await;
    let uri = format!("/api/v1/customers/{}", id(&existing));

    let bad_creates = [
        json!({ "full_name": "" }),
        json!({ "full_name": "   " }),
        json!({ "full_name": "x".repeat(101) }),
        json!({ "full_name": "A", "phone": "abc" }),
        json!({ "full_name": "A", "phone": "123" }),
        json!({ "full_name": "A", "phone": "1".repeat(16) }),
        json!({ "full_name": "A", "phone": "0801-234-5678; DROP TABLE" }),
        json!({ "full_name": "A", "email": "no-at-sign" }),
        json!({ "full_name": "A", "email": "@nolocal.com" }),
        json!({ "full_name": "A", "email": "a@nodot" }),
        json!({ "full_name": "A", "email": "a b@example.com" }),
        json!({ "full_name": "A", "email": "a@@example.com" }),
        json!({ "full_name": "A", "customer_group_id": Uuid::now_v7() }),
    ];
    for body in bad_creates {
        let (status, response) = w.post("/api/v1/customers", body.clone()).await;
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
            "{body} -> {status} {response}"
        );
    }

    for (body, expected) in [
        (json!({}), StatusCode::BAD_REQUEST),
        (json!({ "full_name": " " }), StatusCode::BAD_REQUEST),
        (json!({ "phone": "12" }), StatusCode::BAD_REQUEST),
        (json!({ "email": "nope" }), StatusCode::BAD_REQUEST),
        (
            json!({ "customer_group_id": Uuid::now_v7() }),
            StatusCode::NOT_FOUND,
        ),
        // A valid field next to a bad one is not half-applied.
        (
            json!({ "full_name": "Half Applied", "email": "nope" }),
            StatusCode::BAD_REQUEST,
        ),
        // Store credit and ownership are not editable.
        (
            json!({ "store_credit_amount_minor": 999_999 }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "tenant_id": Uuid::now_v7() }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "deleted_at": null }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let (status, response) = w.patch(&uri, body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
    }

    let (_, after) = w.get(&uri).await;
    assert_eq!(after["data"]["full_name"], "Existing");
    assert_eq!(after["data"]["store_credit_amount_minor"], 0);
    let (_, list) = w.get("/api/v1/customers").await;
    assert_eq!(
        list["data"].as_array().unwrap().len(),
        1,
        "no invalid create was stored"
    );
}

#[tokio::test]
async fn customers_can_be_searched_filtered_and_paged() {
    let w = bare_world().await;
    let (_, group) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    for (name, phone, email, grouped) in [
        ("Ada Obi", Some("08011112222"), Some("ada@shop.com"), true),
        ("Bola Ade", Some("08033334444"), None, false),
        ("Chidi 100% Real", None, Some("chidi_real@mail.com"), true),
        ("Dayo Ade", None, None, false),
    ] {
        let mut body = json!({ "full_name": name });
        if let Some(phone) = phone {
            body["phone"] = json!(phone);
        }
        if let Some(email) = email {
            body["email"] = json!(email);
        }
        if grouped {
            body["customer_group_id"] = json!(id(&group));
        }
        assert_eq!(w.post("/api/v1/customers", body).await.0, StatusCode::OK);
    }
    let list = |query: &str| {
        let uri = format!("/api/v1/customers{query}");
        let w = &w;
        async move { names(&w.get(&uri).await.1) }
    };

    assert_eq!(
        list("").await,
        ["Ada Obi", "Bola Ade", "Chidi 100% Real", "Dayo Ade"]
    );
    assert_eq!(
        list("?q=ade").await,
        ["Bola Ade", "Dayo Ade"],
        "case-insensitive substring on name"
    );
    assert_eq!(list("?q=0803").await, ["Bola Ade"], "matches phone");
    assert_eq!(list("?q=SHOP.com").await, ["Ada Obi"], "matches email");
    // % and _ are literal characters, not wildcards.
    assert_eq!(list("?q=100%25").await, ["Chidi 100% Real"]);
    assert_eq!(list("?q=%25").await, ["Chidi 100% Real"]);
    assert_eq!(list("?q=chidi_real").await, ["Chidi 100% Real"]);
    assert_eq!(list("?q=chidi%5Freal").await, ["Chidi 100% Real"]);
    assert!(list("?q=zzz").await.is_empty());
    assert_eq!(
        list("?q=%20%20").await.len(),
        4,
        "a blank search is no filter"
    );
    assert_eq!(
        list(&format!("?customer_group_id={}", id(&group))).await,
        ["Ada Obi", "Chidi 100% Real"]
    );
    assert_eq!(
        list(&format!("?customer_group_id={}&q=ada", id(&group))).await,
        ["Ada Obi"]
    );

    assert_eq!(list("?limit=2").await, ["Ada Obi", "Bola Ade"]);
    assert_eq!(
        list("?limit=2&offset=2").await,
        ["Chidi 100% Real", "Dayo Ade"]
    );
    assert!(list("?limit=2&offset=4").await.is_empty());
    assert_eq!(
        list("?limit=0").await.len(),
        1,
        "limit is clamped to at least 1"
    );
    assert_eq!(list("?limit=9999").await.len(), 4);
    assert_eq!(
        list("?offset=-5").await.len(),
        4,
        "a negative offset is treated as 0"
    );
}

#[tokio::test]
async fn another_tenants_customers_are_invisible() {
    let w = bare_world().await;
    let other = bare_world().await;
    let (_, theirs) = other
        .post(
            "/api/v1/customers",
            json!({ "full_name": "Their Customer" }),
        )
        .await;
    let uri = format!("/api/v1/customers/{}", id(&theirs));

    assert_eq!(w.get(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.patch(&uri, json!({ "full_name": "Hijacked" })).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(w.delete(&uri).await.0, StatusCode::NOT_FOUND);
    let (_, list) = w.get("/api/v1/customers?q=their").await;
    assert!(list["data"].as_array().unwrap().is_empty());
    assert_eq!(
        other.get(&uri).await.1["data"]["full_name"],
        "Their Customer"
    );
}

// === Permissions

#[tokio::test]
async fn cashiers_serve_customers_but_cannot_change_pricing_tiers() {
    let w = world().await;
    let (_, group) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    let (_, vip) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "Existing", "customer_group_id": id(&group) }),
        )
        .await;
    // A cashier at one store, who holds sale.create only there.
    let cashier = store_cashier(&w, w.stores[0]).await;
    let call = |method: Method, uri: String, body: Option<Value>| {
        let (app, token) = (w.app.clone(), cashier.clone());
        async move { send(&app, method, &uri, &token, body).await }
    };

    // They can register and look up customers from their till...
    let (status, mine) = call(
        Method::POST,
        "/api/v1/customers".into(),
        Some(json!({ "full_name": "Walk In", "phone": "08012345678" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mine}");
    let my_uri = format!("/api/v1/customers/{}", id(&mine));
    assert_eq!(
        call(Method::GET, "/api/v1/customers?q=walk".into(), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call(Method::GET, my_uri.clone(), None).await.0,
        StatusCode::OK
    );
    let (status, _) = call(
        Method::PATCH,
        my_uri.clone(),
        Some(json!({ "full_name": "Walk In Renamed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // ...but cannot put anyone (themselves included) into a cheaper pricing group.
    let (status, _) = call(
        Method::POST,
        "/api/v1/customers".into(),
        Some(json!({ "full_name": "Friend", "customer_group_id": id(&group) })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        Method::PATCH,
        my_uri.clone(),
        Some(json!({ "customer_group_id": id(&group) })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        Method::PATCH,
        format!("/api/v1/customers/{}", id(&vip)),
        Some(json!({ "customer_group_id": null })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, still) = w.get(&my_uri).await;
    assert!(still["data"]["customer_group_id"].is_null());
    assert_eq!(
        w.get(&format!("/api/v1/customers/{}", id(&vip))).await.1["data"]["customer_group_name"],
        "Wholesale"
    );

    // Groups and deletion are for managers.
    assert_eq!(
        call(
            Method::POST,
            "/api/v1/customer-groups".into(),
            Some(json!({ "name": "Mine" }))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::PATCH,
            format!("/api/v1/customer-groups/{}", id(&group)),
            Some(json!({ "name": "x" }))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::DELETE,
            format!("/api/v1/customer-groups/{}", id(&group)),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(Method::DELETE, my_uri, None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(Method::GET, "/api/v1/customer-groups".into(), None)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn staff_who_cannot_sell_cannot_read_customer_details() {
    let w = world().await;
    let (_, customer) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "Private Person", "phone": "08012345678" }),
        )
        .await;
    let accountant = store_user(&w, w.stores[0], ACCOUNTANT_ROLE_ID).await;
    let manager = store_user(&w, w.stores[1], common::MANAGER_ROLE_ID).await;

    let uri = format!("/api/v1/customers/{}", id(&customer));
    for (method, path) in [
        (Method::GET, "/api/v1/customers".to_string()),
        (Method::GET, uri.clone()),
        (Method::DELETE, uri.clone()),
    ] {
        let (status, _) = send(&w.app, method, &path, &accountant, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
    // A manager at any one store may read, edit and delete tenant customers.
    assert_eq!(
        send(&w.app, Method::GET, &uri, &manager, None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&w.app, Method::DELETE, &uri, &manager, None).await.0,
        StatusCode::OK
    );
}

// === Pricing through the API only

#[tokio::test]
async fn a_customer_group_discount_works_end_to_end_through_the_api() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 100).await;
    let (_, group) = w
        .post("/api/v1/customer-groups", json!({ "name": "Wholesale" }))
        .await;
    let (status, _) = w
        .post(
            "/api/v1/price-rules",
            json!({ "product_id": w.rice, "unit_price_amount_minor": 120_000, "customer_group_id": id(&group) }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, member) = w
        .post(
            "/api/v1/customers",
            json!({ "full_name": "Trade Buyer", "customer_group_id": id(&group) }),
        )
        .await;
    let (_, walk_in) = w
        .post("/api/v1/customers", json!({ "full_name": "Walk In" }))
        .await;

    let price_for = |customer: Option<&Value>| {
        let mut sale = w.sale_body(0, &[(w.rice, 1)]);
        if let Some(customer) = customer {
            sale["customer_id"] = json!(id(customer));
        }
        let w = &w;
        async move {
            w.post("/api/v1/sales", sale).await.1["data"]["total_amount_minor"]
                .as_i64()
                .unwrap()
        }
    };
    assert_eq!(price_for(Some(&member)).await, 120_000);
    assert_eq!(price_for(Some(&walk_in)).await, 150_000);
    assert_eq!(price_for(None).await, 150_000);

    // Move the buyer out of the group and they pay the normal price again.
    w.patch(
        &format!("/api/v1/customers/{}", id(&member)),
        json!({ "customer_group_id": null }),
    )
    .await;
    assert_eq!(price_for(Some(&member)).await, 150_000);

    // A deleted customer cannot be put on a new sale; the four earlier sales stand.
    w.delete(&format!("/api/v1/customers/{}", id(&walk_in)))
        .await;
    let mut sale = w.sale_body(0, &[(w.rice, 1)]);
    sale["customer_id"] = json!(id(&walk_in));
    assert_eq!(w.post("/api/v1/sales", sale).await.0, StatusCode::NOT_FOUND);
    assert_eq!(w.on_hand(0, w.rice).await, d(96));
}
