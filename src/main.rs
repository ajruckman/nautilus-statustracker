use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Node, Selector};
use serde_derive::Serialize;
use serde_json::json;
use std::env;
use std::error::Error;
use std::fmt::Debug;

#[derive(Queryable, PartialEq)]
struct NautilusUpdate {
    id: i32,
    current_status: Option<String>,
    ship_location: Option<String>,
    update_message: Option<String>,
    update_time: Option<String>,
    fetched_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = nautilus_update)]
struct NautilusUpdateInsert {
    current_status: Option<String>,
    ship_location: Option<String>,
    update_message: Option<String>,
    update_time: Option<String>,
}

impl Debug for NautilusUpdateInsert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NautilusUpdate")
            .field("current_status", &self.current_status)
            .field("ship_location", &self.ship_location)
            .field("update_message", &self.update_message)
            .field("update_time", &self.update_time)
            .finish()
    }
}

fn compare_update(a: &NautilusUpdate, b: &NautilusUpdateInsert) -> bool {
    a.current_status == b.current_status
        && a.ship_location == b.ship_location
        && a.update_message == b.update_message
}

table! {
    nautilus_update (id) {
        id -> Int4,
        current_status -> Nullable<Varchar>,
        ship_location -> Nullable<Varchar>,
        update_message -> Nullable<Text>,
        update_time -> Nullable<Varchar>,
        fetched_at -> Timestamptz,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let database_url = env::var("CONN_STRING").expect("CONN_STRING must be set");

    //

    let db_manager = ConnectionManager::<PgConnection>::new(database_url);
    let db_pool: Pool<ConnectionManager<PgConnection>> = Pool::builder()
        .build(db_manager)
        .expect("Failed to create pool.");
    let mut db_conn = db_pool.get()?;

    //

    let client = Client::new();
    let resp = client.get("https://nautiluslive.org/").send()?;
    let body = resp.text()?;

    let parsed = Html::parse_document(&body);

    let current_status_selector = Selector::parse(".dash-data").unwrap();
    let ship_location_selector = Selector::parse(".status-full .dash-data").unwrap();
    let update_message_selector_1 = Selector::parse(".dash-message p").unwrap(); // first format
    let update_message_selector_2 = Selector::parse(".dash-message").unwrap(); // second format
    let update_time_selector = Selector::parse(".dash-message em").unwrap();

    let current_status_element = parsed.select(&current_status_selector).next();
    let ship_location_element = parsed.select(&ship_location_selector).next();

    let update_message_element = parsed.select(&update_message_selector_1).next()
        .or_else(|| parsed.select(&update_message_selector_2).next());

    let update_time_element = parsed.select(&update_time_selector).next();

    let new_nautilus_update = NautilusUpdateInsert {
        current_status: current_status_element.map(|e| e.text().collect()),
        ship_location: ship_location_element.map(|e| e.text().collect()),
        update_message: update_message_element.map(|e| get_inner_text_and_links(e)),
        update_time: update_time_element.map(|e| e.text().collect()),
    };

    //

    let last_row: Option<NautilusUpdate> = nautilus_update::table
        .order(nautilus_update::fetched_at.desc())
        .first(&mut db_conn)
        .optional()?;

    let mut is_new = false;

    if let Some(last_row) = last_row {
        if !compare_update(&last_row, &new_nautilus_update) {
            is_new = true;
        } else {
            println!("No new update");
        }
    } else {
        // last_row was None, so insert
        is_new = true;
    }

    if is_new {
        println!("{:?}", new_nautilus_update);

        diesel::insert_into(nautilus_update::table)
            .values(&new_nautilus_update)
            .execute(&mut db_conn)
            .expect("Error saving new Nautilus update");

        send_to_discord(&new_nautilus_update)?;
    }
    Ok(())
}

fn get_inner_text_and_links(element: ElementRef) -> String {
    element
        .children()
        .map(|child| match child.value() {
            Node::Text(text) => text.to_string(),
            Node::Element(element) => {
                if element.name() == "a" {
                    let href = element.attr("href").unwrap_or("");
                    let formatted_href = if href.starts_with("/") {
                        format!("https://nautiluslive.org{}", href)
                    } else {
                        href.to_string()
                    };
                    let link_text = match ElementRef::wrap(child.clone()) {
                        Some(el_ref) => el_ref.text().collect::<String>(),
                        None => String::new(),
                    };
                    format!("[{}]({})", link_text, formatted_href)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        })
        .collect()
}

//

#[derive(Serialize)]
struct DiscordWebhook {
    content: String,
}

fn send_to_discord(update: &NautilusUpdateInsert) -> Result<(), Box<dyn Error>> {
    let webhook_url = env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL must be set");
    let client = Client::new();

    let formatted_message = format!(
        "**Current Status:** {}\n**Ship Location:** {}\n**Update Message:** {}\n**Update Time:** {}",
        update.current_status.as_ref().unwrap_or(&"Unknown".to_string()),
        update.ship_location.as_ref().unwrap_or(&"Unknown".to_string()),
        update.update_message.as_ref().unwrap_or(&"Unknown".to_string()),
        update.update_time.as_ref().unwrap_or(&"Unknown".to_string()),
    );

    let webhook = json!({ "content": formatted_message });

    let res = client
        .post(&webhook_url)
        .header("Content-Type", "application/json")
        .body(webhook.to_string())
        .send()?;

    if res.status().is_success() {
        println!("Message sent to Discord successfully");
    } else {
        println!("Failed to send message to Discord");
    }

    Ok(())
}
