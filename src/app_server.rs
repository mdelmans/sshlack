use std::collections::HashMap;
use std::sync::Arc;

use log::{error, info};

use std::{fs::OpenOptions, net::IpAddr, path::Path};

use tokio::sync::Mutex;

use crate::{
    app::{App, AppController},
    db_models::{Message, User},
};

use russh::{
    keys::PrivateKey,
    server::{Config, Server},
};

use sqlx::Row;
use sqlx::sqlite::SqlitePool;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

const DB_FILE: &str = "sshlack.db";

pub struct AppServerController {
    clients: Mutex<HashMap<usize, Arc<Mutex<AppController>>>>,
    pub next_client_id: Mutex<usize>,

    pub users: Mutex<Vec<User>>,

    db_pool: SqlitePool,
}

impl AppServerController {
    pub async fn new() -> Result<Self, anyhow::Error> {
        Self::ensure_db_exists()?;

        match SqlitePool::connect(format!("sqlite://{}", DB_FILE).as_str()).await {
            Ok(db_pool) => {
                let controller = Self {
                    clients: Mutex::new(HashMap::new()),
                    next_client_id: Mutex::new(0),
                    users: Mutex::new(Vec::new()),
                    db_pool: db_pool,
                };
                controller.initialise().await?;
                Ok(controller)
            }
            Err(e) => {
                error!("Failed to connect to the database: {}", e);
                Err(e.into())
            }
        }
    }

    fn ensure_db_exists() -> Result<(), anyhow::Error> {
        let path = Path::new(DB_FILE);

        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        Ok(())
    }

    pub async fn add_client(&self, app_controller: Arc<Mutex<AppController>>) {
        // let app_controller = Arc::clone(&app_controller);
        let mut users = self.users.lock().await;
        users.push(app_controller.lock().await.app_state.user.clone());

        let mut next_client_id = self.next_client_id.lock().await;
        let mut clients = self.clients.lock().await;
        clients.insert(*next_client_id, Arc::clone(&app_controller));

        *next_client_id += 1;
    }

    pub async fn initialise(&self) -> Result<(), anyhow::Error> {
        sqlx::query("CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT, sender TEXT)").execute(&self.db_pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, username TEXT UNIQUE, password_hash TEXT)").execute(&self.db_pool).await?;
        Ok(())
    }

    pub async fn send_message(&self, message: Message) -> Result<(), anyhow::Error> {
        let mut conn = self.db_pool.acquire().await?;
        sqlx::query("INSERT INTO messages (content, sender) VALUES (?, ?)")
            .bind(message.content)
            .bind(message.sender.username)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let mut conn = self.db_pool.acquire().await?;
        let rows = sqlx::query("SELECT * FROM ( SELECT id, content, sender FROM messages ORDER BY id DESC LIMIT 1000) as recent ORDER BY id ASC")
            .fetch_all(&mut *conn)
            .await?;
        let messages = rows
            .into_iter()
            .map(|row| {
                Message::new(
                    row.get::<String, _>("content"),
                    User::authenticated(row.get::<String, _>("sender").as_str()),
                )
            })
            .collect();
        Ok(messages)
    }

    pub async fn auth_user(&self, username: &str, password: &str) -> Result<User, anyhow::Error> {
        let mut conn = self.db_pool.acquire().await?;
        let user_row = sqlx::query("SELECT password_hash FROM users WHERE username =?")
            .bind(username)
            .fetch_one(&mut *conn)
            .await;

        let argon2 = Argon2::default();

        if user_row.is_ok() {
            let password_hash = user_row.unwrap().get::<String, _>("password_hash");
            let password_hash = PasswordHash::new(password_hash.as_str())
                .map_err(|_e| anyhow::anyhow!("Failed to parse password hash"))?;
            argon2
                .verify_password(password.as_bytes(), &password_hash)
                .map_err(|_e| anyhow::anyhow!("Failed to verify password"))?;
        } else {
            if !password.is_empty() {
                let salt = SaltString::generate(&mut OsRng);
                match argon2.hash_password(password.as_bytes(), &salt) {
                    Ok(hashed_password) => {
                        sqlx::query("INSERT INTO users (username, password_hash) VALUES (?,?)")
                            .bind(username)
                            .bind(hashed_password.to_string())
                            .execute(&mut *conn)
                            .await?;
                    }
                    Err(_) => {
                        return Err(anyhow::anyhow!("Failed to hash password"));
                    }
                }
            }
        }
        Ok(User::authenticated(username))
    }

    pub async fn get_users(&self) -> Vec<User> {
        self.users.lock().await.clone()
    }
}

pub struct AppServer {
    controller: Arc<AppServerController>,
}

impl AppServer {
    pub async fn run(address: IpAddr, port: u16, pem: PrivateKey) -> Result<(), anyhow::Error> {
        let server_controller = AppServerController::new().await?;

        let mut server = Self {
            controller: Arc::new(server_controller),
        };

        let controller = Arc::clone(&server.controller);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let mut disconnected_clients = Vec::<usize>::new();

                for (client_id, app_controller) in controller.clients.lock().await.iter_mut() {
                    let mut app_controller = app_controller.lock().await;
                    app_controller.draw().await.unwrap();

                    if !app_controller.active {
                        info!("{} disconnected", app_controller.app_state.user.username);
                        disconnected_clients.push(*client_id);
                    }
                }

                let mut clients = controller.clients.lock().await;

                for client_id in disconnected_clients {
                    clients.remove(&client_id);
                }

                let mut users = controller.users.lock().await;
                users.clear();
                for (_, app_controller) in clients.iter_mut() {
                    let app_controller = app_controller.lock().await;
                    users.push(app_controller.app_state.user.clone());
                }
            }
        });

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![pem],
            nodelay: true,
            ..Default::default()
        };

        server
            .run_on_address(Arc::new(config), (address, port))
            .await?;
        Ok(())
    }
}

impl Server for AppServer {
    type Handler = App;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> App {
        info!("New client attempting connection");
        App::new(Arc::clone(&self.controller))
    }
}
