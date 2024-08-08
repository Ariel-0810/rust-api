mod model;
#[cfg(test)]
mod test;

use actix_web::{get, post, delete, web, App, HttpResponse, HttpServer};
use model::User;
use mongodb::{bson::{self, doc}, options::IndexOptions, Client, Collection, IndexModel};
use futures_util::stream::TryStreamExt;

const DB_NAME: &str = "myApp";
const COLL_NAME: &str = "users";

/// Adds a new user to the "users" collection in the database.
#[post("/add_user")]
async fn add_user(client: web::Data<Client>, json: web::Json<User>) -> HttpResponse {
    let collection = client.database(DB_NAME).collection(COLL_NAME);
    let result = collection.insert_one(json.into_inner()).await;
    match result {
        Ok(_) => HttpResponse::Ok().body("user added"),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Gets the user with the supplied username.
#[get("/get_user/{username}")]
async fn get_user(client: web::Data<Client>, username: web::Path<String>) -> HttpResponse {
    let username = username.into_inner();
    let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);
    match collection.find_one(doc! { "username": &username }).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => {
            HttpResponse::NotFound().body(format!("No user found with username {username}"))
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Gets all users in the collection.
#[get("/get_users")]
async fn get_users(client: web::Data<Client>) -> HttpResponse {
    let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);
    let cursor = collection.find(doc! {}).await;

    match cursor {
        Ok(mut users) => {
            let mut all_users = vec![];
            while let Some(user) = users.try_next().await.unwrap() {
                all_users.push(user);
            }
            HttpResponse::Ok().json(all_users)
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Updates the user with the supplied username.
#[post("/update_user/{username}")]
async fn update_user(client: web::Data<Client>, username: web::Path<String>, form: web::Json<serde_json::Value>) -> HttpResponse {
    let username = username.into_inner();
    let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);

    let mut update_doc = doc! {};

    if let Some(first_name) = form.get("first_name") {
        if let Ok(bson_first_name) = bson::to_bson(first_name) {
            update_doc.insert("first_name", bson_first_name);
        }
    }
    if let Some(last_name) = form.get("last_name") {
        if let Ok(bson_last_name) = bson::to_bson(last_name) {
            update_doc.insert("last_name", bson_last_name);
        }
    }
    if let Some(email) = form.get("email") {
        if let Ok(bson_email) = bson::to_bson(email) {
            update_doc.insert("email", bson_email);
        }
    }

    let update_doc = doc! { "$set": update_doc };

    match collection.update_one(doc! { "username": &username }, update_doc).await {
        Ok(update_result) => {
            if update_result.matched_count > 0 {
                HttpResponse::Ok().body("User updated")
            } else {
                HttpResponse::NotFound().body(format!("No user found with username {username}"))
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}



/// Deletes the user with the supplied username.
#[delete("/delete_user/{username}")]
async fn delete_user(client: web::Data<Client>, username: web::Path<String>) -> HttpResponse {
    let username = username.into_inner();
    let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);

    match collection.delete_one(doc! { "username": &username }).await {
        Ok(delete_result) => {
            if delete_result.deleted_count > 0 {
                HttpResponse::Ok().body("User deleted")
            } else {
                HttpResponse::NotFound().body(format!("No user found with username {username}"))
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// Creates an index on the "username" field to force the values to be unique.
async fn create_username_index(client: &Client) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "username": 1 })
        .options(options)
        .build();
    client
        .database(DB_NAME)
        .collection::<User>(COLL_NAME)
        .create_index(model)
        .await
        .expect("creating an index should succeed");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());

    let client = Client::with_uri_str(&uri).await.expect("failed to connect");
    create_username_index(&client).await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(add_user)
            .service(get_user)
            .service(get_users)
            .service(update_user)
            .service(delete_user)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}



