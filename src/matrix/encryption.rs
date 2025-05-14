use matrix_sdk::{Client, encryption::CrossSigningResetAuthType, ruma::api::client::uiaa};
use tracing::error;
use tracing::{info, instrument};

use crate::settings::ApplicationConfig;

#[instrument(skip_all)]
pub async fn first_time_signature_identity_bootstrap(
    config: &ApplicationConfig,
    client: &Client,
) -> eyre::Result<()> {
    info!("Bootstrapping a new cross signing identity");
    if let Some(handle) = client.encryption().reset_cross_signing().await? {
        match handle.auth_type() {
            CrossSigningResetAuthType::Uiaa(uiaa) => {
                let mut password = uiaa::Password::new(
                    client
                        .user_id()
                        .expect("a logged in client should have a user id")
                        .to_owned()
                        .into(),
                    config.client.password.clone(),
                );
                password.session = uiaa.session.clone();
                handle
                    .auth(Some(uiaa::AuthData::Password(password)))
                    .await?;
            }
            CrossSigningResetAuthType::OAuth(oauth) => {
                error!(
                    "Bots can't currently reset their identities if they're logged in with a pure oidc setup."
                );
                error!(
                    "To reset the bot's end-to-end encryption cross-signing identity anyway, you first need to login with the bot's account through your matrix oidc provider, then approve it at {}",
                    oauth.approval_url
                );
                handle.auth(None).await?;
            }
        }
    }

    Ok(())
}
