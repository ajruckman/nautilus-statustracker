use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Node, Selector};
use std::env;
use std::error::Error;

#[derive(Insertable)]
#[diesel(table_name = nautilus_update)]
struct NautilusUpdate {
    current_status: Option<String>,
    ship_location: Option<String>,
    update_message: Option<String>,
    update_time: Option<String>,
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
    let update_message_selector = Selector::parse(".dash-message p").unwrap();
    let update_time_selector = Selector::parse(".dash-message em").unwrap();

    let current_status_element = parsed.select(&current_status_selector).next();
    let ship_location_element = parsed.select(&ship_location_selector).next();
    let update_message_element = parsed.select(&update_message_selector).next();
    let update_time_element = parsed.select(&update_time_selector).next();

    if let Some(current_status) = current_status_element {
        println!(
            "Current status: {}",
            current_status.text().collect::<String>()
        );
    } else {
        println!("Current status not found");
    }
    if let Some(ship_location) = ship_location_element {
        println!(
            "Ship location: {}",
            ship_location.text().collect::<String>()
        );
    } else {
        println!("Ship location not found");
    }
    if let Some(update_message) = update_message_element {
        println!(
            "Update message: {}",
            get_inner_text_and_links(update_message)
        );
    } else {
        println!("Update message not found");
    }
    if let Some(update_time) = update_time_element {
        println!("Update time: {}", update_time.text().collect::<String>());
    } else {
        println!("Update time not found");
    }

    //

    diesel::insert_into(nautilus_update::table)
        .values(&NautilusUpdate {
            current_status: current_status_element.map(|e| e.text().collect()),
            ship_location: ship_location_element.map(|e| e.text().collect()),
            update_message: update_message_element.map(|e| get_inner_text_and_links(e)),
            update_time: update_time_element.map(|e| e.text().collect()),
        })
        .execute(&mut db_conn)
        .expect("Error saving new nautilus update");

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
                    let link_text = match ElementRef::wrap(child.clone()) {
                        Some(el_ref) => el_ref.text().collect::<String>(),
                        None => String::new(),
                    };
                    format!("[{}]({})", link_text, href)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        })
        .collect()
}
