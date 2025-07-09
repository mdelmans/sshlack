use std::sync::Arc;

use log::{error, info};

use tokio::sync::Mutex;

use russh::keys::ssh_key::PublicKey;
use russh::server::{Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, MethodKind, MethodSet, Pty};

use ratatui::layout::Rect;

use terminal_keycode::Decoder;

use crate::{app::AppController, app_server::AppServerController, db_models::User};

pub struct App {
    server_controller: Arc<AppServerController>,
    pub app_controller: Option<Arc<Mutex<AppController>>>,
    pub decoder: Decoder,
    user: User,
}

impl App {
    pub fn new(server_controller: Arc<AppServerController>) -> Self {
        Self {
            server_controller,
            app_controller: None,
            decoder: Decoder::new(),
            user: User::unauthenticated(),
        }
    }

    pub async fn create_controller(
        &mut self,
        session: &mut Session,
        channel_id: ChannelId,
        user: User,
    ) -> Result<Arc<Mutex<AppController>>, anyhow::Error> {
        let controller = Arc::new(Mutex::new(
            AppController::new(
                Arc::clone(&self.server_controller),
                session,
                channel_id,
                user,
            )
            .await?,
        ));
        self.app_controller = Some(Arc::clone(&controller));
        Ok(controller)
    }
}

impl Handler for App {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Opening new client session for {}", self.user.username);
        let app_controller = self
            .create_controller(session, channel.id(), self.user.clone())
            .await?;

        self.server_controller.add_client(app_controller).await;
        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::from(&[MethodKind::Password][..])),
            partial_success: false,
        })
    }

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        let username = user;
        info!("Authenticating {} using password", username);
        let user = self.server_controller.auth_user(username, password).await;
        match user {
            Ok(user) => {
                info!("{} authenticated", user.username);
                self.user = user;
            }
            Err(e) => {
                error!("Error authenticating {}: {}", username, e);
                return Ok(Auth::Reject {
                    proceed_with_methods: None,
                    partial_success: false,
                });
            }
        }
        Ok(Auth::Accept)
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let rect = Rect {
            x: 0,
            y: 0,
            width: col_width as u16,
            height: row_height as u16,
        };
        if let Some(controller) = &self.app_controller {
            let mut controller = controller.lock().await;
            controller.resize_terminal(rect);
        }
        session.channel_success(channel)?;

        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let rect = Rect {
            x: 0,
            y: 0,
            width: col_width as u16,
            height: row_height as u16,
        };

        if let Some(controller) = &self.app_controller {
            let mut controller = controller.lock().await;
            controller.resize_terminal(rect);
        }

        Ok(())
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.process_input_data(data).await
    }
}
