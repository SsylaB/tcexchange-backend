use sqlx::sqlite::SqlitePoolOptions;
use serde::Deserialize;

#[derive(Deserialize)]
struct Destination {
    id: i64,
    #[serde(rename = "universityName")]
    university_name: String,
    country: String,
    location: Option<String>,
    url: Option<String>,
    #[serde(rename = "exchangeType")]
    exchange_type: Option<String>,
    languages: Option<Vec<String>>,
    description: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
    position: Vec<f64>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let pool = SqlitePoolOptions::new()
        .connect("sqlite:./tcexchange.db")
        .await
        .expect("Failed to connect");

    // Read the JSON file — adjust the path to where your file actually is
    let data = std::fs::read_to_string("././data/destinations.json")
        .expect("Could not read destinations.json");

    let destinations: Vec<Destination> = serde_json::from_str(&data)
        .expect("Could not parse JSON");

    for dest in destinations {
        let languages_str = dest.languages.map(|l| l.join(","));
        let location = dest.location;
        let url = dest.url;
        let exchange_type = dest.exchange_type;
        let description = dest.description;
        let short_name = dest.short_name;
        let position_str = format!("{:?}", dest.position);

        sqlx::query!(
            "INSERT OR IGNORE INTO destinations
            (id, university_name, country, location, url, exchange_type, languages, description, short_name, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            dest.id,
            dest.university_name,
            dest.country,
            location,
            url,
            exchange_type,
            languages_str,
            description,
            short_name,
            position_str,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert destination");
    }

    println!("Seeding done!");
}
