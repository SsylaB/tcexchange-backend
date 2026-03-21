# tcexchange-backend

A high-performance REST API backend for the tcexchange university exchange catalog, built with **Rust**, **Axum**, and **SQLite**.

## Tech Stack

- **Framework**: [Axum](https://github.com/tokio-rs/axum) — async Rust web framework
- **Runtime**: [Tokio](https://tokio.rs) — async runtime
- **Database**: SQLite with [SQLx](https://github.com/launchbywarp/sqlx) for compile-time SQL verification
- **Serialization**: [Serde](https://serde.rs) — JSON (de)serialization
- **CORS**: [Tower-HTTP](https://github.com/tower-rs/tower-http) — handles cross-origin requests from the React frontend

## Project Structure

```
src/
  main.rs           # Entry point, server initialization, migrations
  db/mod.rs         # Database connection pool setup
  models/mod.rs     # Destination struct (shared type between DB and API)
  routes/mod.rs     # HTTP handlers for all endpoints
  bin/seed.rs       # One-time utility to populate DB from destinations.json

migrations/
  0001_create_destinations.sql  # Database schema (run on startup)

data/
  destinations.json # Seed data for the database
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs) 1.70+
- SQLite3

### Installation

1. Clone the repository:
```bash
git clone https://github.com/SsylaB/tcexchange-backend.git
cd tcexchange-backend
```

2. Set up the database (creates `tcexchange.db` and runs migrations):
```bash
cargo sqlx database create
cargo sqlx migrate run
```

3. Seed the database with initial data:
```bash
cargo run --bin seed
```

4. Run the server:
```bash
cargo run
```

The server starts on `http://localhost:3000`.

## API Endpoints

### Get all destinations
```bash
GET /api/destinations
```

**Response:**
```json
[
  {
    "id": 1,
    "university_name": "MIT",
    "country": "United States",
    "location": "Cambridge, MA",
    "url": "https://mit.edu",
    "exchange_type": "Erasmus+",
    "languages": "English",
    "description": "Leading research university",
    "short_name": "MIT"
  }
]
```

### Get a single destination
```bash
GET /api/destinations/:id
```

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release
```

The optimized binary will be in `target/release/tcexchange-backend`.

### Regenerating SQLx Query Cache
If you modify SQL queries, update the `.sqlx/` cache:
```bash
export DATABASE_URL=sqlite:./tcexchange.db
cargo sqlx prepare
```

## Environment

Create a `.env` file (not committed to git):
```
DATABASE_URL=sqlite:./tcexchange.db
```

The database URL is optional — it defaults to `sqlite:./tcexchange.db` if not set.

## Database Schema

The `destinations` table stores university exchange information:

| Column | Type | Nullable |
|--------|------|----------|
| id | INTEGER | ✗ |
| university_name | TEXT | ✗ |
| country | TEXT | ✗ |
| location | TEXT | ✓ |
| url | TEXT | ✓ |
| exchange_type | TEXT | ✓ |
| languages | TEXT | ✓ |
| description | TEXT | ✓ |
| short_name | TEXT | ✓ |

*Note: `languages` is stored as a comma-separated string (e.g., `"French,English"`) since SQLite has no native array type.*

## Connecting the Frontend

Your React frontend should fetch data from this API. Example:

```typescript
const [destinations, setDestinations] = useState([]);

useEffect(() => {
  fetch("http://localhost:3000/api/destinations")
    .then(res => res.json())
    .then(data => setDestinations(data))
    .catch(err => console.error("Failed to fetch destinations:", err));
}, []);
```

## CORS Configuration

The server allows requests from any origin by default:

```rust
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```

For production, we might restrict this to the frontend domain:

```rust
.allow_origin("https://yourdomain.com".parse::<HeaderValue>().unwrap())
```

## Deployment

### Local Development
```bash
cargo run
```

### Production with SQLite
For small to medium deployments, SQLite is perfectly adequate. Simply deploy the binary and the `.db` file together.

### Migration to PostgreSQL
When scaling up, migrating to PostgreSQL is straightforward — SQLx supports both seamlessly. Only change the connection string and the `sqlx` feature flag in `Cargo.toml`.

## Learning Resources

- [Axum Documentation](https://docs.rs/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [SQLx Documentation](https://github.com/launchbywarp/sqlx)
- [Rust Book](https://doc.rust-lang.org/book/)

## License

MIT

## Author

Created as a learning project to improve Rust skills while building the tcexchange platform.