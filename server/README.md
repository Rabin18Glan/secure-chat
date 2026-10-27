# Secure Chat Server (Axum + MongoDB)

A high-performance chat backend built with Rust's Axum framework and MongoDB, structured following Service-Oriented Architecture (SOA).

## Architecture

- **`config`**: Environment variable management (`parameter.rs`) and MongoDB database pool initialization (`database.rs`).
- **`entity`**: MongoDB document schemas (`User`, `Message`) mapped via `bson` / `serde`.
- **`dto`**: Data Transfer Objects for requests and responses.
- **`error`**: Strongly-typed errors with `thiserror`, mapped to HTTP status codes.
- **`response`**: Unified API response wrappers.
- **`repository`**: Direct data access layer for MongoDB operations.
- **`service`**: Business logic, password hashing (bcrypt), and data transformations.
- **`handler`**: Axum HTTP and WebSocket request handlers.
- **`routes`**: Modular sub-routers nested under `/api` and `/ws`.
- **`state`**: Application and sub-router states (DB connection, active WebSocket connections).

## API Endpoints

### Health / Ping
- `GET /ping` - Server health ping

### Authentication (`/api/auth`)
- `POST /api/auth/register` - Register a new user (`{ username, email, password }`)
- `POST /api/auth/login` - Authenticate user (`{ username, password }`)
- `GET /api/auth/allusers/{id}` - Get all registered users except current user
- `POST /api/auth/setavatar/{id}` - Set user avatar image (`{ image }`)
- `GET /api/auth/logout/{id}` - Log out user and clean up online status

### Messages (`/api/messages`)
- `POST /api/messages/addmsg` - Store a new message (`{ from, to, message }`)
- `POST /api/messages/getmsg` - Retrieve chat history between two users (`{ from, to }`)

### WebSocket (`/ws`)
- `GET /ws` or `GET /ws?userId={id}` - Real-time WebSocket connection for chat
  - Send `{ "event": "add-user", "userId": "<id>" }` to register user session
  - Send `{ "event": "send-msg", "to": "<target_id>", "msg": "<text>" }` to dispatch real-time message
  - Client receives `{ "event": "msg-recieve", "msg": "<text>", "from": "<sender_id>" }`

## Setup & Running

1. Ensure MongoDB is running (e.g. `mongodb://localhost:27017`).
2. Copy `.env.example` to `.env` and configure `PORT`, `MONGO_URL`, and `DB_NAME`.
3. Run with Cargo:
   ```bash
   cargo run
   ```
